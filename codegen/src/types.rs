use std::rc::Rc;
use esh_parser::parser::Node;
use crate::context::CodeScope;

#[derive(Clone, PartialEq, Debug)]
pub struct Field {
    pub field_type: ValueType,
    pub scope: CodeScope,
}

#[derive(Clone, PartialEq, Debug)]
pub struct FieldDefinition {
    pub field: Field,
    pub index: usize
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

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum IdentifierCategory {
    Field,
    RuntimeVariable,
    Domain,
    Function,
    Type,
    SelfFunction
}

impl ValueType {
    pub fn is_realtime(&self) -> bool {
        match self {
            Self::Primitive(..) => true,
            Self::Comptime(..) => false,
            Self::Struct(..) => true,
            Self::Ident(..) => { panic!("An Ident valuetype is only for the scanning phase and should not be checked for realtime/comptime (code generation related checks)"); }
        }
    }
    pub fn is_comptime(&self) -> bool {
        match self {
            Self::Primitive(..) => false,
            Self::Comptime(..) => true,
            Self::Struct(..) => false,
            Self::Ident(..) => { panic!("An Ident valuetype is only for the scanning phase and should not be checked for realtime/comptime (code generation related checks)"); }
        }
    }
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
    None, Number, String, Bool, List(Rc<ValueType>), Map(Rc<ValueType>, Rc<ValueType>), Vector, Location, Sound, Particle, Potion, Item // Realtime Types
}

#[derive(Clone, Debug, PartialEq)]
pub enum ComptimeType {
    Domain(usize), Function(usize), Type(RealtimeValueType), SelfFunction(usize, u32) // Comptime Types
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

impl CodegenTrace {
    pub fn root(ident: u32) -> Self {
        Self {
            root_ident: ident,
            crumbs: Vec::new()
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CodegenTraceCrumb {
    Ident(CodegenTraceCrumbIdent), IndexNode(Rc<Node>), EntryNode(Rc<Node>)
}

#[derive(Clone, Debug, PartialEq)]
pub enum CodegenTraceCrumbIdent {
    Index(u32), Entry(u32), // Node based ones
    IndexDirect(usize), // Direct ones
    Location(CodegenLocationCoordinate),
    Vector(CodegenVectorCoordinate),
}

#[derive(Clone, Debug, PartialEq)]
pub enum CodegenLocationCoordinate {
    X, Y, Z, Pitch, Yaw
}

#[derive(Clone, Debug, PartialEq)]
pub enum CodegenVectorCoordinate {
    X, Y, Z
}

#[derive(Clone, Debug, PartialEq)]
pub struct CodegenExpressionResult {
    pub value: CodegenValue,
    pub trace: Option<CodegenTrace>
}

impl CodegenExpressionResult {
    pub fn value(value: CodegenValue) -> Self {
        Self {
            value,
            trace: None
        }
    }
    pub fn trace(value: CodegenValue, trace: CodegenTrace) -> Self {
        Self {
            value,
            trace: Some(trace)
        }
    }
}

impl CodegenValue {
    pub fn new(ident: u32, value_type: ValueType) -> Self {
        Self {
            ident,
            value_type
        }
    }

    pub fn comptime(void_ident: u32, value: ComptimeType) -> Self {
        Self {
            ident: void_ident,
            value_type: ValueType::Comptime(value)
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

#[derive(Clone, Debug, PartialEq)]
/// Settings for the ``generate_expression`` function - if set_ident and register_group both aren't provided, and save_value is true, and variable_necessary is true / the expression isn't primary, it will generate a new infinitely-lasting register that you'll have to free manually.
/// This is unrecommended behavior.
pub struct GenerateExpressionSettings {
    /// Makes an effort to save the value with codeblocks. if this is false,
    /// no codeblocks will be generated by the expression and it'll just evaluate its end type.
    /// The identifier the expression returns is the void constant if this is true.
    /// Useful for comptime evaluations.
    pub generate_codeblocks: bool,

    /// Optionally choose an identifier that the expression's result ends up in.
    pub set_ident: Option<u32>,

    /// Optionally give the expression a register group it can use to generate an
    /// out-of-scope-lasting register that you can free in the using code.
    /// This won't be used if set_ident is provided.
    pub register_group: Option<u64>,

    /// This option marks if the expression necessarily needs to return a variable or if it can return an identifier to a value.
    pub variable_necessary: bool,

    /// This is used internally by the ``generate_expression`` function to denote the depth of an expression layer.
    pub depth: usize,

    /// If this is set to a value type, the expression will, at the end of the result,
    /// try to implicitly cast it into that type, and if it can't - it'll error.
    pub expected_type: Option<ValueType>,

    pub preferred_category: IdentifierCategory,
}

impl GenerateExpressionSettings {
    pub fn ident(ident: u32) -> Self {
        Self {
            generate_codeblocks: true,
            set_ident: Some(ident),
            register_group: None,
            variable_necessary: true,
            expected_type: None,
            preferred_category: IdentifierCategory::RuntimeVariable,
            depth: 0,
        }
    }
    pub fn group(group: u64) -> Self {
        Self {
            generate_codeblocks: true,
            set_ident: None,
            register_group: Some(group),
            variable_necessary: true,
            expected_type: None,
            preferred_category: IdentifierCategory::RuntimeVariable,
            depth: 0,
        }
    }
    pub fn comptime() -> Self {
        Self {
            generate_codeblocks: false,
            set_ident: None,
            register_group: None,
            variable_necessary: false,
            expected_type: None,
            preferred_category: IdentifierCategory::RuntimeVariable,
            depth: 0,
        }
    }

    /// Like ``group()`` but meant for parameters (``variable_necessary`` is false)
    /// This is used when the value you want out of the expression is placed in a chest (e.g in the usages of construct and function)
    /// Meanwhile a variable output IS necessary if you want to, for example, set the output to a variable (like in assignment)
    /// This is pretty specific.
    pub fn parameter(group: u64) -> Self {
        Self {
            generate_codeblocks: true,
            set_ident: None,
            register_group: Some(group),
            variable_necessary: false,
            expected_type: None,
            preferred_category: IdentifierCategory::RuntimeVariable,
            depth: 0,
        }
    }

    
    /// Used for when you want to execute an expression as a function.
    /// Recommended to put the ``constant_void()`` register in the ``ident`` parameter.
    pub fn void(ident: u32) -> Self {
        Self {
            generate_codeblocks: true,
            set_ident: Some(ident),
            register_group: None,
            variable_necessary: false,
            expected_type: None,
            preferred_category: IdentifierCategory::RuntimeVariable,
            depth: 0,
        }
    }

    /// This is used internally by the ``generate_expression`` function to pass the settings onto other branches of the expressions.
    pub fn pass(&self) -> Self {
        let mut passed = self.clone();
        passed.depth += 1;
        passed.expected_type = None;
        passed.preferred_category = IdentifierCategory::RuntimeVariable;
        passed
    }

    /// Combination of ``.pass`` and ``.prefer_category``
    pub fn pass_prefer(&self, category: IdentifierCategory) -> Self {
        let mut passed = self.clone();
        passed.depth += 1;
        passed.expected_type = None;
        passed.preferred_category = category;
        passed
    }

    /// Copies the ``generate_codeblocks`` field from self.
    pub fn keep_comptime(&self, settings: &GenerateExpressionSettings) -> Self {
        let mut changed = self.clone();
        changed.generate_codeblocks = settings.generate_codeblocks;
        changed
    }

    /// Makes the expression expect a type.
    pub fn expect_type(&self, value_type: &ValueType) -> Self {
        let mut changed = self.clone();
        changed.expected_type = Some(value_type.clone());
        changed
    }

    /// Makes the expression prefer a comptime category type (such as a domain, function, etc.)
    pub fn prefer_category(&self, category: IdentifierCategory) -> Self {
        let mut changed = self.clone();
        changed.preferred_category = category;
        changed
    }
}