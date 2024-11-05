use std::collections::HashMap;

pub struct Field {
    pub field_type: FieldType
}

pub struct FieldMap {
    pub map: HashMap<String, Field>
}

pub enum FieldType {
    Primitive(PrimitiveType),
    Struct(usize),
}

pub enum PrimitiveType {
    Number, String
}