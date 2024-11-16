use std::rc::Rc;
use dfbin::enums::Instruction;
use parser::parser::Node;
use crate::context::CodeScope;

#[derive(Debug, Clone)]
pub struct Field {
    pub field_type: FieldType,
    pub scope: CodeScope,
}

#[derive(Debug)]
pub struct RuntimeVariable {
    pub field_type: FieldType,
    pub name: String,
    pub ident: u32,
    pub param_ident: Option<u32>
}

impl RuntimeVariable {
    pub fn new(field_type: FieldType, name: String, ident: u32) -> Self {
        Self {
            field_type,
            name,
            ident,
            param_ident: None
        }
    }
    pub fn new_param(field_type: FieldType, name: String, param_and_var: (u32, u32)) -> Self {
        Self {
            field_type,
            name,
            ident: param_and_var.1,
            param_ident: Some(param_and_var.0)
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum FieldType {
    Primitive(PrimitiveType),
    Struct(usize),
    Ident(Rc<Node>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum PrimitiveType {
    None, Number, String, Bool
}

#[derive(Debug)]
pub enum FieldModifier {
    None,
    List
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
    // FuncCall,
}