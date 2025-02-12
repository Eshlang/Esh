use std::rc::Rc;
use lexer::types::{Keyword, Token, TokenType, ValuedKeyword};

/// A syntactical node
#[derive(Debug, PartialEq)]
pub enum Node {
    None,                                                       // ()
    Primary(Rc<Token>),                                         // 0
    FunctionCall(Rc<Node>, Rc<Node>),                           // ident(tuple/expr)
    Access(Rc<Node>, Rc<Node>),                                 // ident.ident
    Construct(Rc<Node>, Rc<Node>),                              // ident {block} 
    Not(Rc<Node>),                                              // !expr
    Negative(Rc<Node>),                                         // -expr
    Product(Rc<Node>, Rc<Node>),                                // expr * expr
    Quotient(Rc<Node>, Rc<Node>),                               // expr / expr
    Modulo(Rc<Node>, Rc<Node>),                                 // expr % expr
    Sum(Rc<Node>, Rc<Node>),                                    // expr + expr
    Difference(Rc<Node>, Rc<Node>),                             // expr - expr
    LessThan(Rc<Node>, Rc<Node>),                               // expr < expr
    GreaterThan(Rc<Node>, Rc<Node>),                            // expr > expr
    LessThanOrEqualTo(Rc<Node>, Rc<Node>),                      // expr <= expr
    GreaterThanOrEqualTo(Rc<Node>, Rc<Node>),                   // expr >= expr
    Tuple(Vec<Rc<Node>>),                                       // (decl/expr, decl/expr, decl/expr)
    Equal(Rc<Node>, Rc<Node>),                                  // expr == expr
    NotEqual(Rc<Node>, Rc<Node>),                               // expr != expr
    And(Rc<Node>, Rc<Node>),                                    // expr && expr
    Or(Rc<Node>, Rc<Node>),                                     // expr || expr
    ListCall(Rc<Node>, Rc<Node>),                               // ident[expr]
    List(Vec<Rc<Node>>),                                        // [expr, expr, expr]
    Vector(Rc<Node>, Rc<Node>, Rc<Node>),                       // <expr, expr, expr>
    Location(Rc<Node>, Rc<Node>, Rc<Node>, Rc<Node>, Rc<Node>), // <expr, expr, expr, expr, expr>
    Declaration(Rc<Node>, Rc<Node>),                            // ident ident
    Break,                                                      // break;
    Return(Rc<Node>),                                           // return expr;
    Assignment(Rc<Node>, Rc<Node>),                             // decl/ident = expr;
    If(Rc<Node>, Rc<Node>),                                     // if cond {block}
    Else(Rc<Node>, Rc<Node>),                                   // stmt else {block}
    While(Rc<Node>, Rc<Node>),                                  // while cond {block}
    Func(Rc<Node>, Rc<Node>, Rc<Node>, Rc<Node>),               // func ident (tuple/decl) -> tuple/ident {block}
    Struct(Rc<Node>, Rc<Node>),                                 // struct ident {block}
    Domain(Rc<Node>, Rc<Node>),                                 // domain ident {block}
    Block(Vec<Rc<Node>>),                                       // stmt; stmt; stmt;
}

/// A parser error
#[derive(Debug, PartialEq)]
pub enum ParserError {
    InvalidToken(Rc<Token>),        // Token is not recognized
    InvalidStatement(Rc<Token>),    // Statement is not recognized
    MissingIdentifier(Rc<Token>),   // Expected an ident
    MissingSemicolon(Rc<Token>),    // Expected a semicolon
    MissingParenthesis(Rc<Token>),  // Expected opening/closing parenthesis
    MissingBracket(Rc<Token>),      // Expected opening/closing bracket
    MissingBrace(Rc<Token>),        // Expected opening/closing brace
    MissingAngleBracket(Rc<Token>), // Expected opening/closing angle bracket
}

/// Returns a [ParserError] if [self.curr()](Parser::curr()) does not match the input.
macro_rules! expect {
    ($self:expr, $token:pat) => {
        if $self.is_at_end() || match $self.curr().token_type {
            $token => false,
            _ => true
        } {
            return Err(if let $token = TokenType::Ident("".to_string()) {
                ParserError::MissingIdentifier($self.curr().clone())
            } else if let $token = TokenType::Semicolon {
                ParserError::MissingSemicolon($self.curr().clone())
            } else if let $token = TokenType::LParen {
                ParserError::MissingParenthesis($self.curr().clone())
            } else if let $token = TokenType::RParen {
                ParserError::MissingParenthesis($self.curr().clone())
            } else if let $token = TokenType::LBrace {
                ParserError::MissingBrace($self.curr().clone())
            } else if let $token = TokenType::RBrace {
                ParserError::MissingBrace($self.curr().clone())
            } else {
                ParserError::InvalidToken($self.curr().clone())
            })
        }
    }
}

pub struct Parser<'a> {
    tokens: &'a [Rc<Token>],
    current: usize,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a [Rc<Token>]) -> Self {
        Self {
            tokens: input,
            current: 0,
        }
    }

    pub fn parse(&mut self) -> Result<Node, ParserError> {
        self.statement_block()
    }

    /// Gets the current token
    pub(crate) fn curr(&self) -> &Rc<Token> {
        &self.tokens[self.current]
    }

    /// Gets the previous token
    pub(crate) fn prev(&self) -> &Rc<Token> {
        &self.tokens[self.current - 1]
    }

    /// Advances to the next token
    pub(crate) fn advance(&mut self) {
        self.current += 1;
    }

    /// If the current token is out of range
    pub(crate) fn is_at_end(&mut self) -> bool {
        self.current >= self.tokens.len()
    }

    /// Returns the current statement block
    pub(crate) fn statement_block(&mut self) -> Result<Node, ParserError> {
        let mut block = vec![];
        while !self.is_at_end() {
            if self.curr().token_type == TokenType::RBrace {
                break;
            }
            block.push(Rc::new(self.statement()?));
        }
        return Ok(Node::Block(block));
    }

    /// Returns the current statement
    pub(crate) fn statement(&mut self) -> Result<Node, ParserError> {
        match self.curr().token_type {
            TokenType::Ident(_) | TokenType::Keyword(Keyword::Value(_)) => {
                let expr = self.assignment();
                expect!(self, TokenType::Semicolon);
                self.advance();
                expr
            },
            TokenType::Keyword(Keyword::Struct) => {
                self.struct_statement()
            },
            TokenType::Keyword(Keyword::Domain) => {
                self.domain_statement()
            },
            TokenType::Keyword(Keyword::Func) => {
                self.func()
            },
            TokenType::Keyword(Keyword::If) => {
                self.if_else_block()
            },
            TokenType::Keyword(Keyword::While) => {
                self.while_block()
            },
            TokenType::Keyword(Keyword::Return) => {
                let expr = self.return_block();
                expect!(self, TokenType::Semicolon);
                self.advance();
                expr
            },
            TokenType::Keyword(Keyword::Break) => {
                self.advance();
                expect!(self, TokenType::Semicolon);
                self.advance();
                Ok(Node::Break)
            },
            _ => Err(ParserError::InvalidStatement(self.curr().clone()))
        }
    }

    /// Returns the current struct declaration statement
    pub(crate) fn struct_statement(&mut self) -> Result<Node, ParserError> {
        expect!(self, TokenType::Keyword(Keyword::Struct));
        let expr = Node::Struct(
            {  // Struct name
                self.advance();
                expect!(self, TokenType::Ident(_) | TokenType::Keyword(Keyword::Value(_)));
                Rc::new(self.ident()?)
            },
            {  // Struct body
                expect!(self, TokenType::LBrace);
                self.advance();
                Rc::new(self.statement_block()?)
            },
        );
        expect!(self, TokenType::RBrace);
        self.advance();
        return Ok(expr);
    }

    /// Returns the current domain declaration statement
    pub(crate) fn domain_statement(&mut self) -> Result<Node, ParserError> {
        expect!(self, TokenType::Keyword(Keyword::Domain));
        let expr = Node::Domain(
            {  // Domain name
                self.advance();
                expect!(self, TokenType::Ident(_) | TokenType::Keyword(Keyword::Value(_)));
                Rc::new(self.ident()?)
            },
            {  // Domain body
                expect!(self, TokenType::LBrace);
                self.advance();
                Rc::new(self.statement_block()?)
            },
        );
        expect!(self, TokenType::RBrace);
        self.advance();
        return Ok(expr);
    }

    /// Returns the current function declaration statement
    pub(crate) fn func(&mut self) -> Result<Node, ParserError> {
        expect!(self, TokenType::Keyword(Keyword::Func));
        let expr = Node::Func(
            {  // Function name
                self.advance();
                expect!(self, TokenType::Ident(_) | TokenType::Keyword(Keyword::Value(_)));
                Rc::new(self.ident()?)
            },
            {  // Function parameters
                expect!(self, TokenType::LParen);
                Rc::new(self.tuple()?)
            },
            {  // Return type
                match self.curr().token_type {
                    TokenType::Arrow => {
                        self.advance();
                        match self.curr().token_type {
                            TokenType::Ident(_) | TokenType::Keyword(Keyword::Value(_)) => Rc::new(self.ident()?),
                            _ => Rc::new(self.primary()?)
                        }
                    },
                    _ => Rc::new(Node::None)
                }
            },
            {  // Function body
                expect!(self, TokenType::LBrace);
                self.advance();
                Rc::new(self.statement_block()?)
            },
        );
        expect!(self, TokenType::RBrace);
        self.advance();
        return Ok(expr);
    }

    pub(crate) fn if_else_block(&mut self) -> Result<Node, ParserError> {
        expect!(self, TokenType::Keyword(Keyword::If));
        let mut expr = self.if_block()?;
        if self.is_at_end() {
            return Ok(expr);
        }
        if let TokenType::Keyword(Keyword::Else) = self.curr().token_type {
            self.advance();
            if self.is_at_end() {
                return Err(ParserError::MissingBrace(self.tokens[self.current - 1].clone()));
            }
            match self.curr().token_type {
                TokenType::Keyword(Keyword::If) => {
                    return Ok(Node::Else(
                        Rc::new(expr),
                        Rc::new(self.if_else_block()?)
                    ));
                },
                TokenType::LBrace => {
                    self.advance();
                    expr = Node::Else(
                        Rc::new(expr),
                        Rc::new(self.statement_block()?)
                    );
                },
                _ => return Err(ParserError::MissingBrace(self.curr().clone()))
            }
            expect!(self, TokenType::RBrace);
            self.advance();
        }
        return Ok(expr);
    }

    /// Returns the current if statement
    pub(crate) fn if_block(&mut self) -> Result<Node, ParserError> {
        expect!(self, TokenType::Keyword(Keyword::If));
        let expr = Node::If(
            {    // If statement expression
                self.advance();
                Rc::new(self.logic()?)
            },
            {   // If statement body
                expect!(self, TokenType::LBrace);
                self.advance();
                Rc::new(self.statement_block()?)
            },
        );
        expect!(self, TokenType::RBrace);
        self.advance();
        return Ok(expr);
    }

    /// Returns the current while statement
    pub(crate) fn while_block(&mut self) -> Result<Node, ParserError> {
        expect!(self, TokenType::Keyword(Keyword::While));
        let expr = Node::While(
            {    // While statement expression
                self.advance();
                Rc::new(self.logic()?)
            },
            {   // While statement body
                expect!(self, TokenType::LBrace);
                self.advance();
                Rc::new(self.statement_block()?)
            },
        );
        expect!(self, TokenType::RBrace);
        self.advance();
        return Ok(expr);
    }

    /// Returns the current assignment statement
    pub(crate) fn assignment(&mut self) -> Result<Node, ParserError> {
        let expr = self.declaration()?;
        if self.is_at_end() || self.curr().token_type != TokenType::Assign {
            return Ok(expr);
        }
        self.advance();
        return Ok(Node::Assignment(
            Rc::new(expr),
            Rc::new(self.expression()?),
        ));
    }

    /// Returns the current variable declaration
    pub(crate) fn declaration(&mut self) -> Result<Node, ParserError> {
        let start = self.current;
        match self.curr().token_type {
            TokenType::Ident(_) | TokenType::Keyword(Keyword::Value(_)) => (),
            _ => return self.expression(),
        }
        let expr = self.list_call()?;
        match self.curr().token_type {
            TokenType::Ident(_) | TokenType::Keyword(Keyword::Value(_)) => return Ok(Node::Declaration(
                Rc::new(expr),
                Rc::new(self.ident()?),
            )),
            _ => {
                self.current = start;
                return self.expression();
            }
        }
    }

    /// Returns the current return statement
    pub(crate) fn return_block(&mut self) -> Result<Node, ParserError> {
        self.advance();
        match self.curr().token_type {
            TokenType::Semicolon => Ok(Node::Return(Rc::new(Node::None))),
            _ => Ok(Node::Return(Rc::new(self.expression()?)))
        }
    }

    /// Returns the current expression
    pub(crate) fn expression(&mut self) -> Result<Node, ParserError> {
        self.logic()
    }

    /// Returns the current logic operation
    pub(crate) fn logic(&mut self) -> Result<Node, ParserError> {
        let mut expr = self.equality()?;
        while !self.is_at_end() {
            match self.curr().token_type {
                TokenType::And => {
                    self.advance();
                    expr = Node::And( 
                        Rc::new(expr), 
                        Rc::new(self.equality()?),
                    )
                },
                TokenType::Or => {
                    self.advance();
                    expr = Node::Or(
                        Rc::new(expr), 
                        Rc::new(self.equality()?),
                    )
                },
                _ => break
            }
        }
        return Ok(expr);
    }

    /// Returns the current equality
    pub(crate) fn equality(&mut self) -> Result<Node, ParserError> {
        let mut expr = self.comparison()?;
        while !self.is_at_end() {
            match self.curr().token_type {
                TokenType::Equal => {
                    self.advance();
                    expr = Node::Equal( 
                        Rc::new(expr), 
                        Rc::new(self.comparison()?),
                    )
                },
                TokenType::NotEqual => {
                    self.advance();
                    expr = Node::NotEqual(
                        Rc::new(expr), 
                        Rc::new(self.comparison()?),
                    )
                },
                _ => break
            }
        }
        return Ok(expr);
    }

    /// Returns the current comparison
    pub(crate) fn comparison(&mut self) -> Result<Node, ParserError> {
        let mut expr = self.term()?;
        while !self.is_at_end() {
            match self.curr().token_type {
                TokenType::LAngle => {
                    self.advance();
                    expr = Node::LessThan(
                        Rc::new(expr), 
                        Rc::new(self.term()?),
                    )
                },
                TokenType::RAngle => {
                    self.advance();
                    match self.curr().token_type {
                        TokenType::Ident(_) | TokenType::Number(_) | TokenType::Dash | TokenType::Keyword(Keyword::Value(_)) => expr = Node::GreaterThan(
                            Rc::new(expr), 
                            Rc::new(self.term()?),
                        ),
                        _ => {
                            self.current -= 1;
                            break;
                        }
                    }
                },
                TokenType::LTEqual => {
                    self.advance();
                    expr = Node::LessThanOrEqualTo(
                        Rc::new(expr), 
                        Rc::new(self.term()?),
                    )
                },
                TokenType::GTEqual => {
                    self.advance();
                    expr = Node::GreaterThanOrEqualTo( 
                        Rc::new(expr), 
                        Rc::new(self.term()?),
                    )
                },
                _ => break
            }
        }
        return Ok(expr);
    }

    /// Returns the current term operation
    pub(crate) fn term(&mut self) -> Result<Node, ParserError> {
        let mut expr = self.factor()?;
        while !self.is_at_end() {
            match self.curr().token_type {
                TokenType::Plus => {
                    self.advance();
                    expr = Node::Sum(
                        Rc::new(expr), 
                        Rc::new(self.factor()?),
                    )
                },
                TokenType::Dash => {
                    self.advance();
                    expr = Node::Difference(
                        Rc::new(expr), 
                        Rc::new(self.factor()?),
                    )
                },
                _ => break
            }
        }
        return Ok(expr);
    }

    /// Returns the current factor operation
    pub(crate) fn factor(&mut self) -> Result<Node, ParserError> {
        let mut expr = self.unary()?;
        while !self.is_at_end() {
            match self.curr().token_type {
                TokenType::Asterisk => {
                    self.advance();
                    expr = Node::Product(
                        Rc::new(expr), 
                        Rc::new(self.unary()?),
                    )
                },
                TokenType::Slash => {
                    self.advance();
                    expr = Node::Quotient(
                        Rc::new(expr), 
                        Rc::new(self.unary()?),
                    )
                },
                TokenType::Perc => {
                    self.advance();
                    expr = Node::Modulo(
                        Rc::new(expr), 
                        Rc::new(self.unary()?),
                    )
                },
                _ => break
            }
        }
        return Ok(expr);
    }

    /// Returns the current unary operation
    pub(crate) fn unary(&mut self) -> Result<Node, ParserError> {
        match self.curr().token_type {
            TokenType::Bang => {
                self.advance();
                Ok(Node::Not(Rc::new(self.unary()?)))
            },
            TokenType::Dash => {
                self.advance();
                Ok(Node::Negative(Rc::new(self.unary()?)))
            },
            _ => self.list_call(),
        }
    }

    /// Returns the current list call
    pub(crate) fn list_call(&mut self) -> Result<Node, ParserError> {
        match self.curr().token_type {
            TokenType::Ident(_) => (),
            _ => return self.primary(),
        }
        let start = self.current;
        let mut expr = self.ident()?;
        if self.is_at_end() {
            return Ok(expr)
        }
        match self.curr().token_type {
            TokenType::Ident(_) => return Ok(expr),
            TokenType::LBracket => (),
            _ => {
                self.current = start;
                return self.primary();
            }
        }
        while !self.is_at_end() {
            match &self.curr().token_type {
                TokenType::LBracket => {
                    self.advance();
                    match self.curr().token_type {
                        TokenType::RBracket => {
                            expr = Node::ListCall(
                                Rc::new(expr), 
                                Rc::new(Node::None)
                            );
                            self.advance();
                        }
                        _ => {
                            expr = Node::ListCall(
                                Rc::new(expr), 
                                Rc::new(self.expression()?)
                            );
                            if let TokenType::RBracket = self.curr().token_type {
                                self.advance();
                            } else {
                                return Err(ParserError::MissingBracket(self.curr().clone()))
                            }
                        }
                    }
                },
                _ => break
            }
        }
        return Ok(expr);
    }

    /// Returns the current primary node
    pub(crate) fn primary(&mut self) -> Result<Node, ParserError> {
        match self.curr().token_type {
            TokenType::Ident(_) | TokenType::Keyword(Keyword::Value(ValuedKeyword::SelfIdentity)) => {
                self.construct()
            },
            TokenType::Number(_) | TokenType::String(_) | TokenType::Keyword(Keyword::Value(_)) => {
                self.advance();
                Ok(Node::Primary(self.prev().clone()))
            },
            TokenType::LParen => {
                let start = self.current;
                self.advance();
                let expr = match self.curr().token_type {
                    TokenType::Ident(_) | TokenType::Keyword(Keyword::Value(_)) => self.declaration()?,
                    TokenType::RParen => {
                        self.advance();
                        return Ok(Node::None)
                    },
                    _ => self.expression()?
                };
                match self.curr().token_type {
                    TokenType::RParen => {
                        self.advance();
                        Ok(expr)
                    },
                    TokenType::Comma => {
                        self.current = start;
                        return self.tuple();
                    },
                    _ => Err(ParserError::MissingParenthesis(self.curr().clone()))
                }
            },
            TokenType::LBracket => {
                self.list()
            },
            TokenType::LAngle => {
                self.vector()
            },
            _ => {
                Err(ParserError::InvalidToken(self.curr().clone()))
            }
        }
    }

    /// Returns the current construct expression
    pub(crate) fn construct(&mut self) -> Result<Node, ParserError> {
        let mut expr = self.function_call()?;
        match self.curr().token_type {
            TokenType::LBrace => {
                expr = Node::Construct(
                    Rc::new(expr),
                    {  // Construct body
                        self.advance();
                        Rc::new(self.statement_block()?)
                    },
                );
                expect!(self, TokenType::RBrace);
                self.advance();
            },
            _ => ()
        }
        return Ok(expr);
    }

    /// Returns the current function call
    pub(crate) fn function_call(&mut self) -> Result<Node, ParserError> {
        let mut expr = self.access()?;
        match self.curr().token_type {
            TokenType::LParen => expr = Node::FunctionCall(
                    Rc::new(expr),
                    Rc::new(self.tuple()?),
                ),
            _ => ()
        }
        return Ok(expr);
    }

    /// Returns the current access chain
    pub(crate) fn access(&mut self) -> Result<Node, ParserError> {
        let mut expr = self.ident()?;
        while !self.is_at_end() {
            match self.curr().token_type {
                TokenType::Dot => {
                    self.advance();
                    expr = Node::Access(
                        Rc::new(expr), 
                        Rc::new(self.ident()?),
                    )
                },
                _ => break
            }
        }
        return Ok(expr);
    }

    /// Returns the current tuple
    pub(crate) fn tuple(&mut self) -> Result<Node, ParserError> {
        expect!(self, TokenType::LParen);
        self.advance();
        let mut block = vec![];
        if self.curr().token_type == TokenType::RParen {
            self.advance();
            return Ok(Node::Tuple(block));
        }
        while !self.is_at_end() {
            block.push(Rc::new(self.declaration()?));
            match self.curr().token_type {
                TokenType::Comma => (),
                TokenType::RParen => {
                    self.advance();
                    break;
                },
                _ => return Err(ParserError::MissingParenthesis(self.curr().clone()))
            }
            self.advance();
        }
        return Ok(Node::Tuple(block));
    }

    /// Returns the current list
    pub(crate) fn list(&mut self) -> Result<Node, ParserError> {
        expect!(self, TokenType::LBracket);
        self.advance();
        let mut block = vec![];
        if self.curr().token_type == TokenType::RBracket {
            self.advance();
            return Ok(Node::List(block));
        }
        while !self.is_at_end() {
            block.push(Rc::new(self.declaration()?));
            match self.curr().token_type {
                TokenType::Comma => (),
                TokenType::RBracket => {
                    self.advance();
                    break;
                },
                _ => return Err(ParserError::MissingBracket(self.curr().clone()))
            }
            self.advance();
        }
        return Ok(Node::List(block));
    }

    /// Returns the current vector or location
    pub(crate) fn vector(&mut self) -> Result<Node, ParserError> {
        expect!(self, TokenType::LAngle);
        self.advance();
        let expr1 = self.expression()?;
        expect!(self, TokenType::Comma);
        self.advance();
        let expr2 = self.expression()?;
        expect!(self, TokenType::Comma);
        self.advance();
        let expr3 = self.expression()?;
        match self.curr().token_type {
            TokenType::RAngle =>  {
                self.advance();
                return Ok(Node::Vector(
                    Rc::new(expr1), 
                    Rc::new(expr2), 
                    Rc::new(expr3),
                ));
            },
            TokenType::Comma => (),
            _ => return Err(ParserError::MissingAngleBracket(self.curr().clone()))
        }
        self.advance();
        let expr4 = self.expression()?;
        expect!(self, TokenType::Comma);
        self.advance();
        let expr5 = self.expression()?;
        expect!(self, TokenType::RAngle);
        self.advance();
        return Ok(Node::Location(
            Rc::new(expr1), 
            Rc::new(expr2), 
            Rc::new(expr3),
            Rc::new(expr4), 
            Rc::new(expr5),
        ));
    }

    /// Returns the current identifier
    pub(crate) fn ident(&mut self) -> Result<Node, ParserError> {
        expect!(self, TokenType::Ident(_) | TokenType::Keyword(Keyword::Value(_)));
        self.advance();
        Ok(Node::Primary(self.prev().clone()))
    }
}