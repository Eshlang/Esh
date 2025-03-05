use dfbin::{instruction, Constants::Actions, DFBin};
use crate::{buffer::{self, Buffer}, codeline::{CodelineBranch, CodelineBranchLog}, errors::OptimizerError, optimizer_settings::OptimizerSettings};

#[derive(Clone, Debug, PartialEq)]
pub struct Optimizer {
    bin: DFBin,
    buffer: Buffer,
    pub settings: OptimizerSettings
}

impl Optimizer {
    /// Creates a new optimizer instance from a bin and settings.
    pub fn new(bin: DFBin, settings: OptimizerSettings) -> Result<Self, OptimizerError> {
        Ok(Self {
            bin: bin.clone(),
            buffer: buffer::Buffer::new(bin)?,
            settings,
        })
    }

    /// Flushes the optimizer, returning the finished bin.
    pub fn flush(&mut self) -> DFBin {
        self.buffer.flush()
    }

    /// Runs the optimizer with its given settings and bin.
    pub fn optimize(&mut self) -> Result<(), OptimizerError> {
        if self.settings.remove_end_returns {
            self.remove_end_returns()?;
        }
        if let Some(max) = self.settings.max_codeblocks_per_line {
            self.split_lines(max);
        }
        Ok(())
    }

    /// Locates returns in functions and truncates everything after them.
    pub fn remove_end_returns(&mut self) -> Result<(), OptimizerError> {
        for codeline in self.buffer.code_branches.iter_mut() {
            Self::remove_end_returns_from_branch(&mut codeline.root_branch, true);
            for branch in codeline.branch_list.iter_mut() {
                Self::remove_end_returns_from_branch(&mut branch.body, false);
            }
        }
        Ok(())
    }

    fn remove_end_returns_from_branch(branch: &mut Vec<CodelineBranchLog>, delete_return: bool) {
        let mut truncate_ind = branch.len();
        'branch_loop: for (branch_ind, branch_log) in branch.iter_mut().enumerate() {
            let CodelineBranchLog::Codeblocks(codeblocks) = branch_log else {
                continue 'branch_loop;
            };
            'codeblock_loop: for (codeblock_ind, codeblock) in codeblocks.iter().enumerate() {
                if codeblock.action != Actions::Ctrl::Return {
                    continue 'codeblock_loop;
                }
                codeblocks.truncate(codeblock_ind + if delete_return {0} else {1});
                truncate_ind = branch_ind + 1;
                break 'branch_loop;
            }
        }
        branch.truncate(truncate_ind);
    }

    /// Splits codelines to a maximum codeblock length
    /// Full Algorithm:
    /// ```txt
    /// CB(120), Branch#2(15), CB(30)
    ///     
    /// 
    /// CB(30) > 50? X
    /// CB(30) + Branch#2(15) > 50? X
    /// CB(30) + Branch#2(15) + CB(120) > 50? Y
    /// if reached all elements sum and still X, mark the branch compacted and continue on to the next 
    /// so
    /// take the last 50 codeblocks from:
    /// [CB(120), Branch#2(15), CB(30)]
    /// 
    /// [CB(50-15-30-2 = 3), Branch#2(15), CB(30)]
    /// 
    /// and put that on a new codeline, as an extender function
    /// 
    /// and reset the branch to what's left:
    /// [CB(120-3 = 117), CB(1)] <- CB(1) is call function overhead
    /// 
    /// 
    /// now retry:
    /// CB(1) > 50? X
    /// CB(1) + CB(117) > 50? Y
    /// 
    /// last 50 codeblocks from:
    /// [CB(117), CB(1)]
    /// are
    /// [CB(49), CB(1)]
    /// 
    /// putting that on a new codeline, remains:
    /// [CB(117-49 = 68), CB(1)]
    /// 
    /// then reset, then one last time it'd turn into
    /// [CB(19), CB(1)]
    /// where finally CB(19) + CB(1) < 50, so it marks the branch fully compacted.
    /// 
    /// ```
    pub fn split_lines(&mut self, max_codeblocks: usize) -> Result<(), OptimizerError> {
        for codeline in self.buffer.code_branches.iter_mut() {
            Self::split_branch(&mut codeline.branch_list, &mut codeline.root_branch, max_codeblocks, 0);
            for (depth, branches) in codeline.branches_by_depth.clone().into_iter().enumerate() {
                for branch_ind in branches {
                    let mut branch = codeline.branch_list.get(branch_ind).expect("Should contain branch.").clone();
                    Self::split_branch(&mut codeline.branch_list, &mut branch.body, max_codeblocks, depth);
                    codeline.branch_list[branch_ind] = branch;
                }
            }
        }
        Ok(())
    }

    /// This is used by ``.split_lines()`` - compacts a *branch* down to below the max size.
    fn split_branch(branches: &mut Vec<CodelineBranch>, branch: &mut Vec<CodelineBranchLog>, max_codeblocks: usize, depth: usize) {
        for log in branch.clone() {
            match log {
                CodelineBranchLog::Codeblocks(instructions) => {
                    branch.push(CodelineBranchLog::Codeblocks(vec![instruction!(Enac::FoxSleeping, [(Int, instructions.len())])]));
                },
                CodelineBranchLog::Branch(log_branch_ind) => {
                    let log_branch = &branches[log_branch_ind];
                    dbg!(log_branch);
                    branch.push(CodelineBranchLog::Codeblocks(vec![instruction!(Enac::Tame, [(Int, log_branch_ind), (Int, log_branch.instructions(&branches).len())])]));
                },
            }
        }
        // for 
    }
}



#[cfg(test)]
mod tests {
    use std::{fs, str::from_utf8};

    use compiler::Compiler;

    use super::*;


    #[test]
    pub fn optimize_from_file_test() {
        let name = "first";
        let path = r"C:\Users\koren\OneDrive\Documents\Github\Esh\optimizer\examples\";
        // let path = r"K:\Programming\GitHub\Esh\optimizer\examples\";

        let file_bytes = fs::read(format!("{}{}.dfa", path, name)).expect("File should read");
        let mut compiler = Compiler::new(from_utf8(&file_bytes).expect("File should be valid utf-8"));
        let bin = compiler.compile_string().expect("Compiler should compile.");

        let mut original_decompiler = decompiler::Decompiler::new(bin.clone()).expect("Decompiler should create original");
        original_decompiler.set_capitalization(decompiler::decompiler::DecompilerCapitalization::lowercase);
        let decompiled = original_decompiler.decompile().expect("Decompiler should decompile original");
        
        fs::write(format!("{}{}_before.dfa", path, name), decompiled).expect("Decompiled original DFA should write.");
        
        let mut optimizer = Optimizer::new(bin.clone(), OptimizerSettings {
            remove_end_returns: true,
            max_codeblocks_per_line: Some(10),
        }).expect("Optimizer should create.");  
        
        optimizer.optimize().expect("Optimizer should optimize.");

        let optimized_bin = optimizer.flush();

        let mut original_decompiler = decompiler::Decompiler::new(optimized_bin.clone()).expect("Decompiler should create optimized");
        original_decompiler.set_capitalization(decompiler::decompiler::DecompilerCapitalization::lowercase);
        let decompiled = original_decompiler.decompile().expect("Decompiler should decompile optimized");
        
        fs::write(format!("{}{}_optimized.dfa", path, name), decompiled).expect("Decompiled optimized DFA should write.");
    
        
    }
}