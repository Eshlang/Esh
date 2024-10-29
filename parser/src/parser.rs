use lexer::{types::Token, Lexer};

pub enum Node {
    Tree(Tree),
    Token(Token),
}

pub struct Tree {
    children: Vec<Node>,
}

pub fn parse(lexer: &Lexer) -> Tree {
    todo!()
}

