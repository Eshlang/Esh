use std::fmt::Display;

/// A position inside the code
#[derive(Debug)]
pub struct Position {
    pub line: usize,
    pub character: usize,
}

impl Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.character)
    }
}

/// A range inside the code.
///
/// - If [Self::end] is [None], then the range only spans [Self::start].
/// - If [Self::end] is [Some], then the range spans from [Self::start] to [Self::end]
#[derive(Debug)]
pub struct Range {
    pub start: Position,
    pub end: Option<Position>,
}

pub enum TokenType<'a> {
    Ident(&'a str),
    String(&'a str),
    Number(f32),
    // Comments probably don't need to contain what's in the comment, but I'll leave this for now
    Comment(&'a str),

    Assign,       // =
    Equal,        // ==
    NotEqual,     // !=
    LessThan,     // <=
    GreaterThan,  // >=
    Plus,         // +
    Dash,         // -
    Asterisk,     // *
    ForwardSlash, // /
    Exclamation,  // !
    Comma,        // ,
    Dot,          // .

    LCurly, // {
    RCurly, // }
    LParen, // (
    RParen, // )
    LAngle, // <
    RAngle, // >
    LBrace, // [
    RBrace, // ]

    Keyword(Keyword),
}

pub enum Keyword {
    Func,   // functions
    If,     // ifs
    Else,   // else
    Return, // return
    True,   // true (boolean)
    False,  // false (boolean)
    Struct, // struct definition
    For,    // for loop
}

pub struct Token<'a> {
    pub token_type: TokenType<'a>,
}
