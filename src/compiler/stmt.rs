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
            Token::LBrace => self.parse_block(),
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
        self.free_others(held, &[rhs]);
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
        self.regs.free_reg(reg);
        self.consume(&Token::Semicolon)?;
        Ok(())
    }
}
