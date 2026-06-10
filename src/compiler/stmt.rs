use lasso::Spur;

use crate::{
    compiler::{
        compile::{CompileError, Compiler, Result},
        lexer::Token,
    },
    emit, emit_args,
};

enum JumpKind {
    Break,
    Continue,
}

impl<'a> Compiler<'a> {
    pub(super) fn statement(&mut self) -> Result<()> {
        self.clear_temp();
        self.record_locus();
        match self.current.0 {
            Token::Let => self.parse_let_decl(),
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
        let open_span = self.current.1.clone();
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

        let name = if let Token::Identifier(spur) = self.current.0 {
            let name = spur;
            self.advance()?;
            name
        } else {
            return Err(CompileError::Unexpected {
                token: self.current.0,
                diag: (
                    self.current.1.clone(),
                    format!("expected identifier, found {}", self.current.0),
                ),
            });
        };

        self.consume(&Token::Assign)?;

        let rhs = self.expression()?;

        if self.is_top_level() {
            let slot = self.declare_global(name);
            self.regs.free_temp(rhs);
            emit!(self.chunk, SETGLB, wide slot, rhs);
        } else {
            let held = self.alloc_held()?;
            emit!(self.chunk, MOVE, held, rhs);
            self.free_other_temps(held, &[rhs]);
            self.add_local(name, held);
        }

        self.consume(&Token::Semicolon)?;
        Ok(())
    }

    fn parse_assign(&mut self, id: Spur) -> Result<()> {
        let span = self.prev_span.clone();
        self.advance()?;
        dbg!(self.current.0);
        let reg = self.expression()?;
        if let Some(local) = self.try_resolve_local(id) {
            emit!(self.chunk, MOVE, local, reg);
        } else if let Some(global) = self.resolve_global(id) {
            emit!(self.chunk, SETGLB, wide global, reg);
        } else {
            return Err(CompileError::UndefinedVariable {
                name: self.intern_resolve(&id).to_string(),
                diag: (span, "undefined variable".to_string()),
            });
        }
        self.regs.free_temp(reg);
        self.consume(&Token::Semicolon)?;
        Ok(())
    }

    fn parse_return_stmt(&mut self) -> Result<()> {
        self.consume(&Token::Return)?;
        if self.check(&Token::Semicolon) {
            self.consume(&Token::Semicolon)?;
            let reg = self.alloc_temp()?;
            emit!(self.chunk, LOADNIL, reg);
            emit!(self.chunk, RETURN, reg);
        } else {
            let reg = self.expression()?;
            self.consume(&Token::Semicolon)?;
            emit!(self.chunk, RETURN, reg);
        }
        Ok(())
    }

    fn parse_expr_stmt(&mut self) -> Result<()> {
        let reg = self.expression()?;
        self.regs.free_temp(reg);
        self.consume(&Token::Semicolon)?;
        Ok(())
    }

    fn parse_if(&mut self) -> Result<()> {
        self.consume(&Token::If)?;
        let test = self.expression()?;
        let test_ip = self.chunk.end();
        self.emit_test(test);
        self.regs.free_temp(test);
        self.parse_block()?;
        let if_end = self.chunk.end();

        if self.matches(&Token::Else)? {
            let if_end = self.emit_jmp();
            let else_start = self.chunk.end();
            if self.check(&Token::If) {
                self.parse_if()?;
            } else if self.check(&Token::LBrace) {
                self.parse_block()?;
            } else {
                return Err(CompileError::Unexpected {
                    token: self.current.0,
                    diag: (
                        self.current.1.clone(),
                        format!(
                            "expected else-if clause or else clause, found {}",
                            self.current.0
                        ),
                    ),
                });
            }
            let else_end = self.chunk.end();
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
        if !self.loops.in_loop() {
            return Err(CompileError::Unexpected {
                token,
                diag: (self.prev_span.clone(), format!("{token} outside loop")),
            });
        }
        let jmp_ip = self.emit_jmp();
        match kind {
            JumpKind::Break => self.loops.add_break(jmp_ip),
            JumpKind::Continue => self.loops.add_continue(jmp_ip),
        }
        self.consume(&Token::Semicolon)?;
        Ok(())
    }

    fn parse_while(&mut self) -> Result<()> {
        self.consume(&Token::While)?;
        let test_start = self.chunk.end();
        self.loops.push_loop(test_start);
        let test = self.expression()?;
        let test_ip = self.emit_test(test);
        self.regs.free_temp(test);
        self.parse_block()?;
        let body_end = self.chunk.end();
        let offset = test_start as isize - body_end as isize;
        self.emit_jump_offset(offset);
        let while_end = self.chunk.end();
        let patch = self.loops.pop_loop();

        for &jmp_ip in &patch.breaks {
            self.chunk
                .patch_wide(jmp_ip + 1, (while_end as isize - jmp_ip as isize) as u16);
        }
        for &jmp_ip in &patch.continues {
            self.chunk.patch_wide(
                jmp_ip + 1,
                (patch.cond_start as isize - jmp_ip as isize) as u16,
            );
        }

        self.patch_if(test_ip, while_end);
        Ok(())
    }
}
