use std::str::Chars;

use errors::{LexerError, LexerErrorKind};
use types::{Position, Range, Token, TokenType};

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

    fn parse_string(&mut self) -> Result<Token, LexerError> {
        let start = self.position.clone();
        let mut string = String::new();
        let mut backslashed = false;

        loop {
            let char = self.next_char().ok_or(LexerError {
                range: Range {
                    start: start.clone(),
                    end: self.position.clone(),
                },
                source: LexerErrorKind::UnterminatedString,
            })?;

            match (char, backslashed) {
                // Backslashes
                ('\\', false) => {
                    backslashed = true;
                    // continue here so we don't set backslashed back to false
                    continue;
                }
                ('\\', true) => string.push('\\'),
                // Backslashed quotes
                ('"', false) => break,
                ('"', true) => string.push('"'),
                // Backslashed single characters
                ('n', true) => string.push('\n'),
                ('t', true) => string.push('\t'),
                _ => string.push(char),
            }
            backslashed = false;
        }

        Ok(Token {
            range: Range {
                start: start.clone(),
                end: self.position.clone(),
            },
            token_type: TokenType::String(string),
        })
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token, LexerError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.skip_whitespace();

        match self.next_char() {
            // Newlines can be skipped
            Some('"') => Some(self.parse_string()),
            Some('\0') | None => None,
            _ => todo!(),
        }
    }
}
