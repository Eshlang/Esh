pub enum TokenType {
    Ident(String),
    String(String),
    Number(f32),
    // Comments probably don't need to contain what's in the comment, but I'll leave this for now
    Comment(String),

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

pub struct Token {
    pub token_type: TokenType,
}
