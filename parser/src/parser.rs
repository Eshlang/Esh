use lexer::types::{Keyword, Token, TokenType};

/// A syntactical node
#[derive(Debug, PartialEq)]
pub enum Node {
    None,
    Primary(TokenType),
    FunctionCall(Box<Node>, Box<Node>),
    Not(Box<Node>),
    Negative(Box<Node>),
    Product(Box<Node>, Box<Node>),
    Quotient(Box<Node>, Box<Node>),
    Modulo(Box<Node>, Box<Node>),
    Sum(Box<Node>, Box<Node>),
    Difference(Box<Node>, Box<Node>),
    LessThan(Box<Node>, Box<Node>),
    GreaterThan(Box<Node>, Box<Node>),
    LessThanOrEqualTo(Box<Node>, Box<Node>),
    GreaterThanOrEqualTo(Box<Node>, Box<Node>),
    Tuple(Vec<Node>),
    Equal(Box<Node>, Box<Node>),
    NotEqual(Box<Node>, Box<Node>),
    Declaration(Box<Node>, Box<Node>),
    Return(Box<Node>),
    Assignment(Box<Node>, Box<Node>),
    If(Box<Node>, Box<Node>),
    Else(Box<Node>, Box<Node>),
    While(Box<Node>, Box<Node>),
    Func(Box<Node>, Box<Node>, Box<Node>, Box<Node>),
    Struct(Box<Node>, Box<Node>),
    Block(Vec<Node>),
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
            if self.is_at_end() || *self.curr() == TokenType::RBrace {
                break;
            }
            block.push(self.statement()?);
            if self.is_at_end() || *self.curr() == TokenType::RBrace {
                break;
            }
            expect!(self, TokenType::Semicolon);
            self.advance();
        }
        return Ok(Node::Block(block));
    }

    /// Returns the current statement
    fn statement(&mut self) -> Result<Node, ParserError> {
        match self.curr() {
            TokenType::Ident(_) => {
                self.assignment()
            },
            TokenType::Keyword(Keyword::Struct) => {
                self.struct_statement()
            },
            TokenType::Keyword(Keyword::Func) => {
                self.func()
            },
            TokenType::Keyword(Keyword::If) => {
                self.if_else_block()
            },
            TokenType::Keyword(Keyword::Return) => {
                self.return_block()
            },
            _ => Err(ParserError::InvalidStatement)
        }
    }

    /// Returns the current function declaration statement
    fn struct_statement(&mut self) -> Result<Node, ParserError> {
        expect!(self, TokenType::Keyword(Keyword::Struct));
        let expr = Node::Struct(
            {  // Struct name
                self.advance();
                expect!(self, TokenType::Ident(_));
                Box::new(self.ident()?)
            },
            {  // Struct body
                expect!(self, TokenType::LBrace);
                self.advance();
                Box::new(self.statement_block()?)
            },
        );
        expect!(self, TokenType::RBrace);
        self.advance();
        return Ok(expr);
    }

    /// Returns the current function declaration statement
    fn func(&mut self) -> Result<Node, ParserError> {
        expect!(self, TokenType::Keyword(Keyword::Func));
        let expr = Node::Func(
            {  // Function name
                self.advance();
                expect!(self, TokenType::Ident(_));
                Box::new(self.ident()?)
            },
            {  // Function parameters
                expect!(self, TokenType::LParen);
                Box::new(self.primary()?)
            },
            {  // Return type
                if *self.curr() == TokenType::Arrow {
                    self.advance();
                    Box::new(self.primary()?)
                } else {
                    Box::new(Node::None)
                }
            },
            {  // Function body
                expect!(self, TokenType::LBrace);
                self.advance();
                Box::new(self.statement_block()?)
            },
        );
        expect!(self, TokenType::RBrace);
        self.advance();
        return Ok(expr);
    }

    fn if_else_block(&mut self) -> Result<Node, ParserError> {
        expect!(self, TokenType::Keyword(Keyword::If));
        let mut expr = self.if_block()?;
        while !self.is_at_end() {
            match self.curr() {
                TokenType::Keyword(Keyword::Else) => {
                    self.advance();
                    expr = Node::Else(
                        Box::new(expr),  
                        match self.curr() {
                            TokenType::Keyword(Keyword::If) => Box::new(self.if_else_block()?),
                            TokenType::LBrace => {
                                self.advance();
                                Box::new(self.statement_block()?)
                            },
                            _ => return Err(ParserError::MissingBrace)
                        },
                    );
                    expect!(self, TokenType::RBrace);
                    self.advance();
                },
                _ => break
            }
        }
        return Ok(expr);
    }

    /// Returns the current if statement
    fn if_block(&mut self) -> Result<Node, ParserError> {
        expect!(self, TokenType::Keyword(Keyword::If));
        let expr = Node::If(
            {    // If statement expression
                self.advance();
                Box::new(self.equality()?)
            },
            {   // If statement body
                expect!(self, TokenType::LBrace);
                self.advance();
                Box::new(self.statement_block()?)
            },
        );
        expect!(self, TokenType::RBrace);
        self.advance();
        return Ok(expr);
    }

    /// Returns the current assignment statement
    fn assignment(&mut self) -> Result<Node, ParserError> {
        let mut expr = self.declaration()?;
        if !self.is_at_end() && *self.curr() == TokenType::Assign {
            self.advance();
            expr = Node::Assignment(
                Box::new(expr),
                Box::new(self.expression()?),
            );
        }
        return Ok(expr);
    }

    /// Returns the current variable declaration
    fn declaration(&mut self) -> Result<Node, ParserError> {
        match self.curr() {
            TokenType::Ident(_) => (),
            _ => return self.expression()
        }
        let expr = self.expression()?;
        if let TokenType::Ident(_) = self.curr() {
            return Ok(Node::Declaration(
                Box::new(expr),
                Box::new(self.ident()?),
            ));
        }
        return Ok(expr);
    }

    /// Returns the current return statement
    fn return_block(&mut self) -> Result<Node, ParserError> {
        self.advance();
        return Ok(Node::Return(Box::new(self.expression()?)))
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
                    expr = Node::Equal( 
                        Box::new(expr), 
                        Box::new(self.comparison()?),
                    )
                },
                TokenType::NotEqual => {
                    self.advance();
                    expr = Node::NotEqual(
                        Box::new(expr), 
                        Box::new(self.comparison()?),
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
                    expr = Node::LessThan(
                        Box::new(expr), 
                        Box::new(self.term()?),
                    )
                },
                TokenType::RAngle => {
                    self.advance();
                    expr = Node::GreaterThan(
                        Box::new(expr), 
                        Box::new(self.term()?),
                    )
                },
                TokenType::LTEqual => {
                    self.advance();
                    expr = Node::LessThanOrEqualTo(
                        Box::new(expr), 
                        Box::new(self.term()?),
                    )
                },
                TokenType::GTEqual => {
                    self.advance();
                    expr = Node::GreaterThanOrEqualTo( 
                        Box::new(expr), 
                        Box::new(self.term()?),
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
                    expr = Node::Sum(
                        Box::new(expr), 
                        Box::new(self.factor()?),
                    )
                },
                TokenType::Dash => {
                    self.advance();
                    expr = Node::Difference(
                        Box::new(expr), 
                        Box::new(self.factor()?),
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
                    expr = Node::Product(
                        Box::new(expr), 
                        Box::new(self.unary()?),
                    )
                },
                TokenType::Slash => {
                    self.advance();
                    expr = Node::Quotient(
                        Box::new(expr), 
                        Box::new(self.unary()?),
                    )
                },
                TokenType::Perc => {
                    self.advance();
                    expr = Node::Modulo(
                        Box::new(expr), 
                        Box::new(self.unary()?),
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
                Ok(Node::Not(Box::new(self.unary()?)))
            },
            TokenType::Dash => {
                self.advance();
                Ok(Node::Negative(Box::new(self.unary()?)))
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
                Ok(Node::Primary(self.prev().clone()))
            },
            TokenType::LParen => {
                let start = self.current;
                self.advance();
                let expr = match self.curr() {
                    TokenType::Ident(_) => self.declaration()?,
                    TokenType::RParen => {
                        self.advance();
                        return Ok(Node::None)
                    },
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
            TokenType::LParen => expr = Node::FunctionCall(
                    Box::new(expr),
                    Box::new(self.primary()?),
                ),
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
        return Ok(Node::Tuple(block));
    }

    /// Returns the current identifier
    fn ident(&mut self) -> Result<Node, ParserError> {
        expect!(self, TokenType::Ident(_));
        self.advance();
        Ok(Node::Primary(self.prev().clone()))
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
        let expected = Node::Sum(
            Box::new(Node::Primary(TokenType::Ident("x".to_string()))),
            Box::new(Node::Product(
                Box::new(Node::Quotient(
                    Box::new(Node::Primary(TokenType::Number(8f64))),
                    Box::new(Node::Primary(TokenType::Number(2f64))),
                )),
                Box::new(Node::Primary(TokenType::Number(4f64))),
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
        let expected = Node::Quotient(
            Box::new(Node::Sum(
                Box::new(Node::Primary(TokenType::Ident("x".to_string()))),
                Box::new(Node::Primary(TokenType::Number(8f64)))
            )),
            Box::new(Node::Product(
                Box::new(Node::Primary(TokenType::Number(2f64))),
                Box::new(Node::Primary(TokenType::Number(4f64)))
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
        let expected = Node::Tuple(vec![
            Node::Primary(TokenType::Ident("x".to_string())),
            Node::Primary(TokenType::Number(3f64)),
            Node::Primary(TokenType::String("test".to_string())),
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
        let expected = Node::Assignment(
            Box::new(Node::Declaration(
                Box::new(Node::Primary(TokenType::Ident("num".to_string()))),
                Box::new(Node::Primary(TokenType::Ident("x".to_string()))),
            )),
            Box::new(Node::Primary(TokenType::Number(5f64))),
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
            Token {
                token_type: TokenType::Ident("num".to_string()),
                range: Range::new((0, 0), (0, 2)),
            },
            Token {
                token_type: TokenType::Ident("x".to_string()),
                range: Range::new((0, 4), (0, 4)),
            },
            Token {
                token_type: TokenType::Semicolon,
                range: Range::new((0, 5), (0, 5)),
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
                range: Range::new((1, 15), (1, 15)),
            },
        ];
        let expected = Node::Block(vec![
            Node::Declaration(
                Box::new(Node::Primary(TokenType::Ident("num".to_string()))),
                Box::new(Node::Primary(TokenType::Ident("x".to_string()))),
            ),
            Node::Assignment(
                Box::new(Node::Declaration(
                    Box::new(Node::Primary(TokenType::Ident("str".to_string()))),
                    Box::new(Node::Primary(TokenType::Ident("y".to_string()))),
                )),
                Box::new(Node::Primary(TokenType::String("hello".to_string()))),
            ),
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
        //    str y = "hello";
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
        let expected = Node::If(
            Box::new(Node::Equal(
                Box::new(Node::Primary(TokenType::Ident("x".to_string()))),
                Box::new(Node::Primary(TokenType::Number(5f64))),
            )),
            Box::new(Node::Block(vec![
                Node::Assignment(
                    Box::new(Node::Declaration(
                        Box::new(Node::Primary(TokenType::Ident("str".to_string()))),
                        Box::new(Node::Primary(TokenType::Ident("y".to_string()))),
                    )),
                    Box::new(Node::Primary(TokenType::Ident("hello".to_string()))),
                ),
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
            Token {
                token_type: TokenType::Keyword(Keyword::Else),
                range: Range::new((2, 2), (2, 5)),
            },
            Token {
                token_type: TokenType::LBrace,
                range: Range::new((2, 7), (2, 7)),
            },
            Token {
                token_type: TokenType::Ident("str".to_string()),
                range: Range::new((3, 0), (3, 2)),
            },
            Token {
                token_type: TokenType::Ident("y".to_string()),
                range: Range::new((3, 4), (3, 4)),
            },
            Token {
                token_type: TokenType::Assign,
                range: Range::new((3, 6), (3, 6)),
            },
            Token {
                token_type: TokenType::Ident("evil hello".to_string()),
                range: Range::new((3, 8), (3, 19)),
            },
            Token {
                token_type: TokenType::Semicolon,
                range: Range::new((3, 20), (3, 20)),
            },
            Token {
                token_type: TokenType::RBrace,
                range: Range::new((4, 0), (4, 0)),
            },
        ];
        let expected = Node::Else(
            Box::new(Node::If(
                Box::new(Node::Equal(
                    Box::new(Node::Primary(TokenType::Ident("x".to_string()))),
                    Box::new(Node::Primary(TokenType::Number(5f64))),
                )),
                Box::new(Node::Block(vec![
                    Node::Assignment(
                        Box::new(Node::Declaration(
                            Box::new(Node::Primary(TokenType::Ident("str".to_string()))),
                            Box::new(Node::Primary(TokenType::Ident("y".to_string()))),
                        )),
                        Box::new(Node::Primary(TokenType::Ident("hello".to_string()))),
                    ),
                ])),
            )),
            Box::new(Node::Block(vec![
                Node::Assignment(
                    Box::new(Node::Declaration(
                        Box::new(Node::Primary(TokenType::Ident("str".to_string()))), 
                        Box::new(Node::Primary(TokenType::Ident("y".to_string()))),
                    )), 
                    Box::new(Node::Primary(TokenType::Ident("evil hello".to_string()))),
                )
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
        let expected = Node::Func(
            Box::new(Node::Primary(TokenType::Ident("foo".to_string()))),
            Box::new(Node::Declaration(
                Box::new(Node::Primary(TokenType::Ident("num".to_string()))),
                Box::new(Node::Primary(TokenType::Ident("x".to_string()))),
            )),
            Box::new(Node::None),
            Box::new(Node::Block(vec![
                Node::FunctionCall(
                    Box::new(Node::Primary(TokenType::Ident("bar".to_string()))),
                    Box::new(Node::Primary(TokenType::Ident("x".to_string()))),
                ),
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
        let expected = Node::Func(
            Box::new(Node::Primary(TokenType::Ident("foo".to_string()))),
            Box::new(Node::Declaration(
                Box::new(Node::Primary(TokenType::Ident("num".to_string()))),
                Box::new(Node::Primary(TokenType::Ident("x".to_string()))),
            )),
            Box::new(Node::Primary(TokenType::Ident("num".to_string()))),
            Box::new(Node::Block(vec![
                Node::Return(
                    Box::new(Node::Product(
                        Box::new(Node::Primary(TokenType::Ident("x".to_string()))),
                        Box::new(Node::FunctionCall(
                            Box::new(Node::Primary(TokenType::Ident("bar".to_string()))),
                            Box::new(Node::Primary(TokenType::Number(2f64))),
                        )),
                    )),
                ),
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
        let input = [
            Token {
                token_type: TokenType::Keyword(Keyword::Struct),
                range: Range::new((0, 0), (0, 5)),
            },
            Token {
                token_type: TokenType::Ident("foo".to_string()),
                range: Range::new((0, 7), (0, 9)),
            },
            Token {
                token_type: TokenType::LBrace,
                range: Range::new((0, 11), (0, 11)),
            },
            Token {
                token_type: TokenType::Ident("num".to_string()),
                range: Range::new((1, 0), (1, 2)),
            },
            Token {
                token_type: TokenType::Ident("x".to_string()),
                range: Range::new((1, 4), (1, 4)),
            },
            Token {
                token_type: TokenType::Semicolon,
                range: Range::new((1, 5), (1, 5)),
            },
            Token {
                token_type: TokenType::Ident("str".to_string()),
                range: Range::new((2, 0), (2, 2)),
            },
            Token {
                token_type: TokenType::Ident("y".to_string()),
                range: Range::new((2, 4), (2, 4)),
            },
            Token {
                token_type: TokenType::Semicolon,
                range: Range::new((2, 5), (2, 5)),
            },
            Token {
                token_type: TokenType::Keyword(Keyword::Func),
                range: Range::new((3, 0), (3, 3)),
            },
            Token {
                token_type: TokenType::Ident("bar".to_string()),
                range: Range::new((3, 5), (3, 7)),
            },
            Token {
                token_type: TokenType::LParen,
                range: Range::new((3, 8), (3, 8)),
            },
            Token {
                token_type: TokenType::RParen,
                range: Range::new((3, 9), (3, 9)),
            },
            Token {
                token_type: TokenType::Arrow,
                range: Range::new((3, 11), (3, 12)),
            },
            Token {
                token_type: TokenType::LParen,
                range: Range::new((3, 14), (3, 14)),
            },
            Token {
                token_type: TokenType::Ident("num".to_string()),
                range: Range::new((3, 15), (3, 17)),
            },
            Token {
                token_type: TokenType::Comma,
                range: Range::new((3, 18), (3, 18)),
            },
            Token {
                token_type: TokenType::Ident("str".to_string()),
                range: Range::new((3, 19), (3, 21)),
            },
            Token {
                token_type: TokenType::RParen,
                range: Range::new((3, 22), (3, 22)),
            },
            Token {
                token_type: TokenType::LBrace,
                range: Range::new((3, 24), (3, 24)),
            },
            Token {
                token_type: TokenType::Keyword(Keyword::Return),
                range: Range::new((4, 0), (4, 5)),
            },
            Token {
                token_type: TokenType::LParen,
                range: Range::new((4, 7), (4, 7)),
            },
            Token {
                token_type: TokenType::Ident("x".to_string()),
                range: Range::new((4, 8), (4, 8)),
            },
            Token {
                token_type: TokenType::Comma,
                range: Range::new((4, 9), (4, 9)),
            },
            Token {
                token_type: TokenType::Ident("y".to_string()),
                range: Range::new((4, 11), (4, 11)),
            },
            Token {
                token_type: TokenType::RParen,
                range: Range::new((4, 12), (4, 12)),
            },
            Token {
                token_type: TokenType::Semicolon,
                range: Range::new((4, 13), (4, 13)),
            },
            Token {
                token_type: TokenType::RBrace,
                range: Range::new((5, 0), (5, 0)),
            },
            Token {
                token_type: TokenType::RBrace,
                range: Range::new((6, 0), (6, 0)),
            },
        ];
        let expected = Node::Struct(
            Box::new(Node::Primary(TokenType::Ident("foo".to_string()))), 
            Box::new(Node::Block(vec![
                Node::Declaration(
                    Box::new(Node::Primary(TokenType::Ident("num".to_string()))), 
                    Box::new(Node::Primary(TokenType::Ident("x".to_string()))),
                ), 
                Node::Declaration(
                    Box::new(Node::Primary(TokenType::Ident("str".to_string()))), 
                    Box::new(Node::Primary(TokenType::Ident("y".to_string()))),
                ), 
                Node::Func(
                    Box::new(Node::Primary(TokenType::Ident("bar".to_string()))), 
                    Box::new(Node::None), 
                    Box::new(Node::Tuple(vec![
                        Node::Primary(TokenType::Ident("num".to_string())), 
                        Node::Primary(TokenType::Ident("str".to_string())),
                    ])), 
                    Box::new(Node::Block(vec![
                        Node::Return(
                            Box::new(Node::Tuple(vec![
                                Node::Primary(TokenType::Ident("x".to_string())), 
                                Node::Primary(TokenType::Ident("y".to_string())),
                            ])),
                        ),
                    ])),
                ),
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
}