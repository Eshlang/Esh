use std::{fmt, rc::Rc};

use parser::parser::Node;

#[derive(thiserror::Error, Debug, PartialEq)]
#[error("Compiler error.")]
pub struct CodegenError {
    token: Option<ErrorToken>,
    pub source: ErrorRepr,
}

#[derive(Debug, PartialEq)]
pub struct ErrorToken {
    pub token: Rc<Node>,
    pub position: usize
}
impl fmt::Display for ErrorToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}; {:?})", self.position, self.token)
    }
}

impl CodegenError {
    pub fn new(node: Rc<Node>, source: ErrorRepr) -> CodegenError {
        Self {
            token: Some(ErrorToken {
                token: node,
                position: 0
            }),
            source
        }
    }
    pub fn err<T>(node: Rc<Node>, source: ErrorRepr) -> Result<T, CodegenError> {
        Err(Self::new(node, source))
    }
    pub fn err_headless<T>(source: ErrorRepr) -> Result<T, CodegenError> {
        Err(Self::new_headless(source))
    }
    pub fn map<T, U>(err: Result<T, U>, node: Rc<Node>, source: ErrorRepr) -> Result<T, CodegenError> {
        err.map_err(|_| Self::new(node, source))
    }
    pub fn map_headless<T, U>(err: Result<T, U>, source: ErrorRepr) -> Result<T, CodegenError> {
        err.map_err(|_| Self::new_headless(source))
    }
    pub fn new_position(node: Rc<Node>, position: usize, source: ErrorRepr) -> CodegenError {
        Self {
            token: Some(ErrorToken {
                token: node,
                position
            }),
            source
        }
    }
    pub fn new_headless(source: ErrorRepr) -> CodegenError {
        Self {
            token: None,
            source
        }
    }
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum ErrorRepr {
    #[error("Generic Error")]
    Generic,
    #[error("Expected a code block.")]
    ExpectedBlock,
    #[error("Expected a scannable code block.")]
    ExpectedScannableBlock,
    #[error("Expected a function identifier string.")]
    ExpectedFunctionIdentifier,
    #[error("Expected a struct identifier string.")]
    ExpectedStructIdentifier,
    #[error("Expected an accessable identifier string.")]
    ExpectedAccessableIdentifier,
    #[error("Expected a struct field identifier string.")]
    ExpectedStructFieldIdentifier,
    #[error("Borrowing error. (Yeah you know the system in rust thats supposed to fix errors? it errored)")]
    BadBorrow,
    #[error("Mutable borrowing error. (Yeah you know the system in rust thats supposed to fix errors? it errored)")]
    BadMutBorrow,
    #[error("Unexpected unstructured code in struct.")]
    UnstructuredStructCode,
    #[error("Expected proper function parameter declarations.")]
    ExpectedFunctionParamDeclaration,
    #[error("Expected proper function call parameters.")]
    ExpectedFunctionCallParameters,
    #[error("Expected function parameter identifier.")]
    ExpectedFunctionParamIdent,
    #[error("Expected type identifier.")]
    ExpectedTypeIdent,
    #[error("Expected definition identifier.")]
    ExpectedDefinitionIdent,
    #[error("Type identifier not recognized.")]
    TypeIdentNotRecognized,
    #[error("Definition identifier not recognized.")]
    DefinitionIdentNotRecognized,
    #[error("Domain identifier not recognized.")]
    DomainIdentNotRecognized,
    #[error("Expected a context.")]
    ExpectedContext,
    #[error("Expected a field.")]
    ExpectedField,
    #[error("Expected a struct.")]
    ExpectedStruct,
    #[error("Expected a function.")]
    ExpectedFunction,
    #[error("Expected a variable identifier.")]
    ExpectedVariableIdentifier,
    #[error("Expected a variable.")]
    ExpectedVariable,
    #[error("Invalid variable name.")]
    InvalidVariableName,
    #[error("Unexpected expression token.")]
    UnexpectedExpressionToken,
    #[error("Invalid expression type conversion.")]
    InvalidExpressionTypeConversion,
    #[error("Invalid variable type.")]
    InvalidVariableType,
    #[error("Functions cannot nest inside functions.")]
    FunctionNestedInFunction,
    #[error("Structs cannot nest inside functions.")]
    StructNestedInFunction,
    #[error("Domains cannot nest inside functions.")]
    DomainNestedInFunction,
    #[error("Expected a domain identifier string.")]
    ExpectedDomainIdentifier,
    #[error("Domains cannot nest inside structs.")]
    DomainNestedInStruct,
    #[error("A register deallocation error has occured. This is most likely the result of a mutated ownership variable.")]
    RegisterDeallocationError,
    #[error("Invalid assignment token.")]
    InvalidAssignmentToken,
    #[error("Unexpectedly declaring an existing variable.")]
    DeclaringExistingVariable,
    #[error("Unexpected return value; the function does not return a value.")]
    UnexpectedReturnValue,
    #[error("Invalid return value type.")]
    InvalidReturnValueType,
    #[error("Expected the function to return a value.")]
    ExpectedFunctionReturnValue,
    #[error("Unexpected function parameter.")]
    UnexpectedFunctionParameter,
    #[error("Expected a function parameter.")]
    ExpectedFunctionParameter,
    #[error("Unexpected function parameter type.")]
    UnexpectedFunctionParameterType,
    #[error("Value type is unexpectedly an unparsed identifier.")]
    UnexpectedValueTypeIdent,
    #[error("Unexpected struct access identifier.")]
    UnexpectedStructAccessIdent,
    #[error("Incomplete struct construct, field(s) are missing.")]
    ConstructFieldsMissing,
    #[error("Expected a field assignment.")]
    ExpectedFieldAssignment,
    #[error("Invalid struct field.")]
    InvalidStructField,
    #[error("Unexpected struct field type.")]
    UnexpectedStructFieldType,
    #[error("Expected an accessable type (such as a struct).")]
    ExpectedAccessableType,
    #[error("Expected an accessable node (such as a domain).")]
    ExpectedAccessableNode,
}
