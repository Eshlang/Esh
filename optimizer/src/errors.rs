use crate::optimizer::Optimizer;

#[derive(thiserror::Error, Debug, PartialEq)]
#[error("Optimizer error at instruction #{}", instruction)]
pub struct OptimizerError {
    pub instruction: usize,
    pub source: ErrorRepr,
}

impl OptimizerError {
    pub fn new(optimizer: &Optimizer, source: ErrorRepr) -> OptimizerError {
        let _ = optimizer;
        Self {
            instruction: 0,
            source: source
        }
    }
    pub fn new_headless(source: ErrorRepr) -> OptimizerError {
        Self {
            instruction: 0,
            source: source
        }
    }
    pub fn err<T>(optimizer: &Optimizer, source: ErrorRepr) -> Result<T, OptimizerError> {
        Err(Self::new(optimizer, source))
    }
    pub fn err_headless<T>(source: ErrorRepr) -> Result<T, OptimizerError> {
        Err(Self::new_headless(source))
    }
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum ErrorRepr {
    #[error("Generic Error")]
    Generic,
    #[error("Expected a code block.")]
    ExpectedBlock,
    #[error("Expected an identifier.")]
    ExpectedIdentifier,
}