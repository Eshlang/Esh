#[derive(Clone, Debug, PartialEq)]
pub struct OptimizerSettings {
    /// Removes end returns.
    pub remove_end_returns: bool,
    /// Splits the lines of functions & branches to accommodate a specific plot codeblock size limitation.
    /// 25 would be the limit on a basic plot, 50 on a large, and 150 on a massive.
    pub max_codeblocks_per_line: Option<usize>,
    /// Function inliner
    pub inline_functions: InlinerOptimizerSetting,
    /// Elimination of dead code (functions, variables, etc)
    pub deadcode_elimination: bool
}


#[derive(Clone, Debug, PartialEq)]
pub enum InlinerOptimizerSetting {
    /// Doesn't inline at all.
    None,
    /// Only inlines where it's 100% harmless (the function being inlined is very little codeblocks and not used in succession often).
    Conservative,
    /// Inlines reasonably, more than the ``Conservative`` option but avoids huge inlines of large functions.
    Balanced,
    /// Always inlines, in hopes of prioritizing performance.
    Performance
}