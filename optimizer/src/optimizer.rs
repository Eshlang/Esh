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
        let code_branches_len = self.buffer.code_branches.len();
        let mut count_extensions = 0;
        let free_ids = (self.buffer.next_ident(), self.buffer.next_ident()+1, self.buffer.next_ident()+2);
        self.buffer.ident_count += 3;
        for codeline_index in 0..code_branches_len {
            // dbg!(format!("Codeline {:?}", codeline_index));
            let codeline_vars = self.get_codeline_vars(codeline_index);
            let mut buffer_change = self.buffer.clone();
            let codeline = self.buffer.code_branches.get_mut(codeline_index).expect("Should be within list");
            for (depth, branches) in codeline.branches_by_depth.clone().into_iter().enumerate().rev() {
                for branch_ind in branches {
                    let mut test = 0;
                    'rebreak: loop {
                        let mut branch = codeline.branch_list.get(branch_ind).expect("Should contain branch.").clone();
                        if let Some((c, mut ca)) = Self::split_branch(&mut codeline.branch_list, &mut branch.body, max_codeblocks, depth, buffer_change.next_ident(), free_ids, &codeline_vars)? {
                            let mut func_name = "__e".to_string();
                            count_extensions += 1;
                            func_name.push_str(&count_extensions.to_string());
                            let ident = buffer_change.add_definition(instruction!(DF, [
                                (String, func_name)
                            ]));
                            new_codelines.push((c, ident));
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
                if let Some((c, mut ca)) = Self::split_branch(&mut codeline.branch_list, &mut codeline.root_branch, max_codeblocks, 0, buffer_change.next_ident(), free_ids, &codeline_vars)? {
                    let mut func_name = "__e".to_string();
                    count_extensions += 1;
                    func_name.push_str(&count_extensions.to_string());
                    let ident = buffer_change.add_definition(instruction!(DF, [
                        (String, func_name)
                    ]));
                    new_codelines.push((c, ident));
                    extension_add.append(&mut ca);
                } else {
                    break 'rebreak;
                }
                test += 1;
                if test >= 100 {
                    break;
                }
            }
            self.buffer.load_state(buffer_change);
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
    fn get_codeline_vars(&mut self, codeline_index: usize) -> Vec<(u32, u32, String)> {
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
        let mut idents = Vec::new();
        for potential_ident in potential_idents {
            let Some((defining_instruction, defining_params)) = self.buffer.ident_definitions.get(&potential_ident) else { continue; };
            if !matches!(defining_instruction.action, dfbin::Constants::Actions::DP::Var) { continue; }
            //##dbg!(defining_instruction);
            //##dbg!("test1");
            
            let Some(ParameterValue::String(created_var_string)) = defining_params.get(0) else { continue; };
            //##dbg!("test2");
            if !matches!(defining_instruction.match_tag(dfbin::Constants::Tags::DP::Var::Scope::Global).expect("Var instruction shouldn't have a dynamic scope tag?"), dfbin::Constants::Tags::DP::Var::Scope::Line) {
                continue;
            }
            //##dbg!("test3");

            idents.push((potential_ident, created_var_string.to_owned()));
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
            let param_ident = param_ident.unwrap_or_else(|| self.buffer.add_definition(instruction!(DP::Param, [
                (String, var_name.clone())
                ], { Type: Var })));
            ret.push((var_ident, param_ident, var_name));
            //##dbg!(param_ident);
        }
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
    fn split_branch(branches: &mut Vec<CodelineBranch>, branch: &mut Vec<CodelineBranchLog>, true_max_codeblocks: usize, depth: usize, func_id: u32, free_ids: (u32, u32, u32), codeline_vars: &Vec<(u32, u32, String)>) -> Result<Option<(Codeline, VecDeque<u32>)>, OptimizerError> {
        // dbg!(id, depth);
        let mut sum = 0;
        let mut new_branch_accumulate = Vec::new();
        let mut old_branch_accumulate = Vec::new();
        let padding = 2 + (if codeline_vars.len() > 26 { 7 } else { 0 }); // One for call function, one for extra function
        let max_codeblocks = true_max_codeblocks - padding; // Padding 
        for (log_ind, log) in branch.iter().enumerate().rev() {
            match &log {
                CodelineBranchLog::Codeblocks(instructions) => {
                    // branch.push(CodelineBranchLog::Codeblocks(vec![instruction!(Enac::FoxSleeping, [(Int, instructions.len())])]));
                    let add = DFBin::count_codeblocks(instructions);
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
                    let mut add = log_branch.instructions(&branches).codeblocks().expect("Expecting codeblock count to work.") + 2;
                    let add_amt = add;
                    if log_branch.root.action == Constants::Actions::Else {
                        if let Some(CodelineBranchLog::Branch(log_if_branch_ind)) = branch.get(log_ind-1) {
                            let log_if_branch = &branches[*log_if_branch_ind];
                            add += log_if_branch.instructions(&branches).codeblocks().expect("Expecting codeblock count to work.") + 1;
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
        let f = (max_codeblocks as isize) - (padding as isize) - 4;
        if depth > 0 && sum < max_codeblocks && sum >= (f.max(0) as usize) && sum > 1 {
            // dbg!(sum, f, max_codeblocks, depth);
            old_branch_accumulate.clear();
            new_branch_accumulate = branch.clone();
            new_branch_accumulate.reverse();
            sum = max_codeblocks;
        }
        // dbg!("HEY", sum, max_codeblocks);
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
            let mut call_instruction = instruction!(Call, [(Ident, func_id)]);
            let mut func_instruction = instruction!(Func, [(Ident, func_id)]);
            let mut call_instructions = Vec::new();
            let mut func_instructions = Vec::new();
            let mut func_instructions_end = Vec::new();
            let mut func_instructions_pre = Vec::new();
            if codeline_vars.len() > 26 {
                let combined_names: Vec<String> = codeline_vars.iter().map(|x| x.2.clone()).collect();
                let combined_names = combined_names.join("#");
                let codeline_vars_len = codeline_vars.len();
                let mut split_value_names = vec![
                    instruction!(DP::Var, [ (Ident, free_ids.0), (String, "_optimizercopy") ], { Scope: Line }),
                    instruction!(Var::SplitString, [ (Ident, free_ids.0), (String, combined_names.clone()), (String, "#") ])
                ];
                let mut pack_values = vec![
                    instruction!(DP::Var, [ (Ident, free_ids.2), (String, "_optimizercopy_values") ], { Scope: Line }),
                    instruction!(Var::CreateList, [ (Ident, free_ids.2) ]),
                    instruction!(DP::Var, [ (Ident, free_ids.1), (String, "_optimizercopy_e") ], { Scope: Line }),
                    instruction!(Rep::ForEach, [ (Ident, free_ids.1), (Ident, free_ids.0) ]),
                    instruction!(DP::Var, [ (Ident, free_ids.1), (String, "%var(_optimizercopy_e)") ], { Scope: Line }),
                    instruction!(Var::AppendValue, [ (Ident, free_ids.2), (Ident, free_ids.1) ]),
                    instruction!(EndRep)
                ];
                let mut unpack_values = vec![
                    instruction!(DP::Var, [ (Ident, free_ids.0), (String, "_optimizercopy_i") ], { Scope: Line }),
                    instruction!(Rep::Multiple, [ (Ident, free_ids.0), (Int, codeline_vars_len) ]),
                    instruction!(DP::Var, [ (Ident, free_ids.1), (String, "%index(_optimizercopy,%var(_optimizercopy_i))") ], { Scope: Line }),
                    instruction!(DP::Var, [ (Ident, free_ids.2), (String, "_optimizercopy_values") ], { Scope: Line }),
                    instruction!(Var::GetListValue, [ (Ident, free_ids.1), (Ident, free_ids.2), (Ident, free_ids.0) ]),
                    instruction!(EndRep)
                ];
                // Call: Code, Split, Pack, Call, Unpack
                call_instructions.append(&mut split_value_names.clone());
                call_instructions.append(&mut pack_values.clone());

                call_instructions.push(instruction!(DP::Var, [ (Ident, free_ids.0), (String, "_optimizercopy_values") ], { Scope: Line }));
                call_instruction.params.push(Parameter::from_ident(free_ids.0)); // Add the variable
                call_instructions.push(call_instruction.clone());
                
                call_instructions.append(&mut unpack_values.clone());
                
                // Func: Func, Split, Unpack, Code, Pack
                func_instructions_pre.push(instruction!(DP::Param, [ (Ident, free_ids.0), (String, "_optimizercopy_values") ], { Type: Var }));


                func_instructions.append(&mut split_value_names);
                func_instructions.append(&mut unpack_values);

                
                func_instructions_end.push(instruction!(DP::Var, [ (Ident, free_ids.0), (String, "_optimizercopy") ], { Scope: Line }));
                func_instructions_end.append(&mut pack_values);

                
                func_instruction.params.push(Parameter::from_ident(free_ids.0)); // Add the param

            } else {
                for codeline_var in codeline_vars {
                    call_instruction.params.push(Parameter::from_ident(codeline_var.0)); // Add the variable
                    func_instruction.params.push(Parameter::from_ident(codeline_var.1)); // Add the param
                }
                call_instructions.push(call_instruction.clone());
            }

            branch.push(CodelineBranchLog::Codeblocks(call_instructions));
            let mut codeline = Codeline::from_bin(DFBin::from_instructions(vec![func_instruction]))?;
            codeline.branch_list = branches.clone();
            codeline.root_branch = vec![CodelineBranchLog::Codeblocks(func_instructions)];
            codeline.root_branch.append(&mut new_branch_accumulate);
            codeline.pre_instructions = func_instructions_pre;
            codeline.post_instructions = func_instructions_end;
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
            remove_end_returns: false,
            max_codeblocks_per_line: Some(45),
        }).expect("Optimizer should create.");  
        
        optimizer.optimize().expect("Optimizer should optimize.");

        let optimized_bin = optimizer.flush();

        let mut original_decompiler = decompiler::Decompiler::new(optimized_bin.clone()).expect("Decompiler should create optimized");
        original_decompiler.set_capitalization(decompiler::decompiler::DecompilerCapitalization::lowercase);
        let decompiled = original_decompiler.decompile().expect("Decompiler should decompile optimized");
        
        fs::write(format!("{}{}_optimized.dfa", path, name), decompiled).expect("Decompiled optimized DFA should write.");
    
        
    }
}