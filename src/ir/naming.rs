pub struct IRNaming;

impl IRNaming {
    pub const SSA_VALUE_PREFIX: &'static str = "%t";
    pub const BASIC_BLOCK_PREFIX: &'static str = "bb";
    pub const MODULE_PREFIX: &'static str = "module";

    pub fn module_name(name: impl AsRef<str>) -> String {
        format!("{}::{}", Self::MODULE_PREFIX, name.as_ref())
    }

    pub fn function_name(name: impl AsRef<str>) -> String {
        name.as_ref().to_string()
    }

    pub fn basic_block_name(index: usize) -> String {
        format!("{}{}", Self::BASIC_BLOCK_PREFIX, index)
    }

    pub fn temporary_name(index: usize) -> String {
        format!("{}{}", Self::SSA_VALUE_PREFIX, index)
    }

    pub fn parameter_name(index: usize) -> String {
        format!("%p{}", index)
    }
}

#[derive(Debug, Clone)]
pub struct SSAValueGenerator {
    counter: usize,
}

impl SSAValueGenerator {
    pub fn new() -> Self {
        Self { counter: 0 }
    }

    pub fn next_value(&mut self) -> String {
        let name = IRNaming::temporary_name(self.counter);
        self.counter += 1;
        name
    }

    pub fn next_parameter(&mut self) -> String {
        let name = IRNaming::parameter_name(self.counter);
        self.counter += 1;
        name
    }

    pub fn reset(&mut self) {
        self.counter = 0;
    }

    pub fn current_count(&self) -> usize {
        self.counter
    }
}

impl Default for SSAValueGenerator {
    fn default() -> Self {
        Self::new()
    }
}
