use std::rc::Rc;
use parser::parser::Node;
use crate::context::CodeScope;

#[derive(Debug, Clone)]
pub struct Field {
    pub field_type: ValueType,
    pub scope: CodeScope,
}

#[derive(Debug)]
pub struct RuntimeVariable {
    pub variable: CodegenValue,
    pub name: String,
    pub param_ident: Option<u32>
}

#[derive(Debug)]
pub enum RuntimeVariableIdent {
    Normal(u32),
    Field(usize, usize),
}

impl RuntimeVariable {
    pub fn new(variable: CodegenValue, name: String) -> Self {
        Self {
            variable,
            name,
            param_ident: None
        }
    }
    pub fn new_param(field_type: ValueType, name: String, param_and_var: (u32, u32)) -> Self {
        Self {
            variable: CodegenValue::new(param_and_var.1, field_type),
            name,
            param_ident: Some(param_and_var.0)
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ValueType {
    Primitive(PrimitiveType),
    Struct(usize),
    Ident(Rc<Node>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum PrimitiveType {
    None, Number, String, Bool
}


#[derive(Debug)]
pub enum CodegenExpressionStack<'a> {
    Node(&'a Rc<Node>),
    Calculate(CodegenExpressionType, u32, usize)
}

#[derive(Debug)]
pub enum CodegenExpressionType {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    NotEq,
    Not,
    Or,
    And,
    Greater,
    Lesser,
    GreaterEq,
    LesserEq,
}

#[derive(Debug, Clone)]
pub enum CodegenBodyStackMode {
    None,
    Else,
    // ElseIf
}

#[derive(Debug, Clone)]
pub enum CodegenFunctionAccess {
    Domain(usize),
    Variable(CodegenValue),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CodegenValue {
    pub ident: u32,
    pub value_type: ValueType
}

impl CodegenValue {
    pub fn new(ident: u32, value_type: ValueType) -> Self {
        Self {
            ident,
            value_type
        }
    }

    pub fn default() -> Self {
        Self::new(0, ValueType::Primitive(PrimitiveType::None))
    }
}