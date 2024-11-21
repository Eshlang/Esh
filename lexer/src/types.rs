use std::fmt::Display;

/// A position inside the code
#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    pub line: usize,
    pub char: usize,
}

impl Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.char)
    }
}

/// A range inside the code.
///
/// - If [Self::end] is [None], then the range only spans [Self::start].
/// - If [Self::end] is [Some], then the range spans from [Self::start] to [Self::end]
#[derive(Clone, Debug, PartialEq)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    pub fn new(start: (usize, usize), end: (usize, usize)) -> Self {
        Self {
            start: Position {
                line: start.0,
                char: start.1,
            },
            end: Position {
                line: end.0,
                char: end.1,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TokenType {
    Ident(String),
    String(String),
    Number(f64),
    // Comments probably don't need to contain what's in the comment, but I'll leave this for now
    Comment(String),

    Assign,   // =
    Equal,    // ==
    NotEqual, // !=
    LTEqual,  // <=
    GTEqual,  // >=
    Or,       // ||
    And,      // &&
    Arrow,    // ->

    Plus,      // +
    Dash,      // -
    Asterisk,  // *
    Slash,     // /
    Perc,      // %
    Bang,      // !
    Comma,     // ,
    Dot,       // .
    Semicolon, // ;
    Bar,       // |
    Ampersand, // &
    Colon,     // :

    LBrace,   // {
    RBrace,   // }
    LParen,   // (
    RParen,   // )
    LAngle,   // <
    RAngle,   // >
    LBracket, // [
    RBracket, // ]

    Keyword(Keyword),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Keyword {
    Func,   // functions
    If,     // ifs
    Else,   // else
    Return, // return
    Break,  // break
    True,   // true (boolean)
    False,  // false (boolean)
    Struct, // struct definition
    For,    // for loop
    While,    // while loop
    Domain, // domain definition
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub token_type: TokenType,
    pub range: Range,
}
