use dfbin::DFBin;
use crate::{buffer::{self, Buffer}, codeline::CodelineBranchLog, errors::OptimizerError, optimizer_settings::OptimizerSettings};

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
        Ok(())
    }

    /// Locates returns in functions and truncates everything after them.
    pub fn remove_end_returns(&mut self) -> Result<(), OptimizerError> {
        
        Ok(())
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
            remove_end_returns: true
        }).expect("Optimizer should create.");
        
        optimizer.optimize().expect("Optimizer should optimize.");

        let optimized_bin = optimizer.flush();

        let mut original_decompiler = decompiler::Decompiler::new(optimized_bin.clone()).expect("Decompiler should create optimized");
        original_decompiler.set_capitalization(decompiler::decompiler::DecompilerCapitalization::lowercase);
        let decompiled = original_decompiler.decompile().expect("Decompiler should decompile optimized");
        
        fs::write(format!("{}{}_optimized.dfa", path, name), decompiled).expect("Decompiled optimized DFA should write.");
    
        
    }
}