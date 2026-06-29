use lasso::Spur;

use crate::{
    compiler::{
        compile::{CompileError, Compiler, Result},
        lexer::Token,
        type_registry::TypeId,
    },
    emit, emit_args,
    functions::{FnLookupKey, FnModifier, LangItem},
    value::Value,
};

type Identifier = Spur;

enum JumpKind {
    Break,
    Continue,
}

impl<'a> Compiler<'a> {
    pub(super) fn statement(&mut self) -> Result<()> {
        match self.statement_inner() {
            Err(e) => {
                self.record_error(e);
                self.sync();
                Ok(())
            }
            ok => ok,
        }
    }

    fn statement_inner(&mut self) -> Result<()> {
        self.clear_temp();
        self.record_locus();
        match self.current() {
            Token::Let => self.parse_let_decl(),
            Token::Fn => self.parse_fn_decl(Vec::new()),
            Token::Struct => self.parse_struct_decl(),
            Token::Semicolon => {
                self.advance()?;
                Ok(())
            }
            Token::At => {
                let mut modifiers = Vec::new();
                while self.current() == Token::At {
                    self.consume(&Token::At)?;
                    modifiers.push(self.parse_modifier()?);
                }
                self.parse_fn_decl(modifiers)
            }
            Token::Return => self.parse_return_stmt(),
            Token::Break => self.parse_jump(JumpKind::Break),
            Token::Continue => self.parse_jump(JumpKind::Continue),
            Token::LBrace => self.parse_block(),
            Token::If => self.parse_if(),
            Token::While => self.parse_while(),
            Token::Identifier(id) => {
                // Peek to distinguish: `id = ...` or `id.field... = ...` or expr stmt
                let next = *self.peek()?;
                if next == Token::Assign || next == Token::Dot {
                    self.advance()?; // consume identifier
                    self.parse_assign_lhs(id)?;
                    self.consume(&Token::Semicolon)?;
                    Ok(())
                } else {
                    self.parse_expr_stmt()
                }
            }
            _ => self.parse_expr_stmt(),
        }
    }

    pub(super) fn parse_block(&mut self) -> Result<()> {
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

    fn parse_fn_decl(&mut self, modifiers: Vec<FnModifier>) -> Result<()> {
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

        let is_intrinsic = modifiers.iter().any(|m| matches!(m, FnModifier::Intrinsic));
        let lang_item = modifiers.iter().find_map(|m| match m {
            FnModifier::LangItem(item) => Some(*item),
            _ => None,
        });

        let is_top = self.is_top_level();

        if is_top {
            let id = if is_intrinsic {
                let name_str = self.intern_resolve(&name);
                self.funcs
                    .get_id(&FnLookupKey::Name(name))
                    .or_else(|| {
                        self.funcs
                            .get_id(&FnLookupKey::External(name_str.to_string()))
                    })
                    .copied()
                    .ok_or_else(|| CompileError::UndefinedFunction {
                        name: name_str.to_string(),
                        diag: (
                            self.current_span().clone(),
                            "intrinsic function not provided by runtime".to_string(),
                        ),
                    })?
            } else {
                let chunk_idx = self.chunks.len();
                if let Some(item) = lang_item {
                    self.funcs.register(FnLookupKey::LangItem(item), chunk_idx);
                }
                self.funcs.register(FnLookupKey::Name(name), chunk_idx)
            };
            self.store_global_fn(name, id)?;
        }

        if is_intrinsic {
            self.consume(&Token::Semicolon)?;
        } else {
            let chunk_idx = self.new_frame(&args);
            self.parse_block()?;
            self.emit_safety_net()?;

            if is_top {
                self.exit_frame()?;
            } else {
                // Nested function: capture upvalues, register, emit CLOSURE.
                let upvalues = std::mem::take(&mut self.frame_mut().upvalues);
                self.exit_frame()?;

                let fn_id = self
                    .funcs
                    .register(FnLookupKey::Name(name), chunk_idx);
                let upvalue_count = upvalues.len() as u16;
                let proto_idx = self
                    .chunk_mut()
                    .add_constant(Value::FnProto {
                        fn_id,
                        upvalue_count,
                    });
                self.emit_closure(name, proto_idx, &upvalues)?;
            }
        }

        Ok(())
    }

    fn parse_struct_decl(&mut self) -> Result<()> {
        self.consume(&Token::Struct)?;

        let name = if let Token::Identifier(spur) = self.current() {
            let name = spur;
            self.advance()?;
            name
        } else {
            return Err(CompileError::Unexpected {
                token: self.current(),
                diag: (
                    self.current_span().clone(),
                    format!("expected struct name, found {}", self.current()),
                ),
            });
        };

        // {
        self.consume(&Token::LBrace)?;

        let mut fields: Vec<(Spur, TypeId)> = Vec::new();
        while !self.check(&Token::RBrace) && !self.check(&Token::Eof) {
            let field_name = if let Token::Identifier(spur) = self.current() {
                let n = spur;
                self.advance()?;
                n
            } else {
                return Err(CompileError::Unexpected {
                    token: self.current(),
                    diag: (
                        self.current_span().clone(),
                        format!("expected field name, found {}", self.current()),
                    ),
                });
            };

            // :
            self.consume(&Token::Colon)?;

            let type_name = if let Token::Identifier(spur) = self.current() {
                let tn = spur;
                self.advance()?;
                tn
            } else {
                return Err(CompileError::Unexpected {
                    token: self.current(),
                    diag: (
                        self.current_span().clone(),
                        format!("expected type name, found {}", self.current()),
                    ),
                });
            };

            let type_name_str = self.intern_resolve(&type_name);
            let type_id = self
                .type_registry
                .resolve_builtin_type_name(type_name_str)
                .or_else(|| {
                    // Also try resolving as a previously-declared struct name
                    self.type_registry
                        .resolve_struct(type_name)
                        .map(TypeId::Struct)
                })
                .ok_or_else(|| CompileError::Unexpected {
                    token: Token::Identifier(type_name),
                    diag: (
                        self.prev_span().clone(),
                        format!("unknown type: {type_name_str}"),
                    ),
                })?;

            fields.push((field_name, type_id));

            if self.check(&Token::Comma) {
                self.advance()?;
            } else {
                break;
            }
        }

        self.consume(&Token::RBrace)?;

        self.type_registry.declare_struct(name, fields);

        Ok(())
    }

    fn parse_modifier(&mut self) -> Result<FnModifier> {
        let spur = if let Token::Identifier(spur) = self.current() {
            self.advance()?;
            spur
        } else {
            return Err(CompileError::Unexpected {
                token: self.current(),
                diag: (
                    self.prev_span().clone(),
                    "expected modifier name after @".to_string(),
                ),
            });
        };
        if self.spur_eq(spur, "intrinsic") {
            Ok(FnModifier::Intrinsic)
        } else if self.spur_eq(spur, "lang_item") {
            self.consume(&Token::LParen)
                .map_err(|_| CompileError::Unexpected {
                    token: self.current(),
                    diag: (
                        self.prev_span().clone(),
                        "expected '(' after @lang_item".to_string(),
                    ),
                })?;
            let item_spur = if let Token::Identifier(spur) = self.current() {
                self.advance()?;
                spur
            } else {
                return Err(CompileError::Unexpected {
                    token: self.current(),
                    diag: (
                        self.prev_span().clone(),
                        "expected lang item name".to_string(),
                    ),
                });
            };
            let item_name = self.intern_resolve(&item_spur);
            let lang_item =
                LangItem::from_name(item_name).ok_or_else(|| CompileError::Unexpected {
                    token: self.current(),
                    diag: (
                        self.prev_span().clone(),
                        format!("unknown lang item: {item_name}"),
                    ),
                })?;
            self.consume(&Token::RParen)
                .map_err(|_| CompileError::Unexpected {
                    token: self.current(),
                    diag: (
                        self.prev_span().clone(),
                        "expected ')' after @lang_item".to_string(),
                    ),
                })?;
            Ok(FnModifier::LangItem(lang_item))
        } else {
            let name = self.intern_resolve(&spur);
            Err(CompileError::Unexpected {
                token: Token::Other,
                diag: (
                    self.prev_span().clone(),
                    format!("unknown modifier: @{name}"),
                ),
            })
        }
    }

    fn parse_assign_lhs(&mut self, id: Identifier) -> Result<()> {
        let span = self.prev_span().clone();

        if self.check(&Token::Assign) {
            self.advance()?;
            return self.emit_simple_assign(id, span);
        }

        let mut fields: Vec<Spur> = Vec::new();
        loop {
            self.consume(&Token::Dot)?; // consume '.'
            let field = if let Token::Identifier(spur) = self.current() {
                let s = spur;
                self.advance()?;
                s
            } else {
                return Err(CompileError::Unexpected {
                    token: self.current(),
                    diag: (
                        self.current_span().clone(),
                        format!("expected field name after '.', found {}", self.current()),
                    ),
                });
            };
            fields.push(field);

            if self.check(&Token::Assign) {
                self.advance()?;
                return self.emit_field_chain_assign(id, &fields);
            }
            if !self.check(&Token::Dot) {
                // without '=' — treat as expression statement.
                let reg = self.emit_field_chain_get(id, &fields)?;
                self.frame_mut().regs.free_temp(reg);
                return Ok(());
            }
        }
    }

    /// Emit a simple assignment: `id = expr`.
    fn emit_simple_assign(&mut self, id: Identifier, span: std::ops::Range<usize>) -> Result<()> {
        if let Some(local) = self.try_resolve_local(id) {
            let reg = self.expression(Some(local))?;
            self.emit_move(local, reg);
            self.frame_mut().regs.free_temp(reg);
        } else if let Some(upvalue) = self.resolve_upvalue(id) {
            let reg = self.expression(None)?;
            emit!(self.chunk_mut(), SETUPVAL, reg, wide upvalue as u16);
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
        let skip_then = self.emit_forward_test(test);
        self.frame_mut().regs.free_temp(test);
        self.parse_block()?;

        if self.matches(&Token::Else)? {
            let skip_else = self.emit_forward_jmp();
            self.place_label(skip_then);
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
            self.place_label(skip_else);
        } else {
            self.place_label(skip_then);
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
        let label = match kind {
            JumpKind::Break => self.frame().loops.break_label(),
            JumpKind::Continue => self.frame().loops.continue_label(),
        };
        match kind {
            JumpKind::Break => self.emit_forward_jmp_to(label),
            JumpKind::Continue => self.emit_jmp_to(label),
        }
        self.consume(&Token::Semicolon)?;
        Ok(())
    }

    fn parse_while(&mut self) -> Result<()> {
        self.consume(&Token::While)?;
        let cond = self.emit_here_label();
        let test = self.expression(None)?;
        let end = self.new_label();
        let skip = self.emit_forward_test(test);
        self.frame_mut().regs.free_temp(test);
        self.frame_mut().loops.push_loop(end, cond);
        self.parse_block()?;
        self.frame_mut().loops.pop_loop();
        self.emit_jmp_to(cond);
        self.place_label(end);
        self.place_label(skip);
        Ok(())
    }
}

impl Compiler<'_> {
    pub(super) fn parse_arglist(&mut self) -> Result<Vec<Identifier>> {
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
                    "expect argument list to close here".to_string(),
                ),
            })?;
        Ok(args)
    }
}
