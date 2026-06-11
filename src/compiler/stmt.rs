use lasso::Spur;

use crate::{
    compiler::{
        compile::{CompileError, Compiler, Result},
        lexer::Token,
    },
    emit, emit_args,
    functions::FnLookupKey,
};

type Identifier = Spur;

enum JumpKind {
    Break,
    Continue,
}

impl<'a> Compiler<'a> {
    pub(super) fn statement(&mut self) -> Result<()> {
        self.clear_temp();
        self.record_locus();
        match self.current() {
            Token::Let => self.parse_let_decl(),
            Token::Fn => self.parse_fn_decl(),
            Token::Return => self.parse_return_stmt(),
            Token::Break => self.parse_jump(JumpKind::Break),
            Token::Continue => self.parse_jump(JumpKind::Continue),
            Token::LBrace => self.parse_block(),
            Token::If => self.parse_if(),
            Token::While => self.parse_while(),
            Token::Identifier(id) => {
                if self.peek()? == &Token::Assign {
                    self.advance()?;
                    self.parse_assign(id)
                } else {
                    self.parse_expr_stmt()
                }
            }
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_block(&mut self) -> Result<()> {
        let open_span = self.current_span().clone();
        self.consume(&Token::LBrace)?;
        self.enter_scope();
        while !self.check(&Token::RBrace) && !self.check(&Token::Eof) {
            self.statement()?;
        }
        self.exit_scope();
        self.consume_close(&Token::RBrace, open_span)?;
        Ok(())
    }

    fn parse_let_decl(&mut self) -> Result<()> {
        self.consume(&Token::Let)?;

        let name = if let Token::Identifier(spur) = self.current() {
            let name = spur;
            self.advance()?;
            name
        } else {
            return Err(CompileError::Unexpected {
                token: self.current(),
                diag: (
                    self.current_span().clone(),
                    format!("expected identifier, found {}", self.current()),
                ),
            });
        };

        self.consume(&Token::Assign)?;

        if self.is_top_level() {
            let slot = self.declare_global(name);
            let rhs = self.expression(None)?;
            self.frame_mut().regs.free_temp(rhs);
            emit!(self.chunk_mut(), SETGLB, wide slot, rhs);
        } else {
            let held = self.alloc_held()?;
            let rhs = self.expression(Some(held))?;
            self.emit_move(held, rhs);
            self.free_other_temps(held, &[rhs]);
            self.add_local(name, held);
        }

        self.consume(&Token::Semicolon)?;
        Ok(())
    }

    fn parse_fn_decl(&mut self) -> Result<()> {
        self.consume(&Token::Fn)?;
        let name = if let Token::Identifier(spur) = self.current() {
            let name = spur;
            self.advance()?;
            name
        } else {
            return Err(CompileError::Unexpected {
                token: self.current(),
                diag: (
                    self.current_span().clone(),
                    format!("expected identifier, found {}", self.current()),
                ),
            });
        };
        let args = self.parse_arglist()?;
        let chunk_idx = self.new_frame(&args);
        self.parse_block()?;
        self.exit_frame()?;

        self.funcs.register(FnLookupKey::Name(name), chunk_idx);

        Ok(())
    }

    fn parse_assign(&mut self, id: Identifier) -> Result<()> {
        let span = self.prev_span().clone();
        self.advance()?;
        if let Some(local) = self.try_resolve_local(id) {
            let reg = self.expression(Some(local))?;
            self.emit_move(local, reg);
            self.frame_mut().regs.free_temp(reg);
        } else if let Some(global) = self.resolve_global(id) {
            let reg = self.expression(None)?;
            emit!(self.chunk_mut(), SETGLB, wide global, reg);
            self.frame_mut().regs.free_temp(reg);
        } else {
            return Err(CompileError::UndefinedVariable {
                name: self.intern_resolve(&id).to_string(),
                diag: (span, "undefined variable".to_string()),
            });
        }
        self.consume(&Token::Semicolon)?;
        Ok(())
    }

    fn parse_return_stmt(&mut self) -> Result<()> {
        self.consume(&Token::Return)?;
        if self.check(&Token::Semicolon) {
            self.consume(&Token::Semicolon)?;
            let reg = self.alloc_temp()?;
            emit!(self.chunk_mut(), LOADNIL, reg);
            emit!(self.chunk_mut(), RETURN, reg);
        } else {
            let reg = self.expression(None)?;
            self.consume(&Token::Semicolon)?;
            emit!(self.chunk_mut(), RETURN, reg);
        }
        Ok(())
    }

    fn parse_expr_stmt(&mut self) -> Result<()> {
        let reg = self.expression(None)?;
        self.frame_mut().regs.free_temp(reg);
        self.consume(&Token::Semicolon)?;
        Ok(())
    }

    fn parse_if(&mut self) -> Result<()> {
        self.consume(&Token::If)?;
        let test = self.expression(None)?;
        let test_ip = self.chunk().end();
        self.emit_test(test);
        self.frame_mut().regs.free_temp(test);
        self.parse_block()?;
        let if_end = self.chunk().end();

        if self.matches(&Token::Else)? {
            let if_end = self.emit_jmp();
            let else_start = self.chunk().end();
            if self.check(&Token::If) {
                self.parse_if()?;
            } else if self.check(&Token::LBrace) {
                self.parse_block()?;
            } else {
                return Err(CompileError::Unexpected {
                    token: self.current(),
                    diag: (
                        self.current_span().clone(),
                        format!(
                            "expected else-if clause or else clause, found {}",
                            self.current()
                        ),
                    ),
                });
            }
            let else_end = self.chunk().end();
            self.patch_if_else(test_ip, if_end, else_start, else_end);
        } else {
            self.patch_if(test_ip, if_end);
        }

        Ok(())
    }

    fn parse_jump(&mut self, kind: JumpKind) -> Result<()> {
        let token = match kind {
            JumpKind::Break => Token::Break,
            JumpKind::Continue => Token::Continue,
        };
        self.consume(&token)?;
        if !self.frame_mut().loops.in_loop() {
            return Err(CompileError::Unexpected {
                token,
                diag: (self.prev_span().clone(), format!("{token} outside loop")),
            });
        }
        let jmp_ip = self.emit_jmp();
        match kind {
            JumpKind::Break => self.frame_mut().loops.add_break(jmp_ip),
            JumpKind::Continue => self.frame_mut().loops.add_continue(jmp_ip),
        }
        self.consume(&Token::Semicolon)?;
        Ok(())
    }

    fn parse_while(&mut self) -> Result<()> {
        self.consume(&Token::While)?;
        let test_start = self.chunk().end();
        self.frame_mut().loops.push_loop(test_start);
        let test = self.expression(None)?;
        let test_ip = self.emit_test(test);
        self.frame_mut().regs.free_temp(test);
        self.parse_block()?;
        let body_end = self.chunk().end();
        let offset = test_start as isize - body_end as isize;
        self.emit_jump_offset(offset);
        let while_end = self.chunk().end();
        let patch = self.frame_mut().loops.pop_loop();

        for &jmp_ip in &patch.breaks {
            self.chunk_mut()
                .patch_jmp(jmp_ip, while_end as isize - jmp_ip as isize);
        }
        for &jmp_ip in &patch.continues {
            self.chunk_mut()
                .patch_jmp(jmp_ip, patch.cond_start as isize - jmp_ip as isize);
        }

        self.patch_if(test_ip, while_end);
        Ok(())
    }
}

impl Compiler<'_> {
    fn parse_arglist(&mut self) -> Result<Vec<Identifier>> {
        self.consume(&Token::LParen)
            .map_err(|_| CompileError::Unexpected {
                token: self.current(),
                diag: (
                    self.current_span().clone(),
                    "expect argument list".to_string(),
                ),
            })?;
        let mut args = Vec::new();
        while let Token::Identifier(id) = self.current() {
            args.push(id);
            self.advance()?;
            if !self.matches(&Token::Comma)? {
                break;
            }
        }
        self.consume(&Token::RParen)
            .map_err(|_| CompileError::Unexpected {
                token: self.current(),
                diag: (
                    self.current_span().clone(),
                    "expect argument list to close".to_string(),
                ),
            })?;
        Ok(args)
    }
}
