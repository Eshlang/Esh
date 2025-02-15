#[derive(thiserror::Error, Debug, PartialEq)]
#[error("Optimizer error at instruction #{}", instruction)]
pub struct OptimizerError {
    pub instruction: usize,
    pub source: OptimizerErrorRepr,
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum OptimizerErrorRepr {
}
