use dfbin::{enums::Instruction, instruction, Constants::Actions::{self, Plev}, DFBin};
use crate::{buffer::{self, Buffer}, codeline::{Codeline, CodelineBranch, CodelineBranchLog, CodelineBranchType}, errors::OptimizerError, optimizer_settings::OptimizerSettings};

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
        let mut new_codelines = Vec::new();
        let mut id_offset: u32 = (self.buffer.func_buffer.len() + self.buffer.param_buffer.len()).try_into().expect("Amount of ids should be below a u32 limit.");
        id_offset -= 1;
        let mut id: u32 = id_offset;
        for codeline in self.buffer.code_branches.iter_mut() {
            if let Some(c) = Self::split_branch(&mut codeline.branch_list, &mut codeline.root_branch, max_codeblocks, 0, &mut id)? {
                new_codelines.push(c);
                let mut func_name = "__e".to_string();
                func_name.push_str(&(id - id_offset).to_string());
                self.buffer.func_buffer.push_instruction(instruction!(DF, [
                    (Ident, id), (String, func_name)
                ]));
            }
            for (depth, branches) in codeline.branches_by_depth.clone().into_iter().enumerate() {
                for branch_ind in branches {
                    let mut branch = codeline.branch_list.get(branch_ind).expect("Should contain branch.").clone();
                    if let Some(c) = Self::split_branch(&mut codeline.branch_list, &mut branch.body, max_codeblocks, depth, &mut id)? {
                        new_codelines.push(c);
                        let mut func_name = "__e".to_string();
                        func_name.push_str(&(id - id_offset).to_string());
                        self.buffer.func_buffer.push_instruction(instruction!(DF, [
                            (Ident, id), (String, func_name)
                        ]));
                    }
                    codeline.branch_list[branch_ind] = branch;
                }
            }
        }
        self.buffer.code_branches.append(&mut new_codelines);
        Ok(())
    }

    /// This is used by ``.split_branch()`` to count how many instructions there are in a branch, so that it will know if it needs to compact.
    // fn count_branch(branches: &mut Vec<CodelineBranch>, branch: &Vec<CodelineBranchLog>) -> usize {
    //     let mut sum = 0;
    //     for log in branch.clone() {
    //         match &log {
    //             CodelineBranchLog::Codeblocks(instructions) => {
    //                 // branch.push(CodelineBranchLog::Codeblocks(vec![instruction!(Enac::FoxSleeping, [(Int, instructions.len())])]));
    //                 let add = instructions.len();
    //                 if sum + add < max_codeblocks {
    //                     branch_accumulate.push(log.clone());
    //                 }
    //                 sum += add;
    //             },
    //             CodelineBranchLog::Branch(log_branch_ind) => {
    //                 let log_branch = &branches[*log_branch_ind];
    //                 let add = log_branch.instructions(&branches).len() + 2;
    //                 if sum + add < max_codeblocks {
    //                     branch_accumulate.push(log);
    //                 }
    //                 sum += add;
    //                 // dbg!(log_branch);
    //                 // branch.push(CodelineBranchLog::Codeblocks(vec![instruction!(Enac::Tame, [(Int, log_branch_ind), (Int, log_branch.instructions(&branches).len())])]));
    //             },
    //         }
    //         if sum >= max_codeblocks {
    //             break;
    //         }
    //     };
    //     if sum >= max_codeblocks {
            
    //     }
    // }

    /// This is used by ``.split_lines()`` - compacts a *branch* down to below the max size.
    fn split_branch(branches: &mut Vec<CodelineBranch>, branch: &mut Vec<CodelineBranchLog>, true_max_codeblocks: usize, depth: usize, id: &mut u32) -> Result<Option<Codeline>, OptimizerError> {
        let mut sum = 0;
        let mut new_branch_accumulate = Vec::new();
        let mut old_branch_accumulate = Vec::new();
        let padding = 2; // One for call function, one for extra function
        let max_codeblocks = true_max_codeblocks - padding; // Padding 
        for log in branch.iter().rev() {
            match &log {
                CodelineBranchLog::Codeblocks(instructions) => {
                    // branch.push(CodelineBranchLog::Codeblocks(vec![instruction!(Enac::FoxSleeping, [(Int, instructions.len())])]));
                    let add = instructions.len();
                    if sum < max_codeblocks {
                        if sum + add >= max_codeblocks { // Transition Period
                            let mut old_instructions = instructions.clone();
                            let new_instructions = old_instructions.split_off(old_instructions.len() - (max_codeblocks - sum));
                            old_branch_accumulate.push(CodelineBranchLog::Codeblocks(old_instructions));
                            new_branch_accumulate.push(CodelineBranchLog::Codeblocks(new_instructions));
                        } else {
                            new_branch_accumulate.push(log.clone());
                        }
                    } else {
                        old_branch_accumulate.push(log.clone());
                    }

                    sum += add;
                },
                CodelineBranchLog::Branch(log_branch_ind) => {
                    let log_branch = &branches[*log_branch_ind];
                    let add = log_branch.instructions(&branches).len() + 2;
                    if sum < max_codeblocks {
                        if sum + add >= max_codeblocks { // Transition Period
                            old_branch_accumulate.push(log.clone());
                        } else {
                            new_branch_accumulate.push(log.clone());
                        }
                    } else { // Sum > max_codeblocks
                        old_branch_accumulate.push(log.clone());
                    }
                    sum += add;
                    // dbg!(log_branch);
                    // branch.push(CodelineBranchLog::Codeblocks(vec![instruction!(Enac::Tame, [(Int, log_branch_ind), (Int, log_branch.instructions(&branches).len())])]));
                },
            }
        };
        if sum >= max_codeblocks {
            // Awful code ahead:
            old_branch_accumulate.reverse();
            new_branch_accumulate.reverse();
            branch.clear();
            for b in old_branch_accumulate {
                branch.push(b)
            }
            if new_branch_accumulate.len() == 1 {
                if let CodelineBranchLog::Codeblocks(g) = &new_branch_accumulate[0] {
                    if g.len() == 0 {
                        return Ok(None);
                    }
                }
            }
            *id += 1;
            branch.push(CodelineBranchLog::Codeblocks(vec![instruction!(Call, [(Ident, *id)])]));
            let mut codeline = Codeline::from_bin(DFBin::from_instructions(vec![instruction!(Func, [(Ident, *id)])]))?;
            codeline.branch_list = branches.clone();
            codeline.root_branch = new_branch_accumulate;
            return Ok(Some(codeline))
        }
        Ok(None)
        // branch.push(CodelineBranchLog::Codeblocks(vec![instruction!(Enac::FoxSleeping, [(Int, sum)])]));
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
        let name = "test";
        // let path = r"C:\Users\koren\OneDrive\Documents\Github\Esh\optimizer\examples\";
        let path = r"K:\Programming\GitHub\Esh\optimizer\examples\";

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