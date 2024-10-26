pub enum TokenType {
    Ident(String),
    // String type, but in df it's called txt
    String(String),
    Number(f32),
    // Comments probably don't need to contain what's in the comment, but I'll leave this for now
    Comment(String),

    EOF,
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
    Func,
    If,
    Else,
    Return,
    True,
    False,
    Struct,
}

pub struct Token {
    pub token_type: TokenType,
}

#[cfg(test)]
mod tests {}
