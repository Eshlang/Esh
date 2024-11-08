use std::rc::Rc;
use lexer::types::{Token, TokenType};

// WIP!

pub enum SemanticInstruction {
    Primary(Rc<Token>),                                     // prim
    FunctionCall(SemanticFunctionCall),                 // func(expr)
    TypeConstruction(SemanticType, Rc<Node>),                    // type {block} 
    Declaration(Rc<Node>, Rc<Node>),                  // ident ident
    Return(Rc<Node>),                                  // return expr;
    Assignment(Rc<Node>, Rc<Node>),                   // decl/ident = expr;
    If(Rc<Node>, Rc<Node>),                           // if cond {block}
    Else(Rc<Node>, Rc<Node>),                         // stmt else {block}
    While(Rc<Node>, Rc<Node>),                        // while cond {block}
    Func(Rc<Node>, Rc<Node>, Rc<Node>, Rc<Node>),   // func ident (tuple/decl) -> tuple/ident {block}
    Struct(Rc<Node>, Rc<Node>),                       // struct ident {block}
    Block(Vec<Rc<Node>>),   
}


pub struct SemanticNode {
    pub token: Rc<Token>,
    
}
pub enum SemanticType {

}
pub struct SemanticVariable {
    pub token: Rc<Token>,
    pub var_type: SemanticType,
    pub var_id: SemanticVariableID
}
pub enum SemanticVariableID {
    Field(usize),
    Dynamic(usize),
}
pub struct SemanticExpression {
    pub token: Rc<Token>,
    pub expression_type: SemanticExpressionType,
}

pub enum SemanticExpressionType {
    Constant(TokenType),
    Variable(Rc<SemanticVariable>),
    Not(Rc<SemanticExpression>),                                                  // !expr
    Negative(Rc<SemanticExpression>),                                             // -expr
    Product(Rc<SemanticExpression>, Rc<SemanticExpression>),                      // expr * expr
    Quotient(Rc<SemanticExpression>, Rc<SemanticExpression>),                     // expr / expr
    Modulo(Rc<SemanticExpression>, Rc<SemanticExpression>),                       // expr % expr
    Sum(Rc<SemanticExpression>, Rc<SemanticExpression>),                          // expr + expr
    Difference(Rc<SemanticExpression>, Rc<SemanticExpression>),                   // expr - expr
    LessThan(Rc<SemanticExpression>, Rc<SemanticExpression>),                     // expr < expr
    GreaterThan(Rc<SemanticExpression>, Rc<SemanticExpression>),                  // expr > expr
    LessThanOrEqualTo(Rc<SemanticExpression>, Rc<SemanticExpression>),            // expr <= expr
    GreaterThanOrEqualTo(Rc<SemanticExpression>, Rc<SemanticExpression>),         // expr >= expr
    Equal(Rc<SemanticExpression>, Rc<SemanticExpression>),                        // expr == expr
    NotEqual(Rc<SemanticExpression>, Rc<SemanticExpression>),                     // expr != expr
    And(Rc<SemanticExpression>, Rc<SemanticExpression>),                          // expr && expr
    Or(Rc<SemanticExpression>, Rc<SemanticExpression>),                           // expr || expr
    Function(SemanticFunctionCall),                                               // func(expr)
}

pub struct SemanticFunctionCall {
    pub token: Rc<Token>,
    pub function: usize,
    pub expression: Rc<Vec<SemanticExpression>>
}
 
impl SemanticNode {

}