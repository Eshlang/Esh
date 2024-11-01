use lexer::{types::Token, types::TokenType};

#[derive(Debug, PartialEq)]
pub enum Node {
    Primary(TokenType),
    Unary(UnaryNode),
    Binary(BinaryNode),
    Ternary(TernaryNode),
    Block(Box<Vec<Node>>),
}

#[derive(Debug, PartialEq)]
pub enum Operator {
    Not,
    Negative,
    Product,
    Quotient,
    Modulo,
    Sum,
    Difference,
    LessThan,
    GreaterThan,
    LessThanOrEqualTo,
    GreaterThanOrEqualTo,
    Equal,
    NotEqual,
    Declaration,
    Assignment,
}

#[derive(Debug, PartialEq)]
pub enum ParserError {
    InvalidToken,
    InvalidStatement,
    MissingIdentifier,
    MissingSemicolon,
}

#[derive(Debug, PartialEq)]
pub struct UnaryNode {
    operator: Operator,
    operand: Box<Node>,
}

#[derive(Debug, PartialEq)]
pub struct BinaryNode {
    operator: Operator,
    left: Box<Node>,
    right: Box<Node>,
}

#[derive(Debug, PartialEq)]
pub struct TernaryNode {
    operator: Operator,
    left: Box<Node>,
    middle: Box<Node>,
    right: Box<Node>,
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

    fn curr(&mut self) -> TokenType {
        self.tokens[self.current].token_type.clone()
    }

    fn prev(&mut self) -> TokenType {
        self.tokens[self.current - 1].token_type.clone()
    }

    fn is_at_end(&mut self) -> bool {
        self.current == self.tokens.len()
    }

    fn statement_block(&mut self) -> Result<Node, ParserError> {
        let mut block = vec![];
        while !self.is_at_end() {
            block.push(self.statement()?);
            if self.curr() == TokenType::Semicolon {
                self.current += 1;
            } else {
                return Err(ParserError::MissingSemicolon);
            }
        }
        return Ok(Node::Block(Box::new(block)));
    }

    fn statement(&mut self) -> Result<Node, ParserError> {
        match self.curr() {
            TokenType::Ident(_) => {
                self.assignment()
            }
            _ => Err(ParserError::InvalidStatement)
        }
    }

    fn assignment(&mut self) -> Result<Node, ParserError> {
        let mut expr = self.declaration()?;
        if !self.is_at_end() && self.curr() == TokenType::Assign {
            self.current += 1;
            expr = Node::Binary(BinaryNode {
                operator: Operator::Assignment,
                left: Box::new(expr),
                right: Box::new(self.expression()?),
            });
        }
        return Ok(expr);
    }

    fn declaration(&mut self) -> Result<Node, ParserError> {
        let expr = self.primary()?;
        match self.curr() {
            TokenType::Ident(_) => {
                Ok(Node::Binary(BinaryNode {
                    operator: Operator::Declaration,
                    left: Box::new(expr),
                    right: Box::new(self.primary()?),
                }))
            },
            _ => Err(ParserError::MissingIdentifier)
        }
    }

    fn expression(&mut self) -> Result<Node, ParserError> {
        self.equality()
    }

    fn equality(&mut self) -> Result<Node, ParserError> {
        let mut expr = self.comparison()?;
        while !self.is_at_end() {
            match self.curr() {
                TokenType::Equal => {
                    self.current += 1;
                    expr = Node::Binary(BinaryNode {
                            operator: Operator::Equal, 
                            left: Box::new(expr), 
                            right: Box::new(self.comparison()?),
                        }
                    )
                },
                TokenType::NotEqual => {
                    self.current += 1;
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

    fn comparison(&mut self) -> Result<Node, ParserError> {
        let mut expr = self.term()?;
        while !self.is_at_end() {
            match self.curr() {
                TokenType::LAngle => {
                    self.current += 1;
                    expr = Node::Binary(BinaryNode {
                            operator: Operator::LessThan, 
                            left: Box::new(expr), 
                            right: Box::new(self.term()?),
                        }
                    )
                },
                TokenType::RAngle => {
                    self.current += 1;
                    expr = Node::Binary(BinaryNode {
                            operator: Operator::GreaterThan, 
                            left: Box::new(expr), 
                            right: Box::new(self.term()?),
                        }
                    )
                },
                TokenType::LTEqual => {
                    self.current += 1;
                    expr = Node::Binary(BinaryNode {
                            operator: Operator::LessThanOrEqualTo, 
                            left: Box::new(expr), 
                            right: Box::new(self.term()?),
                        }
                    )
                },
                TokenType::GTEqual => {
                    self.current += 1;
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

    fn term(&mut self) -> Result<Node, ParserError> {
        let mut expr = self.factor()?;
        while !self.is_at_end() {
            match self.curr() {
                TokenType::Plus => {
                    self.current += 1;
                    expr = Node::Binary(BinaryNode {
                            operator: Operator::Sum, 
                            left: Box::new(expr), 
                            right: Box::new(self.factor()?),
                        }
                    )
                },
                TokenType::Dash => {
                    self.current += 1;
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

    fn factor(&mut self) -> Result<Node, ParserError> {
        let mut expr = self.unary()?;
        while !self.is_at_end() {
            match self.curr() {
                TokenType::Asterisk => {
                    self.current += 1;
                    expr = Node::Binary(BinaryNode {
                            operator: Operator::Product, 
                            left: Box::new(expr), 
                            right: Box::new(self.unary()?),
                        }
                    )
                },
                TokenType::Slash => {
                    self.current += 1;
                    expr = Node::Binary(BinaryNode {
                            operator: Operator::Quotient, 
                            left: Box::new(expr), 
                            right: Box::new(self.unary()?),
                        }
                    )
                },
                TokenType::Perc => {
                    self.current += 1;
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

    fn unary(&mut self) -> Result<Node, ParserError> {
        match self.curr() {
            TokenType::Bang => {
                self.current += 1;
                Ok(Node::Unary(UnaryNode {
                        operator: Operator::Not,
                        operand: Box::new(self.unary()?),
                    }
                ))
            },
            TokenType::Dash => {
                self.current += 1;
                Ok(Node::Unary(UnaryNode {
                        operator: Operator::Negative,
                        operand: Box::new(self.unary()?),
                    }
                ))
            },
            _ => self.primary(),
        }
    }

    fn primary(&mut self) -> Result<Node, ParserError> {
        match self.curr() {
            TokenType::Number(_) | TokenType::String(_) | TokenType::Ident(_) => {
                self.current += 1;
                Ok(Node::Primary(self.prev()))
            },
            TokenType::LParen => {
                self.current += 1;
                todo!("implement parenthesis")
            },
            _ => {
                Err(ParserError::InvalidToken)
            }
        }
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
                left: Box::new(Node::Primary(TokenType::Ident("x".to_string()))),
                right: Box::new(Node::Binary(BinaryNode {
                            operator: Operator::Product,
                            left: Box::new(Node::Binary(BinaryNode {
                                        operator: Operator::Quotient,
                                        left: Box::new(Node::Primary(TokenType::Number(8f64))),
                                        right: Box::new(Node::Primary(TokenType::Number(2f64))),
                                    }
                                )
                            ),
                            right: Box::new(Node::Primary(TokenType::Number(4f64))),
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
                left: Box::new(Node::Primary(TokenType::Ident("num".to_string()))),
                right: Box::new(Node::Primary(TokenType::Ident("x".to_string()))),
            })),
            right: Box::new(Node::Primary(TokenType::Number(5f64))),
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
        let expected = Node::Block(Box::new(vec![
            Node::Binary(BinaryNode {
                operator: Operator::Assignment,
                left: Box::new(Node::Binary(BinaryNode {
                    operator: Operator::Declaration,
                    left: Box::new(Node::Primary(TokenType::Ident("num".to_string()))),
                    right: Box::new(Node::Primary(TokenType::Ident("x".to_string()))),
                })),
                right: Box::new(Node::Primary(TokenType::Number(5f64))),
            }),
            Node::Binary(BinaryNode {
                operator: Operator::Assignment,
                left: Box::new(Node::Binary(BinaryNode {
                    operator: Operator::Declaration,
                    left: Box::new(Node::Primary(TokenType::Ident("str".to_string()))),
                    right: Box::new(Node::Primary(TokenType::Ident("y".to_string()))),
                })),
                right: Box::new(Node::Primary(TokenType::String("hello".to_string()))),
            }),
        ]));
        let mut parser = Parser::new(&input);
        match parser.statement_block() {
            Ok(output) => assert_eq!(expected, output),
            Err(e) => {
                dbg!(e);
                panic!()
            }
        }
    }
}