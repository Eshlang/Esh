pub enum Token {
    Ident(String),
    // String type, but in df it's called txt
    Txt(String),
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

    // Keywords
    Func,
    If,
    Else,
    Return,
    True,
    False,
    Struct,
}

#[cfg(test)]
mod tests {}
