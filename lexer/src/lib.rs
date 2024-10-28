use std::{iter::Peekable, str::Chars};

use errors::{LexerError, LexerErrorKind};
use types::{Position, Range, Token, TokenType};

mod errors;
mod types;

pub struct Lexer<'a> {
    input: Peekable<Chars<'a>>,

    current_char: char,
    position: Position,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input: input.chars().peekable(),
            position: Position { line: 0, char: 0 },
            // Initialze as '\0', it gets set right when the lexer starts so it doesn't really
            // matter
            current_char: '\0',
        }
    }

    pub fn err(&self, start: Position, source: LexerErrorKind) -> LexerError {
        LexerError {
            range: Range {
                start,
                end: self.position.clone(),
            },
            source,
        }
    }

    fn skip_whitespace(&mut self) {
        let _ = self.input.by_ref().take_while(|&v| {
            self.position.char += 1;
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

    fn parse_ident(&mut self) -> Result<Token, LexerError> {
        let start = self.position.clone();

        // String needs to start off with the current character !!!
        let mut ident = String::from(self.current_char);

        loop {
            let Some(char) = self.next_char() else {
                break;
            };

            match char {
                // Idents only support A-z, 0-9, and _
                'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => ident.push(char),
                _ => break,
            }
        }
        Ok(Token {
            range: Range {
                start,
                end: self.position.clone(),
            },
            token_type: TokenType::Ident(ident),
        })
    }

    fn parse_string(&mut self) -> Result<Token, LexerError> {
        let start = self.position.clone();
        let mut string = String::new();
        let mut backslashed = false;

        loop {
            let char = self
                .next_char()
                .ok_or_else(|| self.err(start.clone(), LexerErrorKind::UnterminatedString))?;

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

        match self.next_char()? {
            '"' => Some(self.parse_string()),
            'a'..='z' | 'A'..='Z' | '_' => Some(self.parse_ident()),
            '\0' => None,
            _ => todo!(),
        }
    }
}
