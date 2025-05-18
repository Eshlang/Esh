use std::collections::{HashMap, HashSet, VecDeque};

use dfbin::{enums::{Instruction, Parameter, ParameterValue}, instruction, Constants::{self, Actions::{self, Plev, DP}}, DFBin};
use crate::{buffer::{self, Buffer}, codeline::{Codeline, CodelineBranch, CodelineBranchLog, CodelineBranchType}, errors::OptimizerError, optimizer_settings::OptimizerSettings};

#[derive(Clone, Debug, PartialEq)]
pub struct Optimizer {
    bin: DFBin,
    buffer: Buffer,
    pub settings: OptimizerSettings,
    pub(crate) extension_function_idents: HashMap<u32, usize>
}

impl Optimizer {
    /// Creates a new optimizer instance from a bin and settings.
    pub fn new(bin: DFBin, settings: OptimizerSettings) -> Result<Self, OptimizerError> {
        Ok(Self {
            bin: bin.clone(),
            buffer: buffer::Buffer::new(bin)?,
            settings,
            extension_function_idents: HashMap::new()
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
            self.split_lines(max)?;
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
        let max_codeblocks = max_codeblocks + 1;
        let mut new_codelines = Vec::new();
        let mut extension_add = VecDeque::new();
        let mut id_offset: u32 = (self.buffer.func_buffer.len() + self.buffer.param_buffer.len()).try_into().expect("Amount of ids should be below a u32 limit.");
        id_offset -= 1;
        let mut id: u32 = 0;
        let code_branches_len = self.buffer.code_branches.len();
        for codeline_index in 0..code_branches_len {
            // dbg!(format!("Codeline {:?}", codeline_index));
            let pre_codevar_count: u32 = (self.buffer.func_buffer.len() + self.buffer.param_buffer.len()).try_into().expect("Amount of ids should be below a u32 limit.");
            let codeline_vars = self.get_codeline_vars(codeline_index);
            let post_codevar_count: u32 = (self.buffer.func_buffer.len() + self.buffer.param_buffer.len()).try_into().expect("Amount of ids should be below a u32 limit.");
            id_offset += post_codevar_count - pre_codevar_count;
            let codeline = self.buffer.code_branches.get_mut(codeline_index).expect("Should be within list");
            for (depth, branches) in codeline.branches_by_depth.clone().into_iter().enumerate().rev() {
                for branch_ind in branches {
                    let mut test = 0;
                    'rebreak: loop {
                        let mut branch = codeline.branch_list.get(branch_ind).expect("Should contain branch.").clone();
                        if let Some((c, mut ca)) = Self::split_branch(&mut codeline.branch_list, &mut branch.body, max_codeblocks, depth, id+id_offset+1, &codeline_vars)? {
                            id += 1;
                            new_codelines.push((c, id+id_offset));
                            let mut func_name = "__e".to_string();
                            func_name.push_str(&id.to_string());
                            self.buffer.func_buffer.push_instruction(instruction!(DF, [
                                (Ident, id+id_offset), (String, func_name)
                            ]));
                            extension_add.append(&mut ca);
                            codeline.branch_list[branch_ind] = branch;
                        } else {
                            break 'rebreak;
                        }
                        test += 1;
                        if test >= 100 {
                            break;
                        }
                    }
                }
            }
            let mut test = 0;
            'rebreak: loop {
                if let Some((c, mut ca)) = Self::split_branch(&mut codeline.branch_list, &mut codeline.root_branch, max_codeblocks, 0, id+id_offset+1, &codeline_vars)? {
                    id += 1;
                    new_codelines.push((c, id+id_offset));
                    let mut func_name = "__e".to_string();
                    func_name.push_str(&id.to_string());
                    self.buffer.func_buffer.push_instruction(instruction!(DF, [
                        (Ident, id+id_offset), (String, func_name)
                    ]));
                    extension_add.append(&mut ca);
                } else {
                    break 'rebreak;
                }
                test += 1;
                if test >= 100 {
                    break;
                }
            }
        }
        for (new_codeline, new_codeline_key) in new_codelines.into_iter() {
            self.extension_function_idents.insert(new_codeline_key, self.buffer.code_branches.len());
            self.buffer.code_branches.push(new_codeline);
        }
        loop {
            let Some(extension_ident) = extension_add.pop_front() else { break; };
            let Some(extension_ident) = self.extension_function_idents.get(&extension_ident) else { continue; };
            // dbg!(extension_ident);
            self.buffer.code_branches[*extension_ident].nest_depth += 1;
            let mut hash_check = HashSet::new();
            for instruction_check_call in self.buffer.code_branches[*extension_ident].clone().to_bin().instructions() {
                if instruction_check_call.action != Actions::Call { continue; }
                let Some(call_ident) = instruction_check_call.params.get(0) else { continue; };
                let ParameterValue::Ident(call_ident) = call_ident.value else { continue; };
                if hash_check.contains(&call_ident) { continue; }
                hash_check.insert(call_ident);
                extension_add.push_back(call_ident);
            }
        }
        Ok(())
    }

    /// Gets all the line variables that are used in a branch.
    fn get_codeline_vars(&mut self, codeline_index: usize) -> Vec<(u32, u32)> {
        let mut id_offset: u32 = (self.buffer.func_buffer.len() + self.buffer.param_buffer.len()).try_into().expect("Amount of ids should be below a u32 limit.");
        id_offset -= 1;
        let mut id = 0;
        let mut ret = Vec::new();
        let codeline = self.buffer.code_branches.get(codeline_index).expect("Should be within list");
        let codeline_instructions = codeline.clone().to_bin().instructions();
        let mut potential_idents = HashSet::new();
        let _ = self.buffer.param_buffer.verify_buffer();
        for instruction in codeline_instructions {
            for param in instruction.params {
                if let ParameterValue::Ident(ident) = param.value {
                    potential_idents.insert(ident);
                }
            }
        }
        let mut new_instructions = Vec::new();
        let mut idents = Vec::new();
        for potential_ident in potential_idents {
            self.buffer.param_buffer.set_cursor_to_index(0).expect("Should be able to reset the param buffer cursor.");
            for _param_buffer_id in 0..self.buffer.param_buffer.len() {
                // Should be able to speed this up with a hashmap, cbf tho
                let param_instruction = self.buffer.param_buffer.read_instruction().expect("Should have a param instruction.");
                if !matches!(param_instruction.action, dfbin::Constants::Actions::DP::Var) { continue; }
                let Some(param) = param_instruction.params.get(0) else { continue; };
                let ParameterValue::Ident(created_param_ident) = param.value else { continue; };
                let Some(param) = param_instruction.params.get(1) else { continue; };
                let ParameterValue::String(created_var_string) = param.value.clone() else { continue; };
                if potential_ident == created_param_ident {
                    if matches!(param_instruction.match_tag(dfbin::Constants::Tags::DP::Var::Scope::Global).expect("Var instruction shouldn't have a dynamic scope tag?"), dfbin::Constants::Tags::DP::Var::Scope::Line) {
                        idents.push((potential_ident, created_var_string));
                    }
                    break;
                }
            }
        }
        for (var_ident, var_name) in idents {
            let mut param_ident = None;
            self.buffer.param_buffer.set_cursor_to_index(0).expect("Should be able to reset the param buffer cursor.");
            for _param_buffer_id in 0..self.buffer.param_buffer.len() {
                // Should be able to speed this up with a hashmap, cbf tho
                let param_instruction = self.buffer.param_buffer.read_instruction().expect("Should have a param instruction.");
                if !matches!(param_instruction.action, dfbin::Constants::Actions::DP::Param) { continue; }
                let Some(param) = param_instruction.params.get(0) else { continue; };
                let ParameterValue::Ident(created_param_ident) = param.value else { continue; };
                let Some(param) = param_instruction.params.get(1) else { continue; };
                let ParameterValue::String(created_var_string) = param.value.clone() else { continue; };
                if created_var_string != var_name { continue; }
                if matches!(param_instruction.match_tag(dfbin::Constants::Tags::DP::Param::Type::Any).expect("Param instruction shouldn't have a dynamic scope tag?"), dfbin::Constants::Tags::DP::Param::Type::Var) {
                    param_ident = Some(created_param_ident);
                }
            }
            let param_ident = param_ident.unwrap_or_else(|| ({ 
                id += 1; 
                new_instructions.push(instruction!(DP::Param, [
                    (Ident, id+id_offset), (String, var_name)
                ], { Type: Var }));
                id
            }) + id_offset);
            ret.push((var_ident, param_ident))
        }
        self.buffer.param_buffer.append_instructions(new_instructions);

        ret
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
    fn split_branch(branches: &mut Vec<CodelineBranch>, branch: &mut Vec<CodelineBranchLog>, true_max_codeblocks: usize, depth: usize, id: u32, codeline_vars: &Vec<(u32, u32)>) -> Result<Option<(Codeline, VecDeque<u32>)>, OptimizerError> {
        // dbg!(id, depth);
        let mut sum = 0;
        let mut new_branch_accumulate = Vec::new();
        let mut old_branch_accumulate = Vec::new();
        let padding = 2; // One for call function, one for extra function
        let max_codeblocks = true_max_codeblocks - padding; // Padding 
        for (log_ind, log) in branch.iter().enumerate().rev() {
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
                    let mut add = log_branch.instructions(&branches).len() + 2;
                    let add_amt = add;
                    if log_branch.root.action == Constants::Actions::Else {
                        if let Some(CodelineBranchLog::Branch(log_if_branch_ind)) = branch.get(log_ind-1) {
                            let log_if_branch = &branches[*log_if_branch_ind];
                            add += log_if_branch.instructions(&branches).len() + 1;
                        }
                    }
                    if sum < max_codeblocks {
                        if sum + add >= max_codeblocks { // Transition Period
                            // dbg!("transition period on branch!!", sum+add, sum);
                            old_branch_accumulate.push(log.clone());
                        } else {
                            new_branch_accumulate.push(log.clone());
                        }
                    } else { // Sum > max_codeblocks
                        old_branch_accumulate.push(log.clone());
                    }
                    sum += add_amt;
                    // branch.push(CodelineBranchLog::Codeblocks(vec![instruction!(Enac::Tame, [(Int, log_branch_ind), (Int, log_branch.instructions(&branches).len())])]));
                },
            }
        };
        if depth > 0 && sum < max_codeblocks && sum >= max_codeblocks - padding - (depth * 2) - 2 && sum > 1 {
            old_branch_accumulate.clear();
            new_branch_accumulate = branch.clone();
            new_branch_accumulate.reverse();
            sum = max_codeblocks;
        }
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
            let mut call_instruction = instruction!(Call, [(Ident, id)]);
            let mut func_instruction = instruction!(Func, [(Ident, id)]);
            for codeline_var in codeline_vars {
                call_instruction.params.push(Parameter::from_ident(codeline_var.0)); // Add the variable
                func_instruction.params.push(Parameter::from_ident(codeline_var.1)); // Add the param
            }

            branch.push(CodelineBranchLog::Codeblocks(vec![call_instruction]));
            let mut codeline = Codeline::from_bin(DFBin::from_instructions(vec![func_instruction]))?;
            codeline.branch_list = branches.clone();
            codeline.root_branch = new_branch_accumulate;
            codeline.nest_depth += 1;
            let mut extension_funcs = VecDeque::new();
            for instruction_check_call in codeline.clone().to_bin().instructions() {
                if instruction_check_call.action != Actions::Call { continue; }
                let Some(call_ident) = instruction_check_call.params.get(0) else { continue; };
                let ParameterValue::Ident(call_ident) = call_ident.value else { continue; };
                if extension_funcs.contains(&call_ident) { continue; }
                extension_funcs.push_back(call_ident);
            }
            return Ok(Some((codeline, extension_funcs)))
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
        let name = "first";
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
            max_codeblocks_per_line: Some(9),
        }).expect("Optimizer should create.");  
        
        optimizer.optimize().expect("Optimizer should optimize.");

        let optimized_bin = optimizer.flush();

        let mut original_decompiler = decompiler::Decompiler::new(optimized_bin.clone()).expect("Decompiler should create optimized");
        original_decompiler.set_capitalization(decompiler::decompiler::DecompilerCapitalization::lowercase);
        let decompiled = original_decompiler.decompile().expect("Decompiler should decompile optimized");
        
        fs::write(format!("{}{}_optimized.dfa", path, name), decompiled).expect("Decompiled optimized DFA should write.");
    
        
    }
}