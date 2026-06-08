use logos::Logos;

#[derive(Debug, Clone, PartialEq, Logos, Default)]
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
        raw[1..raw.len()-1].to_string()
    })]
    String(String),

    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Identifier(String),

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
    #[default]
    EOF,
}
