use lexer::{types::Token, types::TokenType};

#[derive(Debug, PartialEq)]
pub enum Precedence {
    Primary = 0,
    Unary = 1,
    Factor = 2,
    Term = 3,
    Comparison = 4,
    Equality = 5,
    Expression = 6,
}

#[derive(Debug, PartialEq)]
pub enum Node {
    Primary(Precedence, TokenType),
    Unary(Precedence, UnaryNode),
    Binary(Precedence, BinaryNode),
}

#[derive(Debug, PartialEq)]
pub struct UnaryNode {
    operator: TokenType,
    operand: Box<Node>,
}

#[derive(Debug, PartialEq)]
pub struct BinaryNode {
    operator: TokenType,
    left: Box<Node>,
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

    fn advance(&mut self) {
        if !self.is_at_end() {
            self.current += 1;
        }
    }

    fn is_at_end(&mut self) -> bool {
        self.current == self.tokens.len()
    }

    fn matches(&mut self, tokens: &[TokenType]) -> bool {
        for token in tokens {
            if !self.is_at_end() && &self.curr() == token {
                self.current += 1;
                return true;
            }
        }
        return false;
    }

    fn expression(&mut self) -> Node {
        self.equality()
    }

    fn equality(&mut self) -> Node {
        let mut expr = self.comparison();
        while self.matches(&[TokenType::Equal, TokenType::NotEqual]) {
            expr = Node::Binary(
                Precedence::Equality,
                BinaryNode {
                    operator: self.prev(), 
                    left: Box::new(expr), 
                    right: Box::new(self.comparison()),
                }
            )
        }
        return expr;
    }

    fn comparison(&mut self) -> Node {
        let mut expr = self.term();
        while self.matches(&[TokenType::LAngle, TokenType::RAngle, TokenType::LTEqual, TokenType::GTEqual]) {
            expr = Node::Binary(
                Precedence::Comparison,
                BinaryNode {
                    operator: self.prev(), 
                    left: Box::new(expr), 
                    right: Box::new(self.term()),
                }
            )
        }
        return expr;
    }

    fn term(&mut self) -> Node {
        let mut expr = self.factor();
        while self.matches(&[TokenType::Plus, TokenType::Dash]) {
            expr = Node::Binary(
                Precedence::Term,
                BinaryNode {
                    operator: self.prev(), 
                    left: Box::new(expr), 
                    right: Box::new(self.factor()),
                }
            )
        }
        return expr;
    }

    fn factor(&mut self) -> Node {
        let mut expr = self.unary();
        while self.matches(&[TokenType::Asterisk, TokenType::Slash]) {
            expr = Node::Binary(
                Precedence::Factor,
                BinaryNode {
                    operator: self.prev(), 
                    left: Box::new(expr), 
                    right: Box::new(self.unary()),
                }
            )
        }
        return expr;
    }

    fn unary(&mut self) -> Node {
        match self.curr() {
            TokenType::Bang | TokenType::Dash => {
                self.current += 1;
                Node::Unary(
                    Precedence::Unary, 
                    UnaryNode {
                        operator: self.prev(),
                        operand: Box::new(self.unary()),
                    }
                )
            },
            _ => self.primary(),
        }
    }

    fn primary(&mut self) -> Node {
        match self.curr() {
            TokenType::LParen => {
                self.current += 1;
                todo!("implement parenthesis")
            },
            _ => {
                self.current += 1;
                Node::Primary(Precedence::Primary, self.prev())
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
        // "5 + 8 / 2"
        let input = [
            Token {
                token_type: TokenType::Number(5f64),
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
        ];
        let expected = Node::Binary(
            Precedence::Term, 
            BinaryNode {
                operator: TokenType::Plus,
                left: Box::new(
                    Node::Primary(
                        Precedence::Primary, 
                        TokenType::Number(5f64),
                    ),
                ),
                right: Box::new(
                    Node::Binary(
                        Precedence::Factor,
                        BinaryNode {
                            operator: TokenType::Slash,
                            left: Box::new(
                                Node::Primary(
                                    Precedence::Primary,
                                    TokenType::Number(8f64),
                                ),
                            ),
                            right: Box::new(
                                Node::Primary(
                                    Precedence::Primary, 
                                    TokenType::Number(2f64),
                                ),
                            ),
                        }
                    ),
                ),
            }
        );
        let mut parser = Parser::new(&input);
        let output = parser.expression();
        assert_eq!(output, expected);
    }
}