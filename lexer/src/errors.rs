use std::fmt::Display;

use thiserror;

use crate::types::Range;

#[derive(thiserror::Error, Debug)]
#[error("Parser error at {}", range.start)]
pub struct LexerError {
    range: Range,
    source: LexerErrorKind,
}

#[derive(thiserror::Error, Debug)]
pub enum LexerErrorKind {
    #[error("Invalid character sir")]
    InvalidCharacter,
}
