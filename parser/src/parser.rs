use lexer::{
    lexer::Lexer, types::Token
};

pub enum Node<'a> {
    Tree(Tree<'a>),
    Token(Token<'a>),
}

pub struct Tree<'a>{
    children: Vec<Node<'a>>,
}

pub fn parse(lexer: &Lexer) -> Tree {
    todo!()
}