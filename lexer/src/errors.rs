use std::fmt::Display;

use thiserror;

use crate::types::Range;

#[derive(thiserror::Error, Debug, PartialEq)]
#[error("Parser error at {}", range.start)]
pub struct LexerError {
    pub range: Range,
    pub source: LexerErrorKind,
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum LexerErrorKind {
    #[error("invalid character sir")]
    InvalidCharacter,
    #[error("unterminated string")]
    UnterminatedString,
    #[error("invalid number Sir")]
    InvalidNumber,
}
