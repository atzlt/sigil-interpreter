use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    #[error("syntax error")]
    SyntaxError(String)
}
