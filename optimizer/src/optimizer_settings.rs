#[derive(Clone, Debug, PartialEq)]
pub struct OptimizerSettings {
    pub remove_end_returns: bool,
    pub max_codeblocks_per_line: Option<usize>,
}