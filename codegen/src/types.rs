use std::rc::Rc;
use parser::parser::Node;
use crate::context::CodeScope;

#[derive(Debug)]
pub struct Field {
    pub field_type: FieldType,
    pub modifier: FieldModifier,
    pub scope: CodeScope,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FieldType {
    Primitive(PrimitiveType),
    Struct(usize),
    Ident(Rc<Node>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum PrimitiveType {
    Number, String
}

#[derive(Debug)]
pub enum FieldModifier {
    None,
    List
}