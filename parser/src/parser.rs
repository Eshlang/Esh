use std::result;

use lexer::types::{Keyword, Token, TokenType};

/// A syntactical node
#[derive(Debug, PartialEq)]
pub enum Node {
    Primary(Option<TokenType>),
    Unary(UnaryNode),
    Binary(BinaryNode),
    Ternary(TernaryNode),
    Quaternion(QuaternionNode),
    Block(Vec<Node>),
}

/// A syntactical operator
#[derive(Debug, PartialEq)]
pub enum Operator {
    FunctionCall,           // foo(a)
    Not,                    // !a
    Negative,               // -a
    Product,                // a * b
    Quotient,               // a / b
    Modulo,                 // a % b
    Sum,                    // a + b
    Difference,             // a - b
    LessThan,               // a < b
    GreaterThan,            // a > b
    LessThanOrEqualTo,      // a <= b
    GreaterThanOrEqualTo,   // a >= b
    Tuple,                  // (a, b, c)
    Equal,                  // a == b
    NotEqual,               // a != b
    Declaration,            // foo a
    Return,                 // return a;
    Assignment,             // a = b
    If,                     // if a { b }
    IfElse,                 // TODO if a { b } else { c }
    While,                  // TODO while a { b }
    Func,                   // func foo (a, b) { c }
    Struct,                 // TODO struct foo { a }
}

/// A parser error
#[derive(Debug, PartialEq)]
pub enum ParserError {
    InvalidToken,
    InvalidStatement,
    MissingIdentifier,
    MissingSemicolon,
    MissingParenthesis,
    MissingBrace,
}

/// A node with a single operand
#[derive(Debug, PartialEq)]
pub struct UnaryNode {
    operator: Operator,
    operand: Box<Node>,
}

/// A node with two operands
#[derive(Debug, PartialEq)]
pub struct BinaryNode {
    operator: Operator,
    left: Box<Node>,
    right: Box<Node>,
}

/// A node with three operands
#[derive(Debug, PartialEq)]
pub struct TernaryNode {
    operator: Operator,
    node_1: Box<Node>,
    node_2: Box<Node>,
    node_3: Box<Node>,
}

/// A node with four operands
#[derive(Debug, PartialEq)]
pub struct QuaternionNode {
    operator: Operator,
    node_1: Box<Node>,
    node_2: Box<Node>,
    node_3: Box<Node>,
    node_4: Box<Node>,
}
    
macro_rules! expect {
    ($self:expr, $token:pat) => {
        if $self.is_at_end() || match $self.curr() {
            $token => false,
            _ => true
        } {
            return Err(if let $token = TokenType::Ident("".to_string()) {
                ParserError::MissingIdentifier
            } else if let $token = TokenType::Semicolon {
                ParserError::MissingSemicolon
            } else if let $token = TokenType::LParen {
                ParserError::MissingParenthesis
            } else if let $token = TokenType::RParen {
                ParserError::MissingParenthesis
            } else if let $token = TokenType::LBrace {
                ParserError::MissingBrace
            } else if let $token = TokenType::RBrace {
                ParserError::MissingBrace
            } else {
                ParserError::InvalidToken
            })
        }
    }
}

pub struct Parser<'a> {
    tokens: &'a [Token],
    current: usize,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a [Token]) -> Self {
        Self {
            tokens: input,
            current: 0,
        }
    }

    /// Gets the current token
    fn curr(&self) -> &TokenType {
        &self.tokens[self.current].token_type
    }

    /// Gets the previous token
    fn prev(&self) -> &TokenType {
        &self.tokens[self.current - 1].token_type
    }

    /// Advances to the next token
    fn advance(&mut self) {
        self.current += 1;
    }

    /// If the current token is out of range
    fn is_at_end(&mut self) -> bool {
        self.current >= self.tokens.len()
    }

    /// Returns the current statement block
    fn statement_block(&mut self) -> Result<Node, ParserError> {
        let mut block = vec![];
        while !self.is_at_end() {
            block.push(self.statement()?);
            if self.is_at_end() {
                break;
            }
            expect!(self, TokenType::Semicolon);
            self.advance();
            if self.is_at_end() || *self.curr() == TokenType::RBrace {
                break;
            }
        }
        return Ok(Node::Block(block));
    }

    /// Returns the current statement
    fn statement(&mut self) -> Result<Node, ParserError> {
        match self.curr() {
            TokenType::Ident(_) => {
                self.assignment()
            },
            TokenType::Keyword(Keyword::Func) => {
                self.func()
            },
            TokenType::Keyword(Keyword::If) => {
                self.if_block()
            },
            TokenType::Keyword(Keyword::Return) => {
                self.return_block()
            },
            _ => Err(ParserError::InvalidStatement)
        }
    }

    /// Returns the current assignment statement
    fn assignment(&mut self) -> Result<Node, ParserError> {
        let mut expr = self.declaration()?;
        if !self.is_at_end() && *self.curr() == TokenType::Assign {
            self.advance();
            expr = Node::Binary(BinaryNode {
                operator: Operator::Assignment,
                left: Box::new(expr),
                right: Box::new(self.expression()?),
            });
        }
        return Ok(expr);
    }

    /// Returns the current function declaration statement
    fn func(&mut self) -> Result<Node, ParserError> {
        let expr = Node::Quaternion(QuaternionNode {
            operator: Operator::Func,
            node_1: Box::new({  // Function name
                self.advance();
                expect!(self, TokenType::Ident(_));
                self.ident()?
            }),
            node_2: Box::new({  // Function parameters
                expect!(self, TokenType::LParen);
                self.primary()?
            }),
            node_3: Box::new({  // Return type
                if *self.curr() == TokenType::Arrow {
                    self.advance();
                    self.primary()?
                } else {
                    Node::Primary(None)
                }
            }),
            node_4: Box::new({  // Function body
                expect!(self, TokenType::LBrace);
                self.advance();
                self.statement_block()?
            }),
        });
        expect!(self, TokenType::RBrace);
        self.advance();
        return Ok(expr);
    }

    /// Returns the current if statement
    fn if_block(&mut self) -> Result<Node, ParserError> {
        let expr = Node::Binary(BinaryNode {
            operator: Operator::If,
            left: Box::new({    // If statement expression
                self.advance();
                self.equality()?
            }),
            right: Box::new({   // If statement body
                expect!(self, TokenType::LBrace);
                self.advance();
                self.statement_block()?
            }),
        });
        expect!(self, TokenType::RBrace);
        self.advance();
        return Ok(expr);
    }

    /// Returns the current return statement
    fn return_block(&mut self) -> Result<Node, ParserError> {
        Ok(Node::Unary(UnaryNode {
            operator: Operator::Return,
            operand: Box::new({
                self.advance();
                self.expression()?
            }),
        }))
    }

    /// Returns the current variable declaration
    fn declaration(&mut self) -> Result<Node, ParserError> {
        match self.curr() {
            TokenType::Ident(_) => (),
            _ => return self.expression()
        }
        let expr = self.expression()?;
        if let TokenType::Ident(_) = self.curr() {
            return Ok(Node::Binary(BinaryNode {
                operator: Operator::Declaration,
                left: Box::new(expr),
                right: Box::new(self.ident()?),
            }));
        }
        return Ok(expr);
    }

    /// Returns the current expression
    fn expression(&mut self) -> Result<Node, ParserError> {
        self.equality()
    }

    /// Returns the current equality
    fn equality(&mut self) -> Result<Node, ParserError> {
        let mut expr = self.comparison()?;
        while !self.is_at_end() {
            match self.curr() {
                TokenType::Equal => {
                    self.advance();
                    expr = Node::Binary(BinaryNode {
                            operator: Operator::Equal, 
                            left: Box::new(expr), 
                            right: Box::new(self.comparison()?),
                        }
                    )
                },
                TokenType::NotEqual => {
                    self.advance();
                    expr = Node::Binary(BinaryNode {
                            operator: Operator::NotEqual, 
                            left: Box::new(expr), 
                            right: Box::new(self.comparison()?),
                        }
                    )
                },
                _ => break
            }
        }
        return Ok(expr);
    }

    /// Returns the current comparison
    fn comparison(&mut self) -> Result<Node, ParserError> {
        let mut expr = self.term()?;
        while !self.is_at_end() {
            match self.curr() {
                TokenType::LAngle => {
                    self.advance();
                    expr = Node::Binary(BinaryNode {
                            operator: Operator::LessThan, 
                            left: Box::new(expr), 
                            right: Box::new(self.term()?),
                        }
                    )
                },
                TokenType::RAngle => {
                    self.advance();
                    expr = Node::Binary(BinaryNode {
                            operator: Operator::GreaterThan, 
                            left: Box::new(expr), 
                            right: Box::new(self.term()?),
                        }
                    )
                },
                TokenType::LTEqual => {
                    self.advance();
                    expr = Node::Binary(BinaryNode {
                            operator: Operator::LessThanOrEqualTo, 
                            left: Box::new(expr), 
                            right: Box::new(self.term()?),
                        }
                    )
                },
                TokenType::GTEqual => {
                    self.advance();
                    expr = Node::Binary(BinaryNode {
                            operator: Operator::GreaterThanOrEqualTo, 
                            left: Box::new(expr), 
                            right: Box::new(self.term()?),
                        }
                    )
                },
                _ => break
            }
        }
        return Ok(expr);
    }

    /// Returns the current term operation
    fn term(&mut self) -> Result<Node, ParserError> {
        let mut expr = self.factor()?;
        while !self.is_at_end() {
            match self.curr() {
                TokenType::Plus => {
                    self.advance();
                    expr = Node::Binary(BinaryNode {
                            operator: Operator::Sum, 
                            left: Box::new(expr), 
                            right: Box::new(self.factor()?),
                        }
                    )
                },
                TokenType::Dash => {
                    self.advance();
                    expr = Node::Binary(BinaryNode {
                            operator: Operator::Difference, 
                            left: Box::new(expr), 
                            right: Box::new(self.factor()?),
                        }
                    )
                },
                _ => break
            }
        }
        return Ok(expr);
    }

    /// Returns the current factor operation
    fn factor(&mut self) -> Result<Node, ParserError> {
        let mut expr = self.unary()?;
        while !self.is_at_end() {
            match self.curr() {
                TokenType::Asterisk => {
                    self.advance();
                    expr = Node::Binary(BinaryNode {
                            operator: Operator::Product, 
                            left: Box::new(expr), 
                            right: Box::new(self.unary()?),
                        }
                    )
                },
                TokenType::Slash => {
                    self.advance();
                    expr = Node::Binary(BinaryNode {
                            operator: Operator::Quotient, 
                            left: Box::new(expr), 
                            right: Box::new(self.unary()?),
                        }
                    )
                },
                TokenType::Perc => {
                    self.advance();
                    expr = Node::Binary(BinaryNode {
                            operator: Operator::Modulo, 
                            left: Box::new(expr), 
                            right: Box::new(self.unary()?),
                        }
                    )
                },
                _ => break
            }
        }
        return Ok(expr);
    }

    /// Returns the current unary operation
    fn unary(&mut self) -> Result<Node, ParserError> {
        match self.curr() {
            TokenType::Bang => {
                self.advance();
                Ok(Node::Unary(UnaryNode {
                        operator: Operator::Not,
                        operand: Box::new(self.unary()?),
                    }
                ))
            },
            TokenType::Dash => {
                self.advance();
                Ok(Node::Unary(UnaryNode {
                        operator: Operator::Negative,
                        operand: Box::new(self.unary()?),
                    }
                ))
            },
            _ => self.primary(),
        }
    }

    /// Returns the current primary node
    fn primary(&mut self) -> Result<Node, ParserError> {
        match self.curr() {
            TokenType::Ident(_) => {
                self.function_call()
            }
            TokenType::Number(_) | TokenType::String(_) => {
                self.advance();
                Ok(Node::Primary(Some(self.prev().clone())))
            },
            TokenType::LParen => {
                let start = self.current;
                self.advance();
                let expr = match self.curr() {
                    TokenType::Ident(_) => self.declaration()?,
                    _ => self.expression()?
                };
                match self.curr() {
                    TokenType::RParen => {
                        self.advance();
                        Ok(expr)
                    },
                    TokenType::Comma => {
                        self.current = start;
                        return self.tuple();
                    },
                    _ => Err(ParserError::MissingParenthesis)
                }
            },
            _ => {
                Err(ParserError::InvalidToken)
            }
        }
    }

    fn function_call(&mut self) -> Result<Node, ParserError> {
        let mut expr = self.ident()?;
        match self.curr() {
            TokenType::LParen => expr = Node::Binary(BinaryNode {
                    operator: Operator::FunctionCall,
                    left: Box::new(expr),
                    right: Box::new(self.primary()?),
                }),
            _ => ()
        }
        return Ok(expr);
    }

    /// Returns the current tuple
    fn tuple(&mut self) -> Result<Node, ParserError> {
        expect!(self, TokenType::LParen);
        self.advance();
        let mut block = vec![self.declaration()?];
        expect!(self, TokenType::Comma);
        while !self.is_at_end() {
            self.advance();
            block.push(self.declaration()?);
            match self.curr() {
                TokenType::Comma => (),
                TokenType::RParen => {
                    self.advance();
                    break;
                },
                _ => return Err(ParserError::MissingParenthesis)
            }
        }
        return Ok(Node::Unary(UnaryNode {
            operator: Operator::Tuple,
            operand: Box::new(Node::Block(block)),
        }));
    }

    /// Returns the current identifier
    fn ident(&mut self) -> Result<Node, ParserError> {
        expect!(self, TokenType::Ident(_));
        self.advance();
        Ok(Node::Primary(Some(self.prev().clone())))
    }
}

#[cfg(test)]
mod tests {

    use lexer::types::Range;

    use super::*;

    #[test]
    pub fn expression_test() {
        // x + 8 / 2 * 4
        let input = [
            Token {
                token_type: TokenType::Ident("x".to_string()),
                range: Range::new((0, 0), (0, 0)),
            },
            Token {
                token_type: TokenType::Plus,
                range: Range::new((0, 2), (0, 2)),
            },
            Token {
                token_type: TokenType::Number(8f64),
                range: Range::new((0, 4), (0, 4)),
            },
            Token {
                token_type: TokenType::Slash,
                range: Range::new((0, 6), (0, 6)),
            },
            Token {
                token_type: TokenType::Number(2f64),
                range: Range::new((0, 8), (0, 8)),
            },
            Token {
                token_type: TokenType::Asterisk,
                range: Range::new((0, 10), (0, 10)),
            },
            Token {
                token_type: TokenType::Number(4f64),
                range: Range::new((0, 12), (0, 12)),
            },
        ];
        let expected = Node::Binary(BinaryNode {
                operator: Operator::Sum,
                left: Box::new(Node::Primary(Some(TokenType::Ident("x".to_string())))),
                right: Box::new(Node::Binary(BinaryNode {
                            operator: Operator::Product,
                            left: Box::new(Node::Binary(BinaryNode {
                                        operator: Operator::Quotient,
                                        left: Box::new(Node::Primary(Some(TokenType::Number(8f64)))),
                                        right: Box::new(Node::Primary(Some(TokenType::Number(2f64)))),
                                    }
                                )
                            ),
                            right: Box::new(Node::Primary(Some(TokenType::Number(4f64)))),
                        }
                    )
                )
            }
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
            Token {
                token_type: TokenType::LParen,
                range: Range::new((0, 0), (0, 0)),
            },
            Token {
                token_type: TokenType::Ident("x".to_string()),
                range: Range::new((0, 1), (0, 1)),
            },
            Token {
                token_type: TokenType::Plus,
                range: Range::new((0, 3), (0, 3)),
            },
            Token {
                token_type: TokenType::Number(8f64),
                range: Range::new((0, 5), (0, 5)),
            },
            Token {
                token_type: TokenType::RParen,
                range: Range::new((0, 6), (0, 6)),
            },
            Token {
                token_type: TokenType::Slash,
                range: Range::new((0, 8), (0, 8)),
            },
            Token {
                token_type: TokenType::LParen,
                range: Range::new((0, 10), (0, 10)),
            },
            Token {
                token_type: TokenType::Number(2f64),
                range: Range::new((0, 11), (0, 11)),
            },
            Token {
                token_type: TokenType::Asterisk,
                range: Range::new((0, 13), (0, 13)),
            },
            Token {
                token_type: TokenType::Number(4f64),
                range: Range::new((0, 15), (0, 15)),
            },
            Token {
                token_type: TokenType::RParen,
                range: Range::new((0, 16), (0, 16)),
            },
        ];
        let expected = Node::Binary(BinaryNode {
                operator: Operator::Quotient,
                left: Box::new(Node::Binary(BinaryNode {
                    operator: Operator::Sum,
                    left: Box::new(Node::Primary(Some(TokenType::Ident("x".to_string())))),
                    right: Box::new(Node::Primary(Some(TokenType::Number(8f64))))
                })),
                right: Box::new(Node::Binary(BinaryNode {
                    operator: Operator::Product,
                    left: Box::new(Node::Primary(Some(TokenType::Number(2f64)))),
                    right: Box::new(Node::Primary(Some(TokenType::Number(4f64))))
                })),
            }
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
            Token {
                token_type: TokenType::LParen,
                range: Range::new((0, 0), (0, 0)),
            },
            Token {
                token_type: TokenType::Ident("x".to_string()),
                range: Range::new((0, 1), (0, 1)),
            },
            Token {
                token_type: TokenType::Comma,
                range: Range::new((0, 2), (0, 2)),
            },
            Token {
                token_type: TokenType::Number(3f64),
                range: Range::new((0, 4), (0, 4)),
            },
            Token {
                token_type: TokenType::Comma,
                range: Range::new((0, 5), (0, 5)),
            },
            Token {
                token_type: TokenType::String("test".to_string()),
                range: Range::new((0, 7), (0, 12)),
            },
            Token {
                token_type: TokenType::RParen,
                range: Range::new((0, 13), (0, 13)),
            },
        ];
        let expected = Node::Unary(UnaryNode {
            operator: Operator::Tuple,
            operand: Box::new(Node::Block(vec![
                Node::Primary(Some(TokenType::Ident("x".to_string()))),
                Node::Primary(Some(TokenType::Number(3f64))),
                Node::Primary(Some(TokenType::String("test".to_string()))),
            ])),
        });
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
    pub fn assignment_test() {
        // num x = 5
        let input = [
            Token {
                token_type: TokenType::Ident("num".to_string()),
                range: Range::new((0, 0), (0, 2)),
            },
            Token {
                token_type: TokenType::Ident("x".to_string()),
                range: Range::new((0, 4), (0, 4)),
            },
            Token {
                token_type: TokenType::Assign,
                range: Range::new((0, 6), (0, 6)),
            },
            Token {
                token_type: TokenType::Number(5f64),
                range: Range::new((0, 8), (0, 8)),
            },
        ];
        let expected = Node::Binary(BinaryNode {
            operator: Operator::Assignment,
            left: Box::new(Node::Binary(BinaryNode {
                operator: Operator::Declaration,
                left: Box::new(Node::Primary(Some(TokenType::Ident("num".to_string())))),
                right: Box::new(Node::Primary(Some(TokenType::Ident("x".to_string())))),
            })),
            right: Box::new(Node::Primary(Some(TokenType::Number(5f64)))),
        });
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
        // num x = 5;
        // str y = "hello";
        let input = [
            Token {
                token_type: TokenType::Ident("num".to_string()),
                range: Range::new((0, 0), (0, 2)),
            },
            Token {
                token_type: TokenType::Ident("x".to_string()),
                range: Range::new((0, 4), (0, 4)),
            },
            Token {
                token_type: TokenType::Assign,
                range: Range::new((0, 6), (0, 6)),
            },
            Token {
                token_type: TokenType::Number(5f64),
                range: Range::new((0, 8), (0, 8)),
            },
            Token {
                token_type: TokenType::Semicolon,
                range: Range::new((0, 9), (0, 9)),
            },
            Token {
                token_type: TokenType::Ident("str".to_string()),
                range: Range::new((1, 0), (1, 2)),
            },
            Token {
                token_type: TokenType::Ident("y".to_string()),
                range: Range::new((1, 4), (1, 4)),
            },
            Token {
                token_type: TokenType::Assign,
                range: Range::new((1, 6), (1, 6)),
            },
            Token {
                token_type: TokenType::String("hello".to_string()),
                range: Range::new((1, 8), (1, 14)),
            },
            Token {
                token_type: TokenType::Semicolon,
                range: Range::new((0, 15), (0, 15)),
            },
        ];
        let expected = Node::Block(vec![
            Node::Binary(BinaryNode {
                operator: Operator::Assignment,
                left: Box::new(Node::Binary(BinaryNode {
                    operator: Operator::Declaration,
                    left: Box::new(Node::Primary(Some(TokenType::Ident("num".to_string())))),
                    right: Box::new(Node::Primary(Some(TokenType::Ident("x".to_string())))),
                })),
                right: Box::new(Node::Primary(Some(TokenType::Number(5f64)))),
            }),
            Node::Binary(BinaryNode {
                operator: Operator::Assignment,
                left: Box::new(Node::Binary(BinaryNode {
                    operator: Operator::Declaration,
                    left: Box::new(Node::Primary(Some(TokenType::Ident("str".to_string())))),
                    right: Box::new(Node::Primary(Some(TokenType::Ident("y".to_string())))),
                })),
                right: Box::new(Node::Primary(Some(TokenType::String("hello".to_string())))),
            }),
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
    pub fn if_statement_test() {
        // if x == 5 {
        // str y = "hello";
        // }
        let input = [
            Token {
                token_type: TokenType::Keyword(Keyword::If),
                range: Range::new((0, 0), (0, 1)),
            },
            Token {
                token_type: TokenType::Ident("x".to_string()),
                range: Range::new((0, 3), (0, 3)),
            },
            Token {
                token_type: TokenType::Equal,
                range: Range::new((0, 5), (0, 6)),
            },
            Token {
                token_type: TokenType::Number(5f64),
                range: Range::new((0, 8), (0, 8)),
            },
            Token {
                token_type: TokenType::LBrace,
                range: Range::new((0, 10), (0, 10)),
            },
            Token {
                token_type: TokenType::Ident("str".to_string()),
                range: Range::new((1, 0), (1, 2)),
            },
            Token {
                token_type: TokenType::Ident("y".to_string()),
                range: Range::new((1, 4), (1, 4)),
            },
            Token {
                token_type: TokenType::Assign,
                range: Range::new((1, 6), (1, 6)),
            },
            Token {
                token_type: TokenType::Ident("hello".to_string()),
                range: Range::new((1, 8), (1, 14)),
            },
            Token {
                token_type: TokenType::Semicolon,
                range: Range::new((1, 15), (1, 15)),
            },
            Token {
                token_type: TokenType::RBrace,
                range: Range::new((2, 0), (2, 0)),
            },
        ];
        let expected = Node::Binary(BinaryNode {
            operator: Operator::If,
            left: Box::new(Node::Binary(BinaryNode {
                operator: Operator::Equal,
                left: Box::new(Node::Primary(Some(TokenType::Ident("x".to_string())))),
                right: Box::new(Node::Primary(Some(TokenType::Number(5f64)))),
            })),
            right: Box::new(Node::Block(vec![
                Node::Binary(BinaryNode {
                    operator: Operator::Assignment,
                    left: Box::new(Node::Binary(BinaryNode {
                        operator: Operator::Declaration,
                        left: Box::new(Node::Primary(Some(TokenType::Ident("str".to_string())))),
                        right: Box::new(Node::Primary(Some(TokenType::Ident("y".to_string())))),
                    })),
                    right: Box::new(Node::Primary(Some(TokenType::Ident("hello".to_string())))),
                }),
            ])),
        });
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
        // bar(x);
        // }
        let input = [
            Token {
                token_type: TokenType::Keyword(Keyword::Func),
                range: Range::new((0, 0), (0, 3)),
            },
            Token {
                token_type: TokenType::Ident("foo".to_string()),
                range: Range::new((0, 5), (0, 7)),
            },
            Token {
                token_type: TokenType::LParen,
                range: Range::new((0, 8), (0, 8)),
            },
            Token {
                token_type: TokenType::Ident("num".to_string()),
                range: Range::new((0, 9), (0, 11)),
            },
            Token {
                token_type: TokenType::Ident("x".to_string()),
                range: Range::new((0, 13), (0, 13)),
            },
            Token {
                token_type: TokenType::RParen,
                range: Range::new((0, 14), (0, 14)),
            },
            Token {
                token_type: TokenType::LBrace,
                range: Range::new((0, 16), (0, 16)),
            },
            Token {
                token_type: TokenType::Ident("bar".to_string()),
                range: Range::new((1, 0), (1, 2)),
            },
            Token {
                token_type: TokenType::LParen,
                range: Range::new((1, 3), (1, 3)),
            },
            Token {
                token_type: TokenType::Ident("x".to_string()),
                range: Range::new((1, 4), (1, 4)),
            },
            Token {
                token_type: TokenType::RParen,
                range: Range::new((1, 5), (1, 5)),
            },
            Token {
                token_type: TokenType::Semicolon,
                range: Range::new((1, 6), (1, 6)),
            },
            Token {
                token_type: TokenType::RBrace,
                range: Range::new((2, 0), (2, 0)),
            },
        ];
        let expected = Node::Quaternion(QuaternionNode {
            operator: Operator::Func,
            node_1: Box::new(Node::Primary(Some(TokenType::Ident("foo".to_string())))),
            node_2: Box::new(Node::Binary(BinaryNode {
                        operator: Operator::Declaration,
                        left: Box::new(Node::Primary(Some(TokenType::Ident("num".to_string())))),
                        right: Box::new(Node::Primary(Some(TokenType::Ident("x".to_string())))),
            })),
            node_3: Box::new(Node::Primary(None)),
            node_4: Box::new(Node::Block(vec![
                Node::Binary(BinaryNode {
                    operator: Operator::FunctionCall,
                    left: Box::new(Node::Primary(Some(TokenType::Ident("bar".to_string())))),
                    right: Box::new(Node::Primary(Some(TokenType::Ident("x".to_string())))),
                }),
            ])),
        });
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
        // return x * bar(2);
        // }
        let input = [
            Token {
                token_type: TokenType::Keyword(Keyword::Func),
                range: Range::new((0, 0), (0, 3)),
            },
            Token {
                token_type: TokenType::Ident("foo".to_string()),
                range: Range::new((0, 5), (0, 7)),
            },
            Token {
                token_type: TokenType::LParen,
                range: Range::new((0, 8), (0, 8)),
            },
            Token {
                token_type: TokenType::Ident("num".to_string()),
                range: Range::new((0, 9), (0, 11)),
            },
            Token {
                token_type: TokenType::Ident("x".to_string()),
                range: Range::new((0, 13), (0, 13)),
            },
            Token {
                token_type: TokenType::RParen,
                range: Range::new((0, 14), (0, 14)),
            },
            Token {
                token_type: TokenType::Arrow,
                range: Range::new((0, 14), (0, 14)),
            },
            Token {
                token_type: TokenType::Ident("num".to_string()),
                range: Range::new((0, 16), (0, 18)),
            },
            Token {
                token_type: TokenType::LBrace,
                range: Range::new((0, 20), (0, 20)),
            },
            Token {
                token_type: TokenType::Keyword(Keyword::Return),
                range: Range::new((1, 0), (1, 5)),
            },
            Token {
                token_type: TokenType::Ident("x".to_string()),
                range: Range::new((1, 7), (1, 7)),
            },
            Token {
                token_type: TokenType::Asterisk,
                range: Range::new((1, 9), (1, 9)),
            },
            Token {
                token_type: TokenType::Ident("bar".to_string()),
                range: Range::new((1, 11), (1, 13)),
            },
            Token {
                token_type: TokenType::LParen,
                range: Range::new((1, 14), (1, 14)),
            },
            Token {
                token_type: TokenType::Number(2f64),
                range: Range::new((1, 15), (1, 15)),
            },
            Token {
                token_type: TokenType::RParen,
                range: Range::new((1, 16), (1, 16)),
            },
            Token {
                token_type: TokenType::Semicolon,
                range: Range::new((1, 17), (1, 17)),
            },
            Token {
                token_type: TokenType::RBrace,
                range: Range::new((2, 0), (2, 0)),
            },
        ];
        let expected = Node::Quaternion(QuaternionNode {
            operator: Operator::Func,
            node_1: Box::new(Node::Primary(Some(TokenType::Ident("foo".to_string())))),
            node_2: Box::new(Node::Binary(BinaryNode {
                        operator: Operator::Declaration,
                        left: Box::new(Node::Primary(Some(TokenType::Ident("num".to_string())))),
                        right: Box::new(Node::Primary(Some(TokenType::Ident("x".to_string())))),
            })),
            node_3: Box::new(Node::Primary(Some(TokenType::Ident("num".to_string())))),
            node_4: Box::new(Node::Block(vec![
                Node::Unary(UnaryNode {
                    operator: Operator::Return,
                    operand: Box::new(Node::Binary(BinaryNode {
                        operator: Operator::Product,
                        left: Box::new(Node::Primary(Some(TokenType::Ident("x".to_string())))),
                        right: Box::new(Node::Binary(BinaryNode {
                            operator: Operator::FunctionCall,
                            left: Box::new(Node::Primary(Some(TokenType::Ident("bar".to_string())))),
                            right: Box::new(Node::Primary(Some(TokenType::Number(2f64)))),
                        })),
                    })),
                }),
            ])),
        });
        let mut parser = Parser::new(&input);
        match parser.statement() {
            Ok(output) => assert_eq!(expected, output),
            Err(e) => {
                dbg!(e);
                panic!()
            }
        }
    }
}