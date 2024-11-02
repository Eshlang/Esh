use lexer::types::{Keyword, Token, TokenType};

#[derive(Debug, PartialEq)]
pub enum Node {
    Primary(TokenType),
    Unary(UnaryNode),
    Binary(BinaryNode),
    Ternary(TernaryNode),
    Quaternion(QuaternionNode),
    Block(Box<Vec<Node>>),
}

#[derive(Debug, PartialEq)]
pub enum Operator {
    FunctionCall,
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
    Tuple,
    Equal,
    NotEqual,
    Declaration,
    Return,
    Assignment,
    If,
    IfElse, //TODO
    While, // TODO
    Func,
    Struct, // TODO
}

#[derive(Debug, PartialEq)]
pub enum ParserError {
    InvalidToken,
    InvalidStatement,
    MissingIdentifier,
    MissingSemicolon,
    MissingParenthesis,
    MissingBrace,
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
    node_1: Box<Node>,
    node_2: Box<Node>,
    node_3: Box<Node>,
}

#[derive(Debug, PartialEq)]
pub struct QuaternionNode {
    operator: Operator,
    node_1: Box<Node>,
    node_2: Box<Node>,
    node_3: Box<Node>,
    node_4: Box<Node>,
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

    fn curr(&self) -> &TokenType {
        &self.tokens[self.current].token_type
    }

    fn prev(&self) -> &TokenType {
        &self.tokens[self.current - 1].token_type
    }

    fn advance(&mut self) {
        self.current += 1;
    }

    fn is_at_end(&mut self) -> bool {
        self.current == self.tokens.len()
    }

    fn statement_block(&mut self) -> Result<Node, ParserError> {
        let mut block = vec![];
        while !self.is_at_end() {
            block.push(self.statement()?);
            if self.is_at_end() {
                break;
            }
            if *self.curr() == TokenType::Semicolon {
                self.advance();
                if self.is_at_end() || *self.curr() == TokenType::RBrace {
                    break;
                }
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

    fn func(&mut self) -> Result<Node, ParserError> {
        self.advance();
        match self.curr() {
            TokenType::Ident(_) => (),
            _ => return Err(ParserError::MissingIdentifier)
        }
        let name = self.ident()?;
        if *self.curr() != TokenType::LParen {
            return Err(ParserError::MissingParenthesis)
        }
        let params = self.tuple()?;
        let r_type = if *self.curr() == TokenType::Ampersand {
            self.advance();
            self.tuple()?
        } else {
            Node::Unary(UnaryNode {                         // TODO change to void type when added
                operator: Operator::Tuple,
                operand: Box::new(Node::Block(Box::new(vec![]))),
            })
        };
        if *self.curr() != TokenType::LBrace {
            return Err(ParserError::MissingBrace)
        }
        self.advance();
        let block = self.statement_block()?;
        if *self.curr() == TokenType::RBrace {
            self.advance();
        } else {
            return Err(ParserError::MissingBrace)
        }
        return Ok(Node::Quaternion(QuaternionNode {
            operator: Operator::Func,
            node_1: Box::new(name),
            node_2: Box::new(params),
            node_3: Box::new(r_type),
            node_4: Box::new(block),
        }));
    }

    fn if_block(&mut self) -> Result<Node, ParserError> {
        self.advance();
        let expr = self.equality()?;
        if *self.curr() != TokenType::LBrace {
            return Err(ParserError::MissingBrace)
        }
        self.advance();
        let block = self.statement_block()?;
        if *self.curr() == TokenType::RBrace {
            self.advance();
        } else {
            return Err(ParserError::MissingBrace)
        }
        return Ok(Node::Binary(BinaryNode {
            operator: Operator::If,
            left: Box::new(expr),
            right: Box::new(block),
        }));
    }

    fn return_block(&mut self) -> Result<Node, ParserError> {
        self.advance();
        let expr = self.expression()?;
        return Ok(Node::Unary(UnaryNode {
            operator: Operator::Return,
            operand: Box::new(expr),
        }));
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
            _ => Ok(expr)
        }
    }

    fn expression(&mut self) -> Result<Node, ParserError> {
        self.equality()
    }

    fn equality(&mut self) -> Result<Node, ParserError> {
        let mut expr = self.tuple()?;
        while !self.is_at_end() {
            match self.curr() {
                TokenType::Equal => {
                    self.advance();
                    expr = Node::Binary(BinaryNode {
                            operator: Operator::Equal, 
                            left: Box::new(expr), 
                            right: Box::new(self.tuple()?),
                        }
                    )
                },
                TokenType::NotEqual => {
                    self.advance();
                    expr = Node::Binary(BinaryNode {
                            operator: Operator::NotEqual, 
                            left: Box::new(expr), 
                            right: Box::new(self.tuple()?),
                        }
                    )
                },
                _ => break
            }
        }
        return Ok(expr);
    }

    fn tuple(&mut self) -> Result<Node, ParserError> {
        if *self.curr() != TokenType::LParen {
            return self.comparison();
        }
        let start = self.current;
        self.advance();
        if *self.curr() == TokenType::RParen {
            return Ok(Node::Unary(UnaryNode {
                operator: Operator::Tuple,
                operand: Box::new(Node::Block(Box::new(vec![]))),          // TODO change to void type when added
            }))
        }
        let mut expr = self.comparison()?;
        match self.curr() {
            TokenType::Comma => (),
            TokenType::Ident(_) => {
                if let TokenType::Ident(_) = self.prev() {
                    self.current = start + 1;
                    expr = self.declaration()?;
                    match self.curr() {
                        TokenType::Comma => (),
                        TokenType::RParen => {
                            self.advance();
                            return Ok(expr);
                        },
                        _ => return Err(ParserError::MissingParenthesis),
                    }
                }
            },
            TokenType::RParen => {
                self.current = start;
                return self.comparison();
            },
            _ => return Err(ParserError::MissingParenthesis),
        }
        let mut block = vec![expr];
        while !self.is_at_end() {
            let start = self.current;
            self.advance();
            expr = self.comparison()?;
            match self.curr() {
                TokenType::Comma => (),
                TokenType::Ident(_) => {
                    if let TokenType::Ident(_) = self.prev() {
                        self.current = start + 1;
                        expr = self.declaration()?;
                        match self.curr() {
                            TokenType::Comma => (),
                            TokenType::RParen => {
                                block.push(expr);
                                return Ok(Node::Unary(UnaryNode {
                                    operator: Operator::Tuple,
                                    operand: Box::new(Node::Block(Box::new(block)))
                                }));
                            },
                            _ => break,
                        }
                    }
                },
                TokenType::RParen => {
                    block.push(expr);
                    return Ok(Node::Unary(UnaryNode {
                        operator: Operator::Tuple,
                        operand: Box::new(Node::Block(Box::new(block)))
                    }));
                },
                _ => break,
            }
            block.push(expr);
        }
        return Err(ParserError::MissingParenthesis)
    }

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

    fn primary(&mut self) -> Result<Node, ParserError> {
        match self.curr() {
            TokenType::Ident(_) => {
                let expr = self.ident()?;
                match self.curr() {
                    TokenType::LParen => {
                        Ok(Node::Binary(BinaryNode {
                            operator: Operator::FunctionCall,
                            left: Box::new(expr),
                            right: Box::new(self.tuple()?),
                        }))
                    },
                    _ => Ok(expr)
                }
            }
            TokenType::Number(_) | TokenType::String(_) => {
                self.advance();
                Ok(Node::Primary(self.prev().clone()))
            },
            TokenType::LParen => {
                self.advance();
                let expr = self.expression()?;
                match self.curr() {
                    TokenType::RParen => {
                        self.advance();
                        Ok(expr)
                    },
                    _ => Err(ParserError::MissingParenthesis)
                }
            },
            _ => {
                Err(ParserError::InvalidToken)
            }
        }
    }

    fn ident(&mut self) -> Result<Node, ParserError> {
        match self.curr() {
            TokenType::Ident(_) => {
                self.advance();
                Ok(Node::Primary(self.prev().clone()))
            },
            _ => Err(ParserError::InvalidToken)
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
                    left: Box::new(Node::Primary(TokenType::Ident("x".to_string()))),
                    right: Box::new(Node::Primary(TokenType::Number(8f64)))
                })),
                right: Box::new(Node::Binary(BinaryNode {
                    operator: Operator::Product,
                    left: Box::new(Node::Primary(TokenType::Number(2f64))),
                    right: Box::new(Node::Primary(TokenType::Number(4f64)))
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
            operand: Box::new(Node::Block(Box::new(vec![
                Node::Primary(TokenType::Ident("x".to_string())),
                Node::Primary(TokenType::Number(3f64)),
                Node::Primary(TokenType::String("test".to_string())),
            ]))),
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
                left: Box::new(Node::Primary(TokenType::Ident("x".to_string()))),
                right: Box::new(Node::Primary(TokenType::Number(5f64))),
            })),
            right: Box::new(Node::Block(Box::new(vec![
                Node::Binary(BinaryNode {
                    operator: Operator::Assignment,
                    left: Box::new(Node::Binary(BinaryNode {
                        operator: Operator::Declaration,
                        left: Box::new(Node::Primary(TokenType::Ident("str".to_string()))),
                        right: Box::new(Node::Primary(TokenType::Ident("y".to_string()))),
                    })),
                    right: Box::new(Node::Primary(TokenType::Ident("hello".to_string()))),
                }),
            ]))),
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
            node_1: Box::new(Node::Primary(TokenType::Ident("foo".to_string()))),
            node_2: Box::new(Node::Binary(BinaryNode {
                        operator: Operator::Declaration,
                        left: Box::new(Node::Primary(TokenType::Ident("num".to_string()))),
                        right: Box::new(Node::Primary(TokenType::Ident("x".to_string()))),
            })),
            node_3: Box::new(Node::Unary(UnaryNode {
                operator: Operator::Tuple,
                operand: Box::new(Node::Block(Box::new(vec![]))),
            })),
            node_4: Box::new(Node::Block(Box::new(vec![
                Node::Binary(BinaryNode {
                    operator: Operator::FunctionCall,
                    left: Box::new(Node::Primary(TokenType::Ident("bar".to_string()))),
                    right: Box::new(Node::Primary(TokenType::Ident("x".to_string()))),
                }),
            ]))),
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
        // func foo(num x): num {
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
                token_type: TokenType::Ampersand,                       // TODO change this to -> or : but fejer forgot to token
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
            node_1: Box::new(Node::Primary(TokenType::Ident("foo".to_string()))),
            node_2: Box::new(Node::Binary(BinaryNode {
                        operator: Operator::Declaration,
                        left: Box::new(Node::Primary(TokenType::Ident("num".to_string()))),
                        right: Box::new(Node::Primary(TokenType::Ident("x".to_string()))),
            })),
            node_3: Box::new(Node::Primary(TokenType::Ident("num".to_string()))),
            node_4: Box::new(Node::Block(Box::new(vec![
                Node::Unary(UnaryNode {
                    operator: Operator::Return,
                    operand: Box::new(Node::Binary(BinaryNode {
                        operator: Operator::Product,
                        left: Box::new(Node::Primary(TokenType::Ident("x".to_string()))),
                        right: Box::new(Node::Binary(BinaryNode {
                            operator: Operator::FunctionCall,
                            left: Box::new(Node::Primary(TokenType::Ident("bar".to_string()))),
                            right: Box::new(Node::Primary(TokenType::Number(2f64))),
                        })),
                    })),
                }),
            ]))),
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