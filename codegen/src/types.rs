use std::rc::Rc;
use parser::parser::Node;
use crate::context::CodeScope;

#[derive(Debug, Clone)]
pub struct Field {
    pub field_type: ValueType,
    pub scope: CodeScope,
}

#[derive(Clone, PartialEq, Debug)]
pub struct RuntimeVariable {
    pub variable: CodegenValue,
    pub name: String,
    pub param_ident: Option<u32>
}

#[derive(Clone, PartialEq, Debug)]
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
    Comptime(ComptimeType),
    Struct(usize),
    Ident(Rc<Node>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum RealtimeValueType {
    Primitive(PrimitiveType),
    Struct(usize),
}

impl RealtimeValueType {
    pub fn normalize(&self) -> ValueType {
        match self.clone() {
            RealtimeValueType::Primitive(primitive) => ValueType::Primitive(primitive),
            RealtimeValueType::Struct(struct_id) => ValueType::Struct(struct_id),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PrimitiveType {
    None, Number, String, Bool, // Realtime Types
}

#[derive(Clone, Debug, PartialEq)]
pub enum ComptimeType {
    Domain(usize), Function(usize), Type(RealtimeValueType) // Comptime Types
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
    Access
}

#[derive(Debug, Clone)]
pub enum CodegenBodyStackMode {
    None,
    Else,
    // ElseIf
}

#[derive(Clone, Debug, PartialEq)]
pub struct CodegenValue {
    pub ident: u32,
    pub value_type: ValueType
}

#[derive(Clone, Debug, PartialEq)]
pub struct CodegenTrace {
    pub root_ident: u32,
    pub crumbs: Vec<CodegenTraceCrumb>
}

#[derive(Clone, Debug, PartialEq)]
pub enum CodegenTraceCrumb {
    Index(usize)
}

#[derive(Clone, Debug, PartialEq)]
pub struct CodegenExpressionResult {
    pub value: CodegenValue,
    pub trace: Option<CodegenTrace>
}

impl CodegenValue {
    pub fn new(ident: u32, value_type: ValueType) -> Self {
        Self {
            ident,
            value_type
        }
    }

    pub fn domain(void_ident: u32, domain: usize) -> Self {
        Self {
            ident: void_ident,
            value_type: ValueType::Comptime(ComptimeType::Domain(domain))
        }
    }

    pub fn default() -> Self {
        Self::new(0, ValueType::Primitive(PrimitiveType::None))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CodegenRegisterGroup {
    pub name: String
}

pub enum CodegenAccessNode {
    Field(Rc<Node>),
    Function(Rc<Node>),
    Index(Rc<Node>)
}