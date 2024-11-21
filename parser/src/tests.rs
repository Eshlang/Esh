use lexer::types::Range;
use crate::parser::*;
use std::rc::Rc;
use lexer::types::{Keyword, Token, TokenType};

#[test]
pub fn expression_test() {
    // x + 8 / 2 * 4
    let input = [
        Rc::new(Token {
            token_type: TokenType::Ident("x".to_string()),
            range: Range::new((0, 0), (0, 0)),
        }),
        Rc::new(Token {
            token_type: TokenType::Plus,
            range: Range::new((0, 2), (0, 2)),
        }),
        Rc::new(Token {
            token_type: TokenType::Number(8f64),
            range: Range::new((0, 4), (0, 4)),
        }),
        Rc::new(Token {
            token_type: TokenType::Slash,
            range: Range::new((0, 6), (0, 6)),
        }),
        Rc::new(Token {
            token_type: TokenType::Number(2f64),
            range: Range::new((0, 8), (0, 8)),
        }),
        Rc::new(Token {
            token_type: TokenType::Asterisk,
            range: Range::new((0, 10), (0, 10)),
        }),
        Rc::new(Token {
            token_type: TokenType::Number(4f64),
            range: Range::new((0, 12), (0, 12)),
        }),
    ];
    let expected = Node::Sum(
        Rc::new(Node::Primary(Rc::new(Token {
            token_type: TokenType::Ident("x".to_string()),
            range: Range::new((0, 0), (0, 0)),
        }))),
        Rc::new(Node::Product(
            Rc::new(Node::Quotient(
                Rc::new(Node::Primary(Rc::new(Token {
                        token_type: TokenType::Number(8f64),
                        range: Range::new((0, 4), (0, 4)),
                }))),
                Rc::new(Node::Primary(Rc::new(Token {
                    token_type: TokenType::Number(2f64),
                    range: Range::new((0, 8), (0, 8)),
                }))),
            )),
            Rc::new(Node::Primary(Rc::new(Token {
                    token_type: TokenType::Number(4f64),
                    range: Range::new((0, 12), (0, 12)),
            }))),
        )),
    );
    let mut parser = Parser::new(&input);
    match parser.expression() {
        Ok(output) => assert_eq!(expected, output),
        Err(e) => {
            dbg!(e);
            panic!()
        }
    }
}

#[test]
pub fn parenthesis_test() {
    // (x + 8) / (2 * 4)
    let input = [
        Rc::new(Token {
            token_type: TokenType::LParen,
            range: Range::new((0, 0), (0, 0)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("x".to_string()),
            range: Range::new((0, 1), (0, 1)),
        }),
        Rc::new(Token {
            token_type: TokenType::Plus,
            range: Range::new((0, 3), (0, 3)),
        }),
        Rc::new(Token {
            token_type: TokenType::Number(8f64),
            range: Range::new((0, 5), (0, 5)),
        }),
        Rc::new(Token {
            token_type: TokenType::RParen,
            range: Range::new((0, 6), (0, 6)),
        }),
        Rc::new(Token {
            token_type: TokenType::Slash,
            range: Range::new((0, 8), (0, 8)),
        }),
        Rc::new(Token {
            token_type: TokenType::LParen,
            range: Range::new((0, 10), (0, 10)),
        }),
        Rc::new(Token {
            token_type: TokenType::Number(2f64),
            range: Range::new((0, 11), (0, 11)),
        }),
        Rc::new(Token {
            token_type: TokenType::Asterisk,
            range: Range::new((0, 13), (0, 13)),
        }),
        Rc::new(Token {
            token_type: TokenType::Number(4f64),
            range: Range::new((0, 15), (0, 15)),
        }),
        Rc::new(Token {
            token_type: TokenType::RParen,
            range: Range::new((0, 16), (0, 16)),
        }),
    ];
    let expected = Node::Quotient(
        Rc::new(Node::Sum(
            Rc::new(Node::Primary(Rc::new(Token {
                token_type: TokenType::Ident("x".to_string()),
                range: Range::new((0, 1), (0, 1)),
            }))),
            Rc::new(Node::Primary(Rc::new(Token {
                token_type: TokenType::Number(8f64),
                range: Range::new((0, 5), (0, 5)),
            }))),
        )),
        Rc::new(Node::Product(
            Rc::new(Node::Primary(Rc::new(Token {
                token_type: TokenType::Number(2f64),
                range: Range::new((0, 11), (0, 11)),
            }))),
            Rc::new(Node::Primary(Rc::new(Token {
                token_type: TokenType::Number(4f64),
                range: Range::new((0, 15), (0, 15)),
            }))),
        ))
    );
    let mut parser = Parser::new(&input);
    match parser.expression() {
        Ok(output) => assert_eq!(expected, output),
        Err(e) => {
            dbg!(e);
            panic!()
        }
    }
}

#[test]
pub fn tuple_test() {
    // (x, 3, "test")
    let input = [
        Rc::new(Token {
            token_type: TokenType::LParen,
            range: Range::new((0, 0), (0, 0)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("x".to_string()),
            range: Range::new((0, 1), (0, 1)),
        }),
        Rc::new(Token {
            token_type: TokenType::Comma,
            range: Range::new((0, 2), (0, 2)),
        }),
        Rc::new(Token {
            token_type: TokenType::Number(3f64),
            range: Range::new((0, 4), (0, 4)),
        }),
        Rc::new(Token {
            token_type: TokenType::Comma,
            range: Range::new((0, 5), (0, 5)),
        }),
        Rc::new(Token {
            token_type: TokenType::String("test".to_string()),
            range: Range::new((0, 7), (0, 12)),
        }),
        Rc::new(Token {
            token_type: TokenType::RParen,
            range: Range::new((0, 13), (0, 13)),
        }),
    ];
    let expected = Node::Tuple(vec![
        Rc::new(Node::Primary(Rc::new(Token {
            token_type: TokenType::Ident("x".to_string()),
            range: Range::new((0, 1), (0, 1)),
        }))),
        Rc::new(Node::Primary(Rc::new(Token {
            token_type: TokenType::Number(3f64),
            range: Range::new((0, 4), (0, 4)),
        }))),
        Rc::new(Node::Primary(Rc::new(Token {
            token_type: TokenType::String("test".to_string()),
            range: Range::new((0, 7), (0, 12)),
        }))),
    ]);
    let mut parser = Parser::new(&input);
    match parser.expression() {
        Ok(output) => assert_eq!(expected, output),
        Err(e) => {
            dbg!(e);
            panic!()
        }
    }
}
#[test]
pub fn access_test() {
    // foo.bar().bat().pgoo
    let input = [
        Rc::new(Token {
            token_type: TokenType::Ident("foo".to_string()),
            range: Range::new((0, 0), (0, 2)),
        }),
        Rc::new(Token {
            token_type: TokenType::Dot,
            range: Range::new((0, 3), (0, 3)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("bar".to_string()),
            range: Range::new((0, 4), (0, 6)),
        }),
        Rc::new(Token {
            token_type: TokenType::LParen,
            range: Range::new((0, 7), (0, 7)),
        }),
        Rc::new(Token {
            token_type: TokenType::RParen,
            range: Range::new((0, 8), (0, 8)),
        }),
        Rc::new(Token {
            token_type: TokenType::Dot,
            range: Range::new((0, 9), (0, 9)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("bat".to_string()),
            range: Range::new((0, 10), (0, 12)),
        }),
        Rc::new(Token {
            token_type: TokenType::LParen,
            range: Range::new((0, 13), (0, 13)),
        }),
        Rc::new(Token {
            token_type: TokenType::RParen,
            range: Range::new((0, 14), (0, 14)),
        }),
        Rc::new(Token {
            token_type: TokenType::Dot,
            range: Range::new((0, 15), (0, 15)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("pgoo".to_string()),
            range: Range::new((0, 16), (0, 19)),
        }),
    ];
    let expected = Node::Access(vec![
        Rc::new(Node::FunctionCall(
            Rc::new(Node::Access(vec![
                Rc::new(Node::FunctionCall(
                    Rc::new(Node::Access(vec![
                        Rc::new(Node::Primary(Rc::new(Token {
                            token_type: TokenType::Ident("foo".to_string()),
                            range: Range::new((0, 0), (0, 2)),
                        }))),
                        Rc::new(Node::Primary(Rc::new(Token {
                            token_type: TokenType::Ident("bar".to_string()),
                            range: Range::new((0, 4), (0, 6)),
                        }))),
                    ])),
                    Rc::new(Node::Tuple(vec![])),
                )),
                Rc::new(Node::Primary(Rc::new(Token {
                    token_type: TokenType::Ident("bat".to_string()),
                    range: Range::new((0, 10), (0, 12)),
                }))),
            ])),
            Rc::new(Node::Tuple(vec![])),
        )),
        Rc::new(Node::Primary(Rc::new(Token {
            token_type: TokenType::Ident("pgoo".to_string()),
            range: Range::new((0, 16), (0, 19)),
        }))),
    ]);
    let mut parser = Parser::new(&input);
    match parser.access() {
        Ok(output) => assert_eq!(expected, output),
        Err(e) => {
            dbg!(e);
            panic!()
        }
    }
}

#[test]
pub fn assignment_test() {
    // num x = 5;
    let input = [
        Rc::new(Token {
            token_type: TokenType::Ident("num".to_string()),
            range: Range::new((0, 0), (0, 2)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("x".to_string()),
            range: Range::new((0, 4), (0, 4)),
        }),
        Rc::new(Token {
            token_type: TokenType::Assign,
            range: Range::new((0, 6), (0, 6)),
        }),
        Rc::new(Token {
            token_type: TokenType::Number(5f64),
            range: Range::new((0, 8), (0, 8)),
        }),
        Rc::new(Token {
            token_type: TokenType::Semicolon,
            range: Range::new((0, 9), (0, 9)),
        }),
    ];
    let expected = Node::Assignment(
        Rc::new(Node::Declaration(
            Rc::new(Node::Primary(Rc::new(Token {
                token_type: TokenType::Ident("num".to_string()),
                range: Range::new((0, 0), (0, 2)),
            }))),
            Rc::new(Node::Primary(Rc::new(Token {
                token_type: TokenType::Ident("x".to_string()),
                range: Range::new((0, 4), (0, 4)),
            }))),
        )),
        Rc::new(Node::Primary(Rc::new(Token {
            token_type: TokenType::Number(5f64),
            range: Range::new((0, 8), (0, 8)),
        }))),
    );
    let mut parser = Parser::new(&input);
    match parser.statement() {
        Ok(output) => assert_eq!(expected, output),
        Err(e) => {
            dbg!(e);
            panic!()
        }
    }
}

#[test]
pub fn statement_block_test() {
    // num x;
    // str y = "hello";
    let input = [
        Rc::new(Token {
            token_type: TokenType::Ident("num".to_string()),
            range: Range::new((0, 0), (0, 2)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("x".to_string()),
            range: Range::new((0, 4), (0, 4)),
        }),
        Rc::new(Token {
            token_type: TokenType::Semicolon,
            range: Range::new((0, 5), (0, 5)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("str".to_string()),
            range: Range::new((1, 0), (1, 2)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("y".to_string()),
            range: Range::new((1, 4), (1, 4)),
        }),
        Rc::new(Token {
            token_type: TokenType::Assign,
            range: Range::new((1, 6), (1, 6)),
        }),
        Rc::new(Token {
            token_type: TokenType::String("hello".to_string()),
            range: Range::new((1, 8), (1, 14)),
        }),
        Rc::new(Token {
            token_type: TokenType::Semicolon,
            range: Range::new((1, 15), (1, 15)),
        }),
    ];
    let expected = Node::Block(vec![
        Rc::new(Node::Declaration(
            Rc::new(Node::Primary(Rc::new(Token {
                token_type: TokenType::Ident("num".to_string()),
                range: Range::new((0, 0), (0, 2)),
            }))),
            Rc::new(Node::Primary(Rc::new(Token {
                    token_type: TokenType::Ident("x".to_string()),
                    range: Range::new((0, 4), (0, 4)),
            }))),
        )),
        Rc::new(Node::Assignment(
            Rc::new(Node::Declaration(
                Rc::new(Node::Primary(Rc::new(Token {
                    token_type: TokenType::Ident("str".to_string()),
                    range: Range::new((1, 0), (1, 2)),
                }))),
                Rc::new(Node::Primary(Rc::new(Token {
                    token_type: TokenType::Ident("y".to_string()),
                    range: Range::new((1, 4), (1, 4)),
                }))),
            )),
            Rc::new(Node::Primary(Rc::new(Token {
                token_type: TokenType::String("hello".to_string()),
                range: Range::new((1, 8), (1, 14)),
            }))),
        )),
    ]);
    let mut parser = Parser::new(&input);
    match parser.statement_block() {
        Ok(output) => assert_eq!(expected, output),
        Err(e) => {
            dbg!(e);
            panic!()
        }
    }
}

#[test]
pub fn while_test() {
    // while x == 5 && true != false {
    //    str y = "hello";
    // }
    let input = [
        Rc::new(Token {
            token_type: TokenType::Keyword(Keyword::While),
            range: Range::new((0, 0), (0, 4)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("x".to_string()),
            range: Range::new((0, 6), (0, 6)),
        }),
        Rc::new(Token {
            token_type: TokenType::Equal,
            range: Range::new((0, 8), (0, 9)),
        }),
        Rc::new(Token {
            token_type: TokenType::Number(5f64),
            range: Range::new((0, 11), (0, 11)),
        }),
        Rc::new(Token {
            token_type: TokenType::And,
            range: Range::new((0, 13), (0, 14)),
        }),
        Rc::new(Token {
            token_type: TokenType::Keyword(Keyword::True),
            range: Range::new((0, 16), (0, 19)),
        }),
        Rc::new(Token {
            token_type: TokenType::NotEqual,
            range: Range::new((0, 21), (0, 22)),
        }),
        Rc::new(Token {
            token_type: TokenType::Keyword(Keyword::False),
            range: Range::new((0, 24), (0, 28)),
        }),
        Rc::new(Token {
            token_type: TokenType::LBrace,
            range: Range::new((0, 30), (0, 30)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("str".to_string()),
            range: Range::new((1, 0), (1, 2)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("y".to_string()),
            range: Range::new((1, 4), (1, 4)),
        }),
        Rc::new(Token {
            token_type: TokenType::Assign,
            range: Range::new((1, 6), (1, 6)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("hello".to_string()),
            range: Range::new((1, 8), (1, 14)),
        }),
        Rc::new(Token {
            token_type: TokenType::Semicolon,
            range: Range::new((1, 15), (1, 15)),
        }),
        Rc::new(Token {
            token_type: TokenType::RBrace,
            range: Range::new((2, 0), (2, 0)),
        }),
    ];
    let expected = Node::While(
        Rc::new(Node::And(
            Rc::new(Node::Equal(
                Rc::new(Node::Primary(Rc::new(Token {
                    token_type: TokenType::Ident("x".to_string()),
                    range: Range::new((0, 6), (0, 6)),
                }))),
                Rc::new(Node::Primary(Rc::new(Token {
                    token_type: TokenType::Number(5f64),
                    range: Range::new((0, 11), (0, 11)),
                }))),
            )),
            Rc::new(Node::NotEqual(
                Rc::new(Node::Primary(Rc::new(Token {
                    token_type: TokenType::Keyword(Keyword::True),
                    range: Range::new((0, 16), (0, 19)),
                }))),
                Rc::new(Node::Primary(Rc::new(Token {
                    token_type: TokenType::Keyword(Keyword::False),
                    range: Range::new((0, 24), (0, 28)),
                }))),
            )),
        )),
        Rc::new(Node::Block(vec![
            Rc::new(Node::Assignment(
                Rc::new(Node::Declaration(
                    Rc::new(Node::Primary(Rc::new(Token {
                        token_type: TokenType::Ident("str".to_string()),
                        range: Range::new((1, 0), (1, 2)),
                    }))),
                    Rc::new(Node::Primary(Rc::new(Token {
                        token_type: TokenType::Ident("y".to_string()),
                        range: Range::new((1, 4), (1, 4)),
                    }))),
                )),
                Rc::new(Node::Primary(Rc::new(Token {
                    token_type: TokenType::Ident("hello".to_string()),
                    range: Range::new((1, 8), (1, 14)),
                }))),
            )),
        ])),
    );
    let mut parser = Parser::new(&input);
    match parser.statement() {
        Ok(output) => assert_eq!(expected, output),
        Err(e) => {
            dbg!(e);
            panic!()
        }
    }
}

#[test]
pub fn if_statement_test() {
    // if x == 5 && true != false {
    //    str y = "hello";
    // }
    let input = [
        Rc::new(Token {
            token_type: TokenType::Keyword(Keyword::If),
            range: Range::new((0, 0), (0, 1)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("x".to_string()),
            range: Range::new((0, 3), (0, 3)),
        }),
        Rc::new(Token {
            token_type: TokenType::Equal,
            range: Range::new((0, 5), (0, 6)),
        }),
        Rc::new(Token {
            token_type: TokenType::Number(5f64),
            range: Range::new((0, 8), (0, 8)),
        }),
        Rc::new(Token {
            token_type: TokenType::And,
            range: Range::new((0, 10), (0, 11)),
        }),
        Rc::new(Token {
            token_type: TokenType::Keyword(Keyword::True),
            range: Range::new((0, 13), (0, 16)),
        }),
        Rc::new(Token {
            token_type: TokenType::NotEqual,
            range: Range::new((0, 18), (0, 19)),
        }),
        Rc::new(Token {
            token_type: TokenType::Keyword(Keyword::False),
            range: Range::new((0, 21), (0, 25)),
        }),
        Rc::new(Token {
            token_type: TokenType::LBrace,
            range: Range::new((0, 27), (0, 27)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("str".to_string()),
            range: Range::new((1, 0), (1, 2)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("y".to_string()),
            range: Range::new((1, 4), (1, 4)),
        }),
        Rc::new(Token {
            token_type: TokenType::Assign,
            range: Range::new((1, 6), (1, 6)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("hello".to_string()),
            range: Range::new((1, 8), (1, 14)),
        }),
        Rc::new(Token {
            token_type: TokenType::Semicolon,
            range: Range::new((1, 15), (1, 15)),
        }),
        Rc::new(Token {
            token_type: TokenType::RBrace,
            range: Range::new((2, 0), (2, 0)),
        }),
    ];
    let expected = Node::If(
        Rc::new(Node::And(
            Rc::new(Node::Equal(
                Rc::new(Node::Primary(Rc::new(Token {
                    token_type: TokenType::Ident("x".to_string()),
                    range: Range::new((0, 3), (0, 3)),
                }))),
                Rc::new(Node::Primary(Rc::new(Token {
                    token_type: TokenType::Number(5f64),
                    range: Range::new((0, 8), (0, 8)),
                }))),
            )),
            Rc::new(Node::NotEqual(
                Rc::new(Node::Primary(Rc::new(Token {
                    token_type: TokenType::Keyword(Keyword::True),
                    range: Range::new((0, 13), (0, 16)),
                }))),
                Rc::new(Node::Primary(Rc::new(Token {
                    token_type: TokenType::Keyword(Keyword::False),
                    range: Range::new((0, 21), (0, 25)),
                }))),
            )),
        )),
        Rc::new(Node::Block(vec![
            Rc::new(Node::Assignment(
                Rc::new(Node::Declaration(
                    Rc::new(Node::Primary(Rc::new(Token {
                        token_type: TokenType::Ident("str".to_string()),
                        range: Range::new((1, 0), (1, 2)),
                    }))),
                    Rc::new(Node::Primary(Rc::new(Token {
                        token_type: TokenType::Ident("y".to_string()),
                        range: Range::new((1, 4), (1, 4)),
                    }))),
                )),
                Rc::new(Node::Primary(Rc::new(Token {
                    token_type: TokenType::Ident("hello".to_string()),
                    range: Range::new((1, 8), (1, 14)),
                }))),
            )),
        ])),
    );
    let mut parser = Parser::new(&input);
    match parser.statement() {
        Ok(output) => assert_eq!(expected, output),
        Err(e) => {
            dbg!(e);
            panic!()
        }
    }
}

#[test]
pub fn if_else_test() {
    // if x == 5 {
    //    str y = "hello";
    // } else {
    //    str y = "evil hello";
    // }
    let input = [
        Rc::new(Token {
            token_type: TokenType::Keyword(Keyword::If),
            range: Range::new((0, 0), (0, 1)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("x".to_string()),
            range: Range::new((0, 3), (0, 3)),
        }),
        Rc::new(Token {
            token_type: TokenType::Equal,
            range: Range::new((0, 5), (0, 6)),
        }),
        Rc::new(Token {
            token_type: TokenType::Number(5f64),
            range: Range::new((0, 8), (0, 8)),
        }),
        Rc::new(Token {
            token_type: TokenType::LBrace,
            range: Range::new((0, 10), (0, 10)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("str".to_string()),
            range: Range::new((1, 0), (1, 2)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("y".to_string()),
            range: Range::new((1, 4), (1, 4)),
        }),
        Rc::new(Token {
            token_type: TokenType::Assign,
            range: Range::new((1, 6), (1, 6)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("hello".to_string()),
            range: Range::new((1, 8), (1, 14)),
        }),
        Rc::new(Token {
            token_type: TokenType::Semicolon,
            range: Range::new((1, 15), (1, 15)),
        }),
        Rc::new(Token {
            token_type: TokenType::RBrace,
            range: Range::new((2, 0), (2, 0)),
        }),
        Rc::new(Token {
            token_type: TokenType::Keyword(Keyword::Else),
            range: Range::new((2, 2), (2, 5)),
        }),
        Rc::new(Token {
            token_type: TokenType::LBrace,
            range: Range::new((2, 7), (2, 7)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("str".to_string()),
            range: Range::new((3, 0), (3, 2)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("y".to_string()),
            range: Range::new((3, 4), (3, 4)),
        }),
        Rc::new(Token {
            token_type: TokenType::Assign,
            range: Range::new((3, 6), (3, 6)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("evil hello".to_string()),
            range: Range::new((3, 8), (3, 19)),
        }),
        Rc::new(Token {
            token_type: TokenType::Semicolon,
            range: Range::new((3, 20), (3, 20)),
        }),
        Rc::new(Token {
            token_type: TokenType::RBrace,
            range: Range::new((4, 0), (4, 0)),
        }),
    ];
    let expected = Node::Else(
        Rc::new(Node::If(
            Rc::new(Node::Equal(
                Rc::new(Node::Primary(Rc::new(Token {
                    token_type: TokenType::Ident("x".to_string()),
                    range: Range::new((0, 3), (0, 3)),
                }))),
                Rc::new(Node::Primary(Rc::new(Token {
                    token_type: TokenType::Number(5f64),
                    range: Range::new((0, 8), (0, 8)),
                }))),
            )),
            Rc::new(Node::Block(vec![
                Rc::new(Node::Assignment(
                    Rc::new(Node::Declaration(
                        Rc::new(Node::Primary(Rc::new(Token {
                            token_type: TokenType::Ident("str".to_string()),
                            range: Range::new((1, 0), (1, 2)),
                        }))),
                        Rc::new(Node::Primary(Rc::new(Token {
                            token_type: TokenType::Ident("y".to_string()),
                            range: Range::new((1, 4), (1, 4)),
                        }))),
                    )),
                    Rc::new(Node::Primary(Rc::new(Token {
                        token_type: TokenType::Ident("hello".to_string()),
                        range: Range::new((1, 8), (1, 14)),
                    }))),
                )),
            ])),
        )),
        Rc::new(Node::Block(vec![
            Rc::new(Node::Assignment(
                Rc::new(Node::Declaration(
                    Rc::new(Node::Primary(Rc::new(Token {
                        token_type: TokenType::Ident("str".to_string()),
                        range: Range::new((3, 0), (3, 2)),
                    }))), 
                    Rc::new(Node::Primary(Rc::new(Token {
                        token_type: TokenType::Ident("y".to_string()),
                        range: Range::new((3, 4), (3, 4)),
                    }))),
                )), 
                Rc::new(Node::Primary(Rc::new(Token {
                    token_type: TokenType::Ident("evil hello".to_string()),
                    range: Range::new((3, 8), (3, 19)),
                }))),
            ))
        ])),
    );
    let mut parser = Parser::new(&input);
    match parser.statement() {
        Ok(output) => assert_eq!(expected, output),
        Err(e) => {
            dbg!(e);
            panic!()
        }
    }
}

#[test]
pub fn no_return_function_test() {
    // func foo(num x) {
    //    bar(x);
    //    return;
    // }
    let input = [
        Rc::new(Token {
            token_type: TokenType::Keyword(Keyword::Func),
            range: Range::new((0, 0), (0, 3)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("foo".to_string()),
            range: Range::new((0, 5), (0, 7)),
        }),
        Rc::new(Token {
            token_type: TokenType::LParen,
            range: Range::new((0, 8), (0, 8)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("num".to_string()),
            range: Range::new((0, 9), (0, 11)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("x".to_string()),
            range: Range::new((0, 13), (0, 13)),
        }),
        Rc::new(Token {
            token_type: TokenType::RParen,
            range: Range::new((0, 14), (0, 14)),
        }),
        Rc::new(Token {
            token_type: TokenType::LBrace,
            range: Range::new((0, 16), (0, 16)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("bar".to_string()),
            range: Range::new((1, 0), (1, 2)),
        }),
        Rc::new(Token {
            token_type: TokenType::LParen,
            range: Range::new((1, 3), (1, 3)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("x".to_string()),
            range: Range::new((1, 4), (1, 4)),
        }),
        Rc::new(Token {
            token_type: TokenType::RParen,
            range: Range::new((1, 5), (1, 5)),
        }),
        Rc::new(Token {
            token_type: TokenType::Semicolon,
            range: Range::new((1, 6), (1, 6)),
        }),
        Rc::new(Token {
            token_type: TokenType::Keyword(Keyword::Return),
            range: Range::new((2, 0), (2, 5)),
        }),
        Rc::new(Token {
            token_type: TokenType::Semicolon,
            range: Range::new((2, 6), (2, 6)),
        }),
        Rc::new(Token {
            token_type: TokenType::RBrace,
            range: Range::new((3, 0), (3, 0)),
        }),
    ];
    let expected = Node::Func(
        Rc::new(Node::Primary(Rc::new(Token {
            token_type: TokenType::Ident("foo".to_string()),
            range: Range::new((0, 5), (0, 7)),
        }))),
        Rc::new(Node::Tuple(vec![
            Rc::new(Node::Declaration(
                Rc::new(Node::Primary(Rc::new(Token {
                    token_type: TokenType::Ident("num".to_string()),
                    range: Range::new((0, 9), (0, 11)),
                }))),
                Rc::new(Node::Primary(Rc::new(Token {
                    token_type: TokenType::Ident("x".to_string()),
                    range: Range::new((0, 13), (0, 13)),
                }))),
            )),
        ])),
        Rc::new(Node::None),
        Rc::new(Node::Block(vec![
            Rc::new(Node::FunctionCall(
                Rc::new(Node::Primary(Rc::new(Token {
                    token_type: TokenType::Ident("bar".to_string()),
                    range: Range::new((1, 0), (1, 2)),
                }))),
                Rc::new(Node::Tuple(vec![
                    Rc::new(Node::Primary(Rc::new(Token {
                        token_type: TokenType::Ident("x".to_string()),
                        range: Range::new((1, 4), (1, 4)),
                    }))),
                ])),
            )),
            Rc::new(Node::Return(Rc::new(Node::None))),
        ])),
    );
    let mut parser = Parser::new(&input);
    match parser.statement() {
        Ok(output) => assert_eq!(expected, output),
        Err(e) => {
            dbg!(e);
            panic!()
        }
    }
}

#[test]
pub fn return_function_test() {
    // func foo(num x) -> num {
    //    return x * bar(2);
    // }
    let input = [
        Rc::new(Token {
            token_type: TokenType::Keyword(Keyword::Func),
            range: Range::new((0, 0), (0, 3)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("foo".to_string()),
            range: Range::new((0, 5), (0, 7)),
        }),
        Rc::new(Token {
            token_type: TokenType::LParen,
            range: Range::new((0, 8), (0, 8)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("num".to_string()),
            range: Range::new((0, 9), (0, 11)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("x".to_string()),
            range: Range::new((0, 13), (0, 13)),
        }),
        Rc::new(Token {
            token_type: TokenType::RParen,
            range: Range::new((0, 14), (0, 14)),
        }),
        Rc::new(Token {
            token_type: TokenType::Arrow,
            range: Range::new((0, 14), (0, 14)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("num".to_string()),
            range: Range::new((0, 16), (0, 18)),
        }),
        Rc::new(Token {
            token_type: TokenType::LBrace,
            range: Range::new((0, 20), (0, 20)),
        }),
        Rc::new(Token {
            token_type: TokenType::Keyword(Keyword::Return),
            range: Range::new((1, 0), (1, 5)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("x".to_string()),
            range: Range::new((1, 7), (1, 7)),
        }),
        Rc::new(Token {
            token_type: TokenType::Asterisk,
            range: Range::new((1, 9), (1, 9)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("bar".to_string()),
            range: Range::new((1, 11), (1, 13)),
        }),
        Rc::new(Token {
            token_type: TokenType::LParen,
            range: Range::new((1, 14), (1, 14)),
        }),
        Rc::new(Token {
            token_type: TokenType::Number(2f64),
            range: Range::new((1, 15), (1, 15)),
        }),
        Rc::new(Token {
            token_type: TokenType::RParen,
            range: Range::new((1, 16), (1, 16)),
        }),
        Rc::new(Token {
            token_type: TokenType::Semicolon,
            range: Range::new((1, 17), (1, 17)),
        }),
        Rc::new(Token {
            token_type: TokenType::RBrace,
            range: Range::new((2, 0), (2, 0)),
        }),
    ];
    let expected = Node::Func(
        Rc::new(Node::Primary(Rc::new(Token {
            token_type: TokenType::Ident("foo".to_string()),
            range: Range::new((0, 5), (0, 7)),
        }))),
        Rc::new(Node::Tuple(vec![
            Rc::new(Node::Declaration(
                Rc::new(Node::Primary(Rc::new(Token {
                    token_type: TokenType::Ident("num".to_string()),
                    range: Range::new((0, 9), (0, 11)),
                }))),
                Rc::new(Node::Primary(Rc::new(Token {
                    token_type: TokenType::Ident("x".to_string()),
                    range: Range::new((0, 13), (0, 13)),
                }))),
            ))
        ])),
        Rc::new(Node::Primary(Rc::new(Token {
            token_type: TokenType::Ident("num".to_string()),
            range: Range::new((0, 16), (0, 18)),
        }))),
        Rc::new(Node::Block(vec![
            Rc::new(Node::Return(
                Rc::new(Node::Product(
                    Rc::new(Node::Primary(Rc::new(Token {
                        token_type: TokenType::Ident("x".to_string()),
                        range: Range::new((1, 7), (1, 7)),
                    }))),
                    Rc::new(Node::FunctionCall(
                        Rc::new(Node::Primary(Rc::new(Token {
                            token_type: TokenType::Ident("bar".to_string()),
                            range: Range::new((1, 11), (1, 13)),
                        }))),
                        Rc::new(Node::Tuple(vec![
                            Rc::new(Node::Primary(Rc::new(Token {
                                token_type: TokenType::Number(2f64),
                                range: Range::new((1, 15), (1, 15)),
                            }))),
                        ])),
                    )),
                )),
            )),
        ])),
    );
    let mut parser = Parser::new(&input);
    match parser.statement() {
        Ok(output) => assert_eq!(expected, output),
        Err(e) => {
            dbg!(e);
            panic!()
        }
    }
}

#[test]
pub fn struct_test() {
    // struct foo {
    //    num x;
    //    str y;
    //    func bar() -> (num, str) {
    //       return (x, y);
    //    }
    // }
    // func main() {
    //    foo z = foo {
    //       x = 1;
    //       y = "a";
    //    };
    //    print(z.bar());
    // }
    
    let input = [
        Rc::new(Token {
            token_type: TokenType::Keyword(Keyword::Struct),
            range: Range::new((0, 0), (0, 5)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("foo".to_string()),
            range: Range::new((0, 7), (0, 9)),
        }),
        Rc::new(Token {
            token_type: TokenType::LBrace,
            range: Range::new((0, 11), (0, 11)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("num".to_string()),
            range: Range::new((1, 0), (1, 2)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("x".to_string()),
            range: Range::new((1, 4), (1, 4)),
        }),
        Rc::new(Token {
            token_type: TokenType::Semicolon,
            range: Range::new((1, 5), (1, 5)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("str".to_string()),
            range: Range::new((2, 0), (2, 2)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("y".to_string()),
            range: Range::new((2, 4), (2, 4)),
        }),
        Rc::new(Token {
            token_type: TokenType::Semicolon,
            range: Range::new((2, 5), (2, 5)),
        }),
        Rc::new(Token {
            token_type: TokenType::Keyword(Keyword::Func),
            range: Range::new((3, 0), (3, 3)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("bar".to_string()),
            range: Range::new((3, 5), (3, 7)),
        }),
        Rc::new(Token {
            token_type: TokenType::LParen,
            range: Range::new((3, 8), (3, 8)),
        }),
        Rc::new(Token {
            token_type: TokenType::RParen,
            range: Range::new((3, 9), (3, 9)),
        }),
        Rc::new(Token {
            token_type: TokenType::Arrow,
            range: Range::new((3, 11), (3, 12)),
        }),
        Rc::new(Token {
            token_type: TokenType::LParen,
            range: Range::new((3, 14), (3, 14)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("num".to_string()),
            range: Range::new((3, 15), (3, 17)),
        }),
        Rc::new(Token {
            token_type: TokenType::Comma,
            range: Range::new((3, 18), (3, 18)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("str".to_string()),
            range: Range::new((3, 19), (3, 21)),
        }),
        Rc::new(Token {
            token_type: TokenType::RParen,
            range: Range::new((3, 22), (3, 22)),
        }),
        Rc::new(Token {
            token_type: TokenType::LBrace,
            range: Range::new((3, 24), (3, 24)),
        }),
        Rc::new(Token {
            token_type: TokenType::Keyword(Keyword::Return),
            range: Range::new((4, 0), (4, 5)),
        }),
        Rc::new(Token {
            token_type: TokenType::LParen,
            range: Range::new((4, 7), (4, 7)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("x".to_string()),
            range: Range::new((4, 8), (4, 8)),
        }),
        Rc::new(Token {
            token_type: TokenType::Comma,
            range: Range::new((4, 9), (4, 9)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("y".to_string()),
            range: Range::new((4, 11), (4, 11)),
        }),
        Rc::new(Token {
            token_type: TokenType::RParen,
            range: Range::new((4, 12), (4, 12)),
        }),
        Rc::new(Token {
            token_type: TokenType::Semicolon,
            range: Range::new((4, 13), (4, 13)),
        }),
        Rc::new(Token {
            token_type: TokenType::RBrace,
            range: Range::new((5, 0), (5, 0)),
        }),
        Rc::new(Token {
            token_type: TokenType::RBrace,
            range: Range::new((6, 0), (6, 0)),
        }),
        Rc::new(Token {
            token_type: TokenType::Keyword(Keyword::Func),
            range: Range::new((7, 0), (7, 3)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("main".to_string()),
            range: Range::new((7, 5), (7, 8)),
        }),
        Rc::new(Token {
            token_type: TokenType::LParen,
            range: Range::new((7, 9), (7, 9)),
        }),
        Rc::new(Token {
            token_type: TokenType::RParen,
            range: Range::new((7, 10), (7, 10)),
        }),
        Rc::new(Token {
            token_type: TokenType::LBrace,
            range: Range::new((7, 12), (7, 12)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("foo".to_string()),
            range: Range::new((8, 0), (8, 2)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("z".to_string()),
            range: Range::new((8, 4), (8, 4)),
        }),
        Rc::new(Token {
            token_type: TokenType::Assign,
            range: Range::new((8, 6), (8, 6)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("foo".to_string()),
            range: Range::new((8, 8), (8, 10)),
        }),
        Rc::new(Token {
            token_type: TokenType::LBrace,
            range: Range::new((8, 12), (8, 12)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("x".to_string()),
            range: Range::new((9, 0), (9, 0)),
        }),
        Rc::new(Token {
            token_type: TokenType::Assign,
            range: Range::new((9, 2), (9, 2)),
        }),
        Rc::new(Token {
            token_type: TokenType::Number(1f64),
            range: Range::new((9, 4), (9, 4)),
        }),
        Rc::new(Token {
            token_type: TokenType::Semicolon,
            range: Range::new((9, 5), (9, 5)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("y".to_string()),
            range: Range::new((10, 0), (10, 0)),
        }),
        Rc::new(Token {
            token_type: TokenType::Assign,
            range: Range::new((10, 2), (10, 2)),
        }),
        Rc::new(Token {
            token_type: TokenType::String("a".to_string()),
            range: Range::new((10, 4), (10, 4)),
        }),
        Rc::new(Token {
            token_type: TokenType::Semicolon,
            range: Range::new((10, 5), (10, 5)),
        }),
        Rc::new(Token {
            token_type: TokenType::RBrace,
            range: Range::new((11, 0), (11, 0)),
        }),
        Rc::new(Token {
            token_type: TokenType::Semicolon,
            range: Range::new((11, 1), (11, 1)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("print".to_string()),
            range: Range::new((12, 0), (12, 4)),
        }),
        Rc::new(Token {
            token_type: TokenType::LParen,
            range: Range::new((12, 5), (12, 5)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("z".to_string()),
            range: Range::new((12, 6), (12, 6)),
        }),
        Rc::new(Token {
            token_type: TokenType::Dot,
            range: Range::new((12, 7), (12, 7)),
        }),
        Rc::new(Token {
            token_type: TokenType::Ident("bar".to_string()),
            range: Range::new((12, 8), (12, 10)),
        }),
        Rc::new(Token {
            token_type: TokenType::LParen,
            range: Range::new((12, 11), (12, 11)),
        }),
        Rc::new(Token {
            token_type: TokenType::RParen,
            range: Range::new((12, 12), (12, 12)),
        }),
        Rc::new(Token {
            token_type: TokenType::RParen,
            range: Range::new((12, 13), (12, 13)),
        }),
        Rc::new(Token {
            token_type: TokenType::Semicolon,
            range: Range::new((12, 14), (12, 14)),
        }),
        Rc::new(Token {
            token_type: TokenType::RBrace,
            range: Range::new((13, 0), (13, 0)),
        }),
    ];
    let expected = Node::Block(vec![
        Rc::new(Node::Struct(
            Rc::new(Node::Primary(Rc::new(Token {
                token_type: TokenType::Ident("foo".to_string()),
                range: Range::new((0, 7), (0, 9)),
            }))), 
            Rc::new(Node::Block(vec![
                Rc::new(Node::Declaration(
                    Rc::new(Node::Primary(Rc::new(Token {
                        token_type: TokenType::Ident("num".to_string()),
                        range: Range::new((1, 0), (1, 2)),
                    }))), 
                    Rc::new(Node::Primary(Rc::new(Token {
                        token_type: TokenType::Ident("x".to_string()),
                        range: Range::new((1, 4), (1, 4)),
                    }))),
                )), 
                Rc::new(Node::Declaration(
                    Rc::new(Node::Primary(Rc::new(Token {
                        token_type: TokenType::Ident("str".to_string()),
                        range: Range::new((2, 0), (2, 2)),
                    }))), 
                    Rc::new(Node::Primary(Rc::new(Token {
                        token_type: TokenType::Ident("y".to_string()),
                        range: Range::new((2, 4), (2, 4)),
                    }))),
                )), 
                Rc::new(Node::Func(
                    Rc::new(Node::Primary(Rc::new(Token {
                        token_type: TokenType::Ident("bar".to_string()),
                        range: Range::new((3, 5), (3, 7)),
                    }))), 
                    Rc::new(Node::Tuple(vec![])), 
                    Rc::new(Node::Tuple(vec![
                        Rc::new(Node::Primary(Rc::new(Token {
                            token_type: TokenType::Ident("num".to_string()),
                            range: Range::new((3, 15), (3, 17)),
                        }))), 
                        Rc::new(Node::Primary(Rc::new(Token {
                            token_type: TokenType::Ident("str".to_string()),
                            range: Range::new((3, 19), (3, 21)),
                        }))),
                    ])), 
                    Rc::new(Node::Block(vec![
                        Rc::new(Node::Return(
                            Rc::new(Node::Tuple(vec![
                                Rc::new(Node::Primary(Rc::new(Token {
                                    token_type: TokenType::Ident("x".to_string()),
                                    range: Range::new((4, 8), (4, 8)),
                                }))), 
                                Rc::new(Node::Primary(Rc::new(Token {
                                    token_type: TokenType::Ident("y".to_string()),
                                    range: Range::new((4, 11), (4, 11)),
                                }))),
                            ])),
                        )),
                    ])),
                )),
            ])),
        )),
        Rc::new(Node::Func(
            Rc::new(Node::Primary(Rc::new(Token {
                token_type: TokenType::Ident("main".to_string()),
                range: Range::new((7, 5), (7, 8)),
            }))),
            Rc::new(Node::Tuple(vec![])), 
            Rc::new(Node::None),
            Rc::new(Node::Block(vec![
                Rc::new(Node::Assignment(
                    Rc::new(Node::Declaration(
                        Rc::new(Node::Primary(Rc::new(Token {
                            token_type: TokenType::Ident("foo".to_string()),
                            range: Range::new((8, 0), (8, 2)),
                        }))),
                        Rc::new(Node::Primary(Rc::new(Token {
                            token_type: TokenType::Ident("z".to_string()),
                            range: Range::new((8, 4), (8, 4)),
                        }))),
                    )),
                    Rc::new(Node::Construct(
                        Rc::new(Node::Primary(Rc::new(Token {
                            token_type: TokenType::Ident("foo".to_string()),
                            range: Range::new((8, 8), (8, 10)),
                        }))),
                        Rc::new(Node::Block(vec![
                            Rc::new(Node::Assignment(
                                Rc::new(Node::Primary(Rc::new(Token {
                                    token_type: TokenType::Ident("x".to_string()),
                                    range: Range::new((9, 0), (9, 0)),
                                }))),
                                Rc::new(Node::Primary(Rc::new(Token {
                                    token_type: TokenType::Number(1f64),
                                    range: Range::new((9, 4), (9, 4)),
                                }))),
                            )),
                            Rc::new(Node::Assignment(
                                Rc::new(Node::Primary(Rc::new(Token {
                                    token_type: TokenType::Ident("y".to_string()),
                                    range: Range::new((10, 0), (10, 0)),
                                }))),
                                Rc::new(Node::Primary(Rc::new(Token {
                                    token_type: TokenType::String("a".to_string()),
                                    range: Range::new((10, 4), (10, 4)),
                                }))),
                            )),
                        ])),
                    )),
                )),
                Rc::new(Node::FunctionCall(
                    Rc::new(Node::Primary(Rc::new(Token {
                        token_type: TokenType::Ident("print".to_string()),
                        range: Range::new((12, 0), (12, 4)),
                    }))),
                    Rc::new(Node::Tuple(vec![
                        Rc::new(Node::FunctionCall(
                            Rc::new(Node::Access(vec![
                                Rc::new(Node::Primary(Rc::new(Token {
                                    token_type: TokenType::Ident("z".to_string()),
                                    range: Range::new((12, 6), (12, 6)),
                                }))),
                                Rc::new(Node::Primary(Rc::new(Token {
                                    token_type: TokenType::Ident("bar".to_string()),
                                    range: Range::new((12, 8), (12, 10)),
                                }))),
                            ])),
                            Rc::new(Node::Tuple(vec![])),
                        )),
                    ])), 
                )),
            ])),
        )),
    ]);
    let mut parser = Parser::new(&input);
    match parser.statement_block() {
        Ok(output) => assert_eq!(expected, output),
        Err(e) => {
            dbg!(e);
            panic!()
        }
    }
}


