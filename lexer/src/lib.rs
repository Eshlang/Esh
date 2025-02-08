use std::{iter::Peekable, str::Chars};

use errors::{LexerError, LexerErrorKind};
use types::{Keyword, Position, Range, Token, TokenType};

mod errors;
pub mod types;

#[derive(Debug)]
pub struct Lexer<'a> {
    input: Peekable<Chars<'a>>,

    current_char: char,
    position: Position,

    /// Used to ensure that when there is a new line, we do not increment
    /// [Self.position.line](Position::line) right away. This way, we can have the position start
    /// at 0 and not 1
    started_line: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input: input.chars().peekable(),
            position: Position { line: 0, char: 0 },
            // Initialze as '\0', it gets set right when the lexer starts so it doesn't really
            // matter
            current_char: '\0',
            started_line: true,
        }
    }

    /// Generates a lovely error for the lexer.
    ///
    /// The error will span from `start` to whatever [Self::position](field@Self::position) is currently at
    fn err(&self, start: Position, source: LexerErrorKind) -> LexerError {
        LexerError {
            range: Range {
                start,
                end: self.position.clone(),
            },
            source,
        }
    }

    /// Go past all the whitespace grr
    fn skip_whitespace(&mut self) {
        loop {
            let Some(char) = self.input.peek() else {
                break;
            };
            if !char.is_whitespace() {
                break;
            }
            let _ = self.next_char();
        }
    }

    /// Can i have the next character please Sir?
    fn next_char(&mut self) -> Option<char> {
        // We do not want to increment the position on the first character in a line.
        if !self.started_line {
            self.position.char += 1;
        }
        self.started_line = false;
        self.current_char = self.input.next()?;
        if self.current_char == '\n' {
            self.position.line += 1;
            self.position.char = 0;
            self.started_line = true;
        }
        Some(self.current_char)
    }

    /// Parse an identifier
    fn parse_ident(&mut self) -> Result<Token, LexerError> {
        let start = self.position.clone();

        // String needs to start off with the current character !!!
        let mut ident = String::from(self.current_char);

        loop {
            let Some(char) = self.input.peek() else {
                break;
            };
            match char {
                // Idents only support A-z, 0-9, and _
                'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => {
                    ident.push(*char);
                    self.next_char();
                }
                _ => break,
            }
        }
        Ok(Token {
            range: Range {
                start,
                end: self.position.clone(),
            },
            token_type: match Self::keyword_from_ident(&ident) {
                Some(keyword) => TokenType::Keyword(keyword),
                None => TokenType::Ident(ident),
            },
        })
    }

    fn keyword_from_ident(input: &str) -> Option<Keyword> {
        match input {
            "func" => Some(Keyword::Func),
            "if" => Some(Keyword::If),
            "else" => Some(Keyword::Else),
            "return" => Some(Keyword::Return),
            "break" => Some(Keyword::Break),
            "true" => Some(Keyword::True),
            "self" => Some(Keyword::SelfIdentity),
            "false" => Some(Keyword::False),
            "struct" => Some(Keyword::Struct),
            "domain" => Some(Keyword::Domain),
            "for" => Some(Keyword::For),
            "while" => Some(Keyword::While),

            _ => None,
        }
    }

    fn parse_number(&mut self) -> Result<Token, LexerError> {
        let start = self.position.clone();
        let mut decimal = false;
        let mut string = String::from(self.current_char);

        loop {
            let Some(char) = self.input.peek() else {
                break;
            };

            match char {
                '0'..='9' => string.push(*char),
                '.' if !decimal => {
                    decimal = true;
                    string.push(*char);
                }
                // If we meet any invalid character, we can break.
                //
                // This will create a problem in the parser if there are numbers followed by an
                // ident/string with no space but that is not my problem :smiling_imp:
                //
                // TODO fix this later because it is really stupid
                _ => break,
            };
            let _ = self.next_char();
        }
        Ok(Token {
            range: Range {
                start: start.clone(),
                end: self.position.clone(),
            },
            token_type: TokenType::Number(
                string.parse::<f64>().expect("We made sure it's a number"),
            ),
        })
    }

    /// parse out a string
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
                (_, true) => {
                    // TODO this prob;ably isnt a good handler for backslashes? idk we should maybe
                    // think about how this should be handled
                    string.push('\\');
                    string.push(char)
                }
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

    /// Converts a [TokenType] into a [Token]
    fn type_to_token(&self, token_type: TokenType) -> Token {
        Token {
            range: Range {
                start: self.position.clone(),
                end: self.position.clone(),
            },
            token_type,
        }
    }

    /// Will return a Token if it makes sense ok i dont want to write documen tation rn
    fn parse_char_lookahead(
        &mut self,
        token_type: TokenType,
        lookahead: (char, TokenType),
    ) -> Result<Token, LexerError> {
        if self.input.peek() == Some(&lookahead.0) {
            // Skip over the character :grin:
            let start = self.position.clone();
            let _ = self.next_char();
            Ok(Token {
                range: Range {
                    start,
                    end: self.position.clone(),
                },
                token_type: lookahead.1,
            })
        } else {
            Ok(Token {
                range: Range {
                    start: self.position.clone(),
                    end: self.position.clone(),
                },
                token_type,
            })
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token, LexerError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.skip_whitespace();

        let char = self.next_char()?;
        match char {
            '"' => Some(self.parse_string()),
            'a'..='z' | 'A'..='Z' | '_' => Some(self.parse_ident()),
            '0'..='9' => Some(self.parse_number()),

            // Single character tokens
            '(' => Some(Ok(self.type_to_token(TokenType::LParen))),
            ')' => Some(Ok(self.type_to_token(TokenType::RParen))),
            '{' => Some(Ok(self.type_to_token(TokenType::LBrace))),
            '}' => Some(Ok(self.type_to_token(TokenType::RBrace))),
            '[' => Some(Ok(self.type_to_token(TokenType::LBracket))),
            ']' => Some(Ok(self.type_to_token(TokenType::RBracket))),
            '.' => Some(Ok(self.type_to_token(TokenType::Dot))),
            ',' => Some(Ok(self.type_to_token(TokenType::Comma))),
            ';' => Some(Ok(self.type_to_token(TokenType::Semicolon))),
            ':' => Some(Ok(self.type_to_token(TokenType::Colon))),
            // TODO Make these guys have a += and maybe even a ++??!!
            // '-' => Some(Ok(self.type_to_token(TokenType::Dash))),
            '-' => Some(self.parse_char_lookahead(TokenType::Dash, ('>', TokenType::Arrow))),
            '+' => Some(Ok(self.type_to_token(TokenType::Plus))),
            '*' => Some(Ok(self.type_to_token(TokenType::Asterisk))),
            '/' => Some(Ok(self.type_to_token(TokenType::Slash))),
            '%' => Some(Ok(self.type_to_token(TokenType::Perc))),

            // <, >, =, or ! can be interpreted as <=, >=, ==, or != (separate tokens!!!)
            '<' => Some(self.parse_char_lookahead(TokenType::LAngle, ('=', TokenType::LTEqual))),
            '>' => Some(self.parse_char_lookahead(TokenType::RAngle, ('=', TokenType::GTEqual))),
            '=' => Some(self.parse_char_lookahead(TokenType::Assign, ('=', TokenType::Equal))),
            '!' => Some(self.parse_char_lookahead(TokenType::Bang, ('=', TokenType::NotEqual))),
            '|' => Some(self.parse_char_lookahead(TokenType::Bar, ('|', TokenType::Or))),
            '&' => Some(self.parse_char_lookahead(TokenType::Ampersand, ('&', TokenType::And))),

            '\0' => None,
            _ => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_simple_line() {
        let input = r#"foo bar "baazz"  1.0"#;

        let expected = [
            Token {
                token_type: TokenType::Ident("foo".to_string()),
                range: Range::new((0, 0), (0, 2)),
            },
            Token {
                token_type: TokenType::Ident("bar".to_string()),
                range: Range::new((0, 4), (0, 6)),
            },
            Token {
                token_type: TokenType::String("baazz".to_string()),
                range: Range::new((0, 8), (0, 14)),
            },
            Token {
                token_type: TokenType::Number(1.0),
                range: Range::new((0, 17), (0, 19)),
            },
        ];

        let actual = Lexer::new(input);

        expected
            .iter()
            .zip(actual)
            .for_each(|(expected, actual)| assert_eq!(dbg!(actual).unwrap(), *expected));
    }

    #[test]
    pub fn test_strings() {
        let input = r#""Hi im \\   \n\ta \"string\"

whooo
ho""#;

        let mut actual = Lexer::new(input);

        assert_eq!(
            actual.next().unwrap().unwrap(),
            Token {
                token_type: TokenType::String(
                    "Hi im \\   \n\ta \"string\"

whooo
ho"
                    .to_string()
                ),
                range: Range::new((0, 0), (3, 2)),
            },
        )
    }

    #[test]
    pub fn test_multiple_lines() {
        let input = "  id_1ent
=

this



  \" ois\"
100 grob
\"cool\" \
            ";

        let expected = [
            Token {
                token_type: TokenType::Ident("id_1ent".to_string()),
                range: Range::new((0, 2), (0, 8)),
            },
            Token {
                token_type: TokenType::Assign,
                range: Range::new((1, 0), (1, 0)),
            },
            Token {
                token_type: TokenType::Ident("this".to_string()),
                range: Range::new((3, 0), (3, 3)),
            },
            Token {
                token_type: TokenType::String(" ois".to_string()),
                range: Range::new((7, 2), (7, 7)),
            },
            Token {
                token_type: TokenType::Number(100.0),
                range: Range::new((8, 0), (8, 2)),
            },
            Token {
                token_type: TokenType::Ident("grob".to_string()),
                range: Range::new((8, 4), (8, 7)),
            },
            Token {
                token_type: TokenType::String("cool".to_string()),
                range: Range::new((9, 0), (9, 5)),
            },
        ];

        let actual = Lexer::new(input);

        expected
            .iter()
            .zip(actual)
            .for_each(|(expected, actual)| assert_eq!(dbg!(actual).unwrap(), *expected));
    }

    #[test]
    pub fn test_actual_code() {
        let input = "\
func test(number x, Player p) {
    p.health = x;
    wait(10);
    x = x - 10;
    p.health = x;
    if p.health >= 10 || p.health < 0{

    }
}
";

        use crate::types::Keyword::*;
        use TokenType::*;
        let expected = [
            // line 1
            Keyword(Func),
            Ident("test".to_string()),
            LParen,
            Ident("number".to_string()),
            Ident("x".to_string()),
            Comma,
            Ident("Player".to_string()),
            Ident("p".to_string()),
            RParen,
            LBrace,
            // line 2
            Ident("p".to_string()),
            Dot,
            Ident("health".to_string()),
            Assign,
            Ident("x".to_string()),
            Semicolon,
            // line 3
            Ident("wait".to_string()),
            LParen,
            Number(10.0),
            RParen,
            Semicolon,
            // line 4
            Ident("x".to_string()),
            Assign,
            Ident("x".to_string()),
            Dash,
            Number(10.0),
            Semicolon,
            // line 5
            Ident("p".to_string()),
            Dot,
            Ident("health".to_string()),
            Assign,
            Ident("x".to_string()),
            Semicolon,
            // line 6
            Keyword(If),
            Ident("p".to_string()),
            Dot,
            Ident("health".to_string()),
            GTEqual,
            Number(10.0),
            Or,
            Ident("p".to_string()),
            Dot,
            Ident("health".to_string()),
            LAngle,
            Number(0.0),
            LBrace,
            // line 8
            RBrace,
            // line 9
            RBrace,
        ];

        let actual = Lexer::new(input);
        expected
            .iter()
            .zip(actual.map(|v| v.unwrap().token_type))
            .for_each(|(exp, act)| assert_eq!(exp, dbg!(&act)));
    }
}
