use std::fmt::Display;

use thiserror;

use crate::types::Range;

#[derive(thiserror::Error, Debug)]
#[error("Parser error at {}", range.start)]
pub struct LexerError {
    pub range: Range,
    pub source: LexerErrorKind,
}

#[derive(thiserror::Error, Debug)]
pub enum LexerErrorKind {
    #[error("invalid character sir")]
    InvalidCharacter,
    #[error("unterminated string")]
    UnterminatedString,
}
