use std::fmt;

use lasso::{Rodeo, Spur};
use logos::Logos;

#[derive(Debug)]
pub struct LexerExtras {
    pub interner: Rodeo,
}

impl Default for LexerExtras {
    fn default() -> Self {
        LexerExtras {
            interner: Rodeo::new(),
        }
    }
}

impl LexerExtras {
    pub fn resolve(&self, spur: Spur) -> &str {
        self.interner.resolve(&spur)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Logos, Default)]
#[logos(extras = LexerExtras)]
#[logos(skip r"[ \t\n\r]+")]
#[logos(skip r"//[^\n]*")]
pub enum Token {
    // ── Keywords ────
    #[token("let")]
    Let,
    #[token("fn")]
    Fn,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("while")]
    While,
    #[token("for")]
    For,
    #[token("in")]
    In,
    #[token("return")]
    Return,
    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("nil")]
    Nil,
    #[token("and")]
    And,
    #[token("or")]
    Or,
    // TODO: add more keywords as language design settles

    // ── Literals ────
    #[regex(r"[0-9]+(\.[0-9]+)?", |lex| lex.slice().parse::<f64>().unwrap_or(f64::NAN))]
    Number(f64),

    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        let raw = lex.slice();
        let s = &raw[1..raw.len()-1];
        lex.extras.interner.get_or_intern(s)
    })]
    String(Spur),

    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| {
        lex.extras.interner.get_or_intern(lex.slice())
    })]
    Identifier(Spur),

    // ── Operators ────
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("=")]
    Equals,
    #[token("==")]
    EqEq,
    #[token("!=")]
    Neq,
    #[token("<")]
    Lt,
    #[token("<=")]
    Le,
    #[token(">")]
    Gt,
    #[token(">=")]
    Ge,
    #[token("!")]
    Bang,
    #[token(".")]
    Dot,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token("->")]
    Arrow,
    // TODO: add more operators as language design settles

    // ── Delimiters ────
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token(";")]
    Semicolon,

    // ── Special ────
    Eof,
    #[default]
    Other,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Keywords
            Token::Let => write!(f, "let"),
            Token::Fn => write!(f, "fn"),
            Token::If => write!(f, "if"),
            Token::Else => write!(f, "else"),
            Token::While => write!(f, "while"),
            Token::For => write!(f, "for"),
            Token::In => write!(f, "in"),
            Token::Return => write!(f, "return"),
            Token::True => write!(f, "true"),
            Token::False => write!(f, "false"),
            Token::Nil => write!(f, "nil"),
            Token::And => write!(f, "and"),
            Token::Or => write!(f, "or"),
            // Literals
            Token::Number(_) => write!(f, "NUMBER"),
            Token::String(_) => write!(f, "STRING"),
            Token::Identifier(_) => write!(f, "IDENT"),
            // Operators
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Star => write!(f, "*"),
            Token::Slash => write!(f, "/"),
            Token::Percent => write!(f, "%"),
            Token::Equals => write!(f, "="),
            Token::EqEq => write!(f, "=="),
            Token::Neq => write!(f, "!="),
            Token::Lt => write!(f, "<"),
            Token::Le => write!(f, "<="),
            Token::Gt => write!(f, ">"),
            Token::Ge => write!(f, ">="),
            Token::Bang => write!(f, "!"),
            Token::Dot => write!(f, "."),
            Token::Comma => write!(f, ","),
            Token::Colon => write!(f, ":"),
            Token::Arrow => write!(f, "->"),
            // Delimiters
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::LBrace => write!(f, "{{"),
            Token::RBrace => write!(f, "}}"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::Semicolon => write!(f, ";"),
            // Special
            Token::Eof => write!(f, "EOF"),
            Token::Other => write!(f, "OTHER"),
        }
    }
}
