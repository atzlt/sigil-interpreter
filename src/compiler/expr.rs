use lasso::Spur;

use crate::{
    compiler::{
        compile::{CompileError, Compiler, Result},
        lexer::Token,
    },
    emit, emit_args,
    functions::{FnLookupKey, LangItem},
    value::Value,
};

// ── Precedence levels (low → high) ──

const PREC_TERNARY: u8 = 10;
const PREC_OR: u8 = 20;
const PREC_AND: u8 = 30;
const PREC_EQUALITY: u8 = 40;
const PREC_COMPARISON: u8 = 50;
const PREC_TERM: u8 = 60;
const PREC_FACTOR: u8 = 70;
const PREC_UNARY: u8 = 80;
const PREC_CALL: u8 = 90;

#[derive(Clone, Copy)]
enum Assoc {
    Left,
    Right,
}

impl<'a> Compiler<'a> {
    pub(super) fn expression(&mut self, target: Option<u8>) -> Result<u8> {
        self.record_locus();
        self.parse_precedence(0, target)
    }

    fn parse_precedence(&mut self, min_bp: u8, target: Option<u8>) -> Result<u8> {
        let mut lhs = self.parse_prefix(target)?;

        loop {
            let Some((bp, assoc)) = infix_bp_assoc(&self.current()) else {
                break;
            };
            if bp < min_bp {
                break;
            }
            let op = self.current();
            self.advance()?;

            match op {
                Token::Quest => {
                    lhs = self.emit_ternary(lhs, target)?;
                }
                Token::And => {
                    lhs = self.emit_short_circuit_and(lhs, target)?;
                }
                Token::Or => {
                    lhs = self.emit_short_circuit_or(lhs, target)?;
                }
                Token::LParen => {
                    lhs = self.parse_call_expression(lhs, target)?;
                }
                Token::Dot => {
                    // Read field name, emit GETFIELD
                    let field_name = if let Token::Identifier(spur) = self.current() {
                        let n = spur;
                        self.advance()?;
                        n
                    } else {
                        return Err(CompileError::Unexpected {
                            token: self.current(),
                            diag: (
                                self.current_span().clone(),
                                format!("expected field name after '.', found {}", self.current()),
                            ),
                        });
                    };
                    let dst = self.emit_getfield(lhs, field_name)?;
                    self.frame_mut().regs.free_temp(lhs);
                    lhs = dst;
                }
                _ => {
                    let next_bp = match assoc {
                        Assoc::Left => bp + 1,
                        Assoc::Right => bp,
                    };
                    let rhs = self.parse_precedence(next_bp, target)?;
                    lhs = self.emit_binary(&op, lhs, rhs, target)?;
                }
            }
        }

        Ok(lhs)
    }

    fn parse_prefix(&mut self, target: Option<u8>) -> Result<u8> {
        match &self.current() {
            Token::Number(n) => {
                let val = *n;
                self.advance()?;
                self.emit_number(val)
            }
            Token::String(spur) => {
                let spur = *spur;
                self.advance()?;
                self.emit_string(spur)
            }
            Token::BooleanLit(b) => {
                let b = *b;
                self.advance()?;
                self.emit_bool(b)
            }
            Token::Nil => {
                self.advance()?;
                self.emit_nil()
            }
            Token::Fn => {
                self.advance()?;
                self.parse_closure_expr(target)
            }
            Token::Identifier(name) => {
                let name = *name;
                self.advance()?;
                // Struct construction: `Name { ... }` or `Name(...)` — resolve via type registry
                if self.type_registry.resolve_struct(name).is_some() {
                    if self.check(&Token::LBrace) {
                        return self.parse_struct_literal_named(name, target);
                    }
                    if self.check(&Token::LParen) {
                        return self.parse_struct_literal_positional(name, target);
                    }
                }
                self.emit_identifier(name)
            }
            Token::Minus => {
                self.advance()?;
                let inner = self.parse_precedence(PREC_UNARY, target)?;
                self.emit_unary(FnLookupKey::LangItem(LangItem::Neg), inner, target)
            }
            Token::Bang => {
                self.advance()?;
                let inner = self.parse_precedence(PREC_UNARY, target)?;
                self.emit_unary(FnLookupKey::LangItem(LangItem::Not), inner, target)
            }
            Token::LParen => {
                let open_span = self.current_span().clone();
                self.advance()?;
                let inner = self.expression(target)?;
                self.consume_close(&Token::RParen, open_span)?;
                Ok(inner)
            }
            _ => Err(CompileError::Unexpected {
                token: self.current(),
                diag: (
                    self.current_span().clone(),
                    format!("expected expression, found {}", &self.current()),
                ),
            }),
        }
    }

    fn parse_closure_expr(&mut self, target: Option<u8>) -> Result<u8> {
        let args = self.parse_arglist()?;

        let chunk_idx = self.new_frame(&args);

        if self.current() == Token::LBrace {
            self.parse_block()?;
            self.emit_safety_net()?;
        } else {
            // Expression body: `fn(x, y) expr`
            let reg = self.expression(None)?;
            emit!(self.chunk_mut(), RETURN, reg);
        }

        let upvalues = std::mem::take(&mut self.frame_mut().upvalues);
        self.exit_frame()?;

        let anon_id = self.next_anon_id();
        let fn_id = self
            .funcs
            .register(FnLookupKey::Anon(anon_id), chunk_idx);
        let upvalue_count = upvalues.len() as u16;
        let proto_idx = self.chunk_mut().add_constant(Value::FnProto {
            fn_id,
            upvalue_count,
        });

        let reg = self.emit_closure_temp(proto_idx, &upvalues)?;
        // If the caller wants the result in a specific register, move it.
        if let Some(t) = target {
            self.emit_move(t, reg);
            self.frame_mut().regs.free_temp(reg);
            Ok(t)
        } else {
            Ok(reg)
        }
    }


    // ── Bytecode emission helpers (now in emit.rs) ──

    fn emit_binary(&mut self, op: &Token, lhs: u8, rhs: u8, target: Option<u8>) -> Result<u8> {
        self.emit_lang_item_call(binary_op_lang_item(op), &[lhs, rhs], target)
    }

    fn emit_unary(&mut self, fun: FnLookupKey, operand: u8, target: Option<u8>) -> Result<u8> {
        self.emit_lang_item_call(fun, &[operand], target)
    }

    fn emit_ternary(&mut self, test: u8, target: Option<u8>) -> Result<u8> {
        let skip_then = self.emit_forward_test(test);
        let reg = self.target_or_reuse(target, &[test])?;
        self.free_other_temps(reg, &[test]);

        let mhs = self.parse_precedence(0, target)?;
        self.emit_move(reg, mhs);
        let skip_else = self.emit_forward_jmp();
        self.free_other_temps(reg, &[mhs]);

        self.place_label(skip_then);
        self.consume(&Token::Colon)?;

        let rhs = self.parse_precedence(PREC_TERNARY, target)?;
        self.emit_move(reg, rhs);
        self.free_other_temps(reg, &[rhs]);

        self.place_label(skip_else);
        Ok(reg)
    }

    fn emit_short_circuit_or(&mut self, lhs: u8, target: Option<u8>) -> Result<u8> {
        let skip_rhs = self.emit_forward_test(lhs);
        let reg = self.target_or_reuse(target, &[lhs])?;
        self.free_other_temps(reg, &[lhs]);

        self.emit_move(reg, lhs);
        let skip_end = self.emit_forward_jmp();

        self.place_label(skip_rhs);
        let rhs = self.parse_precedence(PREC_OR + 1, target)?;
        self.emit_move(reg, rhs);
        self.free_other_temps(reg, &[rhs]);

        self.place_label(skip_end);
        Ok(reg)
    }

    fn emit_short_circuit_and(&mut self, lhs: u8, target: Option<u8>) -> Result<u8> {
        let skip_rhs = self.emit_forward_test(lhs);
        let reg = self.target_or_reuse(target, &[lhs])?;
        self.free_other_temps(reg, &[lhs]);

        let rhs = self.parse_precedence(PREC_AND + 1, target)?;
        self.emit_move(reg, rhs);
        let skip_end = self.emit_forward_jmp();

        self.place_label(skip_rhs);
        self.emit_move(reg, lhs);
        self.free_other_temps(reg, &[rhs]);

        self.place_label(skip_end);
        Ok(reg)
    }

    fn parse_call_expression(&mut self, lhs: u8, target: Option<u8>) -> Result<u8> {
        let mut args = vec![];
        loop {
            if self.current() == Token::RParen {
                break;
            }
            let arg = self.expression(target)?;
            args.push(arg);
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
        let dst = self.target_or_reuse(target, &[lhs])?;
        self.emit_call(dst, lhs, self.reg_watermark(), &args);
        Ok(dst)
    }

    /// Parse `Name { field: expr, ... }` struct construction.
    fn parse_struct_literal_named(
        &mut self,
        struct_name: Spur,
        target: Option<u8>,
    ) -> Result<u8> {
        let def_id = self
            .type_registry
            .resolve_struct(struct_name)
            .ok_or_else(|| CompileError::UndefinedVariable {
                name: self.intern_resolve(&struct_name).to_string(),
                diag: (self.prev_span().clone(), "undefined struct type".to_string()),
            })?;

        let def = self.type_registry.get_struct(def_id).clone();
        let field_count = def.fields.len();

        self.consume(&Token::LBrace)?;

        // Collect field expressions in struct declaration order.
        // We support named fields in any order (matched by name).
        let mut field_regs: Vec<Option<u8>> = vec![None; field_count];
        let mut seen = vec![false; field_count];

        loop {
            if self.check(&Token::RBrace) || self.check(&Token::Eof) {
                break;
            }

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

            self.consume(&Token::Colon)?;

            let val_reg = self.expression(None)?;

            let idx = def
                .fields
                .iter()
                .position(|(n, _)| *n == field_name)
                .ok_or_else(|| CompileError::Unexpected {
                    token: Token::Identifier(field_name),
                    diag: (
                        self.prev_span().clone(),
                        format!(
                            "no field '{}' in struct '{}'",
                            self.intern_resolve(&field_name),
                            self.intern_resolve(&struct_name),
                        ),
                    ),
                })?;

            if seen[idx] {
                return Err(CompileError::Unexpected {
                    token: Token::Identifier(field_name),
                    diag: (
                        self.prev_span().clone(),
                        format!(
                            "duplicate field '{}'",
                            self.intern_resolve(&field_name),
                        ),
                    ),
                });
            }
            seen[idx] = true;
            field_regs[idx] = Some(val_reg);

            if self.check(&Token::Comma) {
                self.advance()?;
            } else {
                break;
            }
        }

        self.consume(&Token::RBrace)?;

        // All fields must be provided — missing fields are an error.
        for (i, reg_opt) in field_regs.iter().enumerate() {
            if reg_opt.is_none() {
                let field_name = self.intern_resolve(&def.fields[i].0).to_string();
                let struct_name = self.intern_resolve(&struct_name).to_string();
                return Err(CompileError::MissingField {
                    field: field_name,
                    struct_name,
                    diag: (self.prev_span().clone(), "missing field".to_string()),
                });
            }
        }

        let regs: Vec<u8> = field_regs.into_iter().map(|r| r.unwrap()).collect();
        let dst = self.target_or_reuse(target, &regs)?;
        self.emit_newstruct(dst, def_id, &def, &regs);
        self.free_other_temps(dst, &regs);
        Ok(dst)
    }

    /// Parse `Name(val, val, ...)` positional struct construction.
    fn parse_struct_literal_positional(
        &mut self,
        struct_name: Spur,
        target: Option<u8>,
    ) -> Result<u8> {
        let def_id = self
            .type_registry
            .resolve_struct(struct_name)
            .ok_or_else(|| CompileError::UndefinedVariable {
                name: self.intern_resolve(&struct_name).to_string(),
                diag: (self.prev_span().clone(), "undefined struct type".to_string()),
            })?;

        let def = self.type_registry.get_struct(def_id).clone();
        let field_count = def.fields.len();

        self.consume(&Token::LParen)?;

        let mut regs: Vec<u8> = Vec::with_capacity(field_count);
        loop {
            if self.check(&Token::RParen) {
                break;
            }
            let arg = self.expression(None)?;
            regs.push(arg);
            if !self.matches(&Token::Comma)? {
                break;
            }
        }
        self.consume(&Token::RParen)?;

        if regs.len() != field_count {
            return Err(CompileError::Unexpected {
                token: self.current(),
                diag: (
                    self.current_span().clone(),
                    format!(
                        "struct '{}' expects {} fields, got {}",
                        self.intern_resolve(&struct_name),
                        field_count,
                        regs.len(),
                    ),
                ),
            });
        }

        let dst = self.target_or_reuse(target, &regs)?;
        self.emit_newstruct(dst, def_id, &def, &regs);
        self.free_other_temps(dst, &regs);
        Ok(dst)
    }
}

fn binary_op_lang_item(op: &Token) -> FnLookupKey {
    match op {
        Token::Plus => FnLookupKey::LangItem(LangItem::Add),
        Token::Minus => FnLookupKey::LangItem(LangItem::Sub),
        Token::Star => FnLookupKey::LangItem(LangItem::Mul),
        Token::Slash => FnLookupKey::LangItem(LangItem::Div),
        Token::Percent => FnLookupKey::LangItem(LangItem::Rem),
        Token::Equal => FnLookupKey::LangItem(LangItem::Eq),
        Token::Neq => FnLookupKey::LangItem(LangItem::Neq),
        Token::Lt => FnLookupKey::LangItem(LangItem::Lt),
        Token::Le => FnLookupKey::LangItem(LangItem::Le),
        Token::Gt => FnLookupKey::LangItem(LangItem::Gt),
        Token::Ge => FnLookupKey::LangItem(LangItem::Ge),
        // TODO: add more operators as language design settles
        _ => unreachable!("binary_op_lang_item called with non-binary token {op:?}"),
    }
}

// ── Binding power tables ──

fn infix_bp_assoc(tok: &Token) -> Option<(u8, Assoc)> {
    match tok {
        Token::Quest => Some((PREC_TERNARY, Assoc::Right)),
        Token::Or => Some((PREC_OR, Assoc::Left)),
        Token::And => Some((PREC_AND, Assoc::Left)),
        Token::Equal | Token::Neq => Some((PREC_EQUALITY, Assoc::Left)),
        Token::Lt | Token::Le | Token::Gt | Token::Ge => Some((PREC_COMPARISON, Assoc::Left)),
        Token::Plus | Token::Minus => Some((PREC_TERM, Assoc::Left)),
        Token::Star | Token::Slash | Token::Percent => Some((PREC_FACTOR, Assoc::Left)),
        Token::LParen => Some((PREC_CALL, Assoc::Left)),
        Token::Dot => Some((PREC_CALL, Assoc::Left)),
        // TODO: add more infix operators as language design settles
        _ => None,
    }
}
