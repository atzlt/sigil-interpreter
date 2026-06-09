use crate::{
    compiler::{
        compile::{CompileError, Compiler, Result},
        lexer::Token,
    },
    emit, emit_args,
};

impl<'a> Compiler<'a> {
    pub(super) fn statement(&mut self) -> Result<()> {
        self.clear_temp();
        match self.current.0 {
            Token::Let => self.parse_let_decl(),
            Token::Return => self.parse_return_stmt(),
            Token::Break => self.parse_break(),
            Token::LBrace => self.parse_block(),
            Token::If => self.parse_if(),
            Token::While => self.parse_while(),
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

        self.consume(&Token::Equals)?;

        let held = self.alloc_held()?;
        let rhs = self.expression()?;
        emit!(self.chunk, MOVE, held, rhs);
        self.free_other_temps(held, &[rhs]);
        self.add_local(name, held);
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
            } else {
                self.parse_block()?;
            }
            let else_end = self.chunk.end();
            self.patch_if_else(test_ip, if_end, else_start, else_end);
        } else {
            self.patch_if(test_ip, if_end);
        }

        Ok(())
    }

    fn parse_break(&mut self) -> Result<()> {
        self.consume(&Token::Break)?;
        if self.loop_exits.is_empty() {
            return Err(CompileError::Unexpected {
                token: Token::Break,
                diag: (
                    self.current.1.clone(),
                    "break outside loop".to_string(),
                ),
            });
        }
        let jmp_ip = self.emit_jmp();
        self.loop_exits.last_mut().unwrap().push(jmp_ip);
        self.consume(&Token::Semicolon)?;
        Ok(())
    }

    fn parse_while(&mut self) -> Result<()> {
        self.consume(&Token::While)?;
        self.loop_exits.push(Vec::new());
        let test_start = self.chunk.end();
        let test = self.expression()?;
        let test_ip = self.emit_test(test);
        self.regs.free_temp(test);
        self.parse_block()?;
        let body_end = self.chunk.end();
        let offset = test_start as isize - body_end as isize;
        self.emit_jump_offset(offset);
        let while_end = self.chunk.end();
        for &jmp_ip in self.loop_exits.last().unwrap() {
            self.chunk
                .patch_wide(jmp_ip + 1, (while_end - jmp_ip) as u16);
        }
        self.loop_exits.pop();
        self.patch_if(test_ip, while_end);
        Ok(())
    }
}
