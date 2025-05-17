pub struct CodeGenConstants {
    pub void_variable: Option<u32>
}

impl CodeGenConstants {
    pub fn new() -> Self {
        Self {
            void_variable: None
        }
    }
}