use std::str::Chars;

use types::Token;

mod types;
mod errors;

pub struct Lexer<'a> {
    input: Chars<'a>,
    position: usize,

    current_char: char,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input: input.chars(),
            position: 0,
            // Initialze as '\0', it gets set right when the lexer starts so it doesn't really
            // matter
            current_char: '\0',
        }
    }
}
