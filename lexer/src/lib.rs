use std::str::Chars;

use errors::LexerError;
use types::{Position, Token};

mod errors;
mod types;

pub struct Lexer<'a> {
    input: Chars<'a>,

    current_char: char,
    position: Position,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input: input.chars(),
            position: Position { line: 0, char: 0 },
            // Initialze as '\0', it gets set right when the lexer starts so it doesn't really
            // matter
            current_char: '\0',
        }
    }

    fn skip_whitespace(&mut self) {
        let _ = self.input.by_ref().take_while(|&v| {
            if v == '\n' {
                self.position.line += 1;
                self.position.char = 0;
            }
            v.is_whitespace()
        });
    }

    fn next_char(&mut self) -> Option<char> {
        self.position.char += 1;
        self.current_char = self.input.next()?;
        Some(self.current_char)
    }

    fn parse_string(&mut self) -> Result<LexerError, Token<'a>> {
        let start = self.position.clone();
        let backslashed = false;

        loop {
            let char = self.next_char();
            if char == '\\' {
                backslashed = true;
            }
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<LexerError, Token<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.skip_whitespace();

        match self.next_char() {
            // Newlines can be skipped
            Some('"') => Some(self.parse_string()),
            Some('\0') | None | _ => return None,
        }
    }
}

#[cfg(test)]
mod tests {}
