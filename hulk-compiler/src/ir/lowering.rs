// Persona 3 — AST lowering into IR
// Assigned: Persona3
// Responsibilities: implementar la pasarela que convierte `Program` (AST)
// en `IRModule`. Manejar lowering de expresiones, variables, control flow y
// conectar con el IR builder sin exponer detalles internos.

use crate::parser::ast::Program;

use super::block::BasicBlock;
use super::instruction::IRInstruction;
use super::module::{IRFunction, IRModule};
use super::value::IRValue;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IRLoweringError {
    pub message: String,
    pub location: Option<String>,
}

impl IRLoweringError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            location: None,
        }
    }

    pub fn at(message: impl Into<String>, location: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            location: Some(location.into()),
        }
    }
}

impl std::fmt::Display for IRLoweringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.location {
            Some(location) => write!(f, "{} ({})", self.message, location),
            None => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for IRLoweringError {}

pub trait IRLoweringContext {
    fn lower_program(&mut self, program: &Program) -> Result<IRModule, IRLoweringError>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct IRLoweringResult {
    pub module: IRModule,
    pub warnings: Vec<String>,
}

impl IRLoweringResult {
    pub fn new(module: IRModule) -> Self {
        Self {
            module,
            warnings: Vec::new(),
        }
    }

    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }
}

#[derive(Debug)]
pub struct IRBuilder {
    module: IRModule,
    current_function: Option<String>,
}

impl IRBuilder {
    pub fn new(module_name: impl Into<String>) -> Self {
        Self {
            module: IRModule::new(module_name),
            current_function: None,
        }
    }

    pub fn create_function(
        &mut self,
        name: impl Into<String>,
        parameters: Vec<IRValue>,
        return_type: Option<String>,
    ) -> Result<(), IRLoweringError> {
        let name_str = name.into();

        if self.module.function(&name_str).is_some() {
            return Err(IRLoweringError::new(format!(
                "Function '{}' already exists",
                name_str
            )));
        }

        let mut func = IRFunction::new(name_str.clone());
        func.parameters = parameters;
        func.return_type = return_type;

        self.module.add_function(func);
        self.current_function = Some(name_str);
        Ok(())
    }

    pub fn create_block_in_function(
        &mut self,
        function_name: impl AsRef<str>,
        block_name: impl Into<String>,
    ) -> Result<(), IRLoweringError> {
        let func_name = function_name.as_ref();
        let block_name_str = block_name.into();

        let func = self
            .module
            .function_mut(func_name)
            .ok_or_else(|| IRLoweringError::new(format!("Function '{}' not found", func_name)))?;

        if func.block(&super::block::BasicBlockId::new(&block_name_str)).is_some() {
            return Err(IRLoweringError::new(format!(
                "Block '{}' already exists in function '{}'",
                block_name_str, func_name
            )));
        }

        let block = BasicBlock::new(block_name_str);
        func.add_block(block);
        Ok(())
    }

    pub fn add_instruction_to_block(
        &mut self,
        function_name: impl AsRef<str>,
        block_name: impl AsRef<str>,
        instruction: IRInstruction,
    ) -> Result<(), IRLoweringError> {
        let func_name = function_name.as_ref();
        let block_name_str = block_name.as_ref();

        let func = self
            .module
            .function_mut(func_name)
            .ok_or_else(|| IRLoweringError::new(format!("Function '{}' not found", func_name)))?;

        let block_id = super::block::BasicBlockId::new(block_name_str);
        let block = func.block_mut(&block_id).ok_or_else(|| {
            IRLoweringError::new(format!(
                "Block '{}' not found in function '{}'",
                block_name_str, func_name
            ))
        })?;

        block.push_instruction(instruction);
        Ok(())
    }

    pub fn set_current_function(&mut self, name: impl Into<String>) {
        self.current_function = Some(name.into());
    }

    pub fn current_function(&self) -> Option<&str> {
        self.current_function.as_deref()
    }

    pub fn function_exists(&self, name: &str) -> bool {
        self.module.function(name).is_some()
    }

    pub fn block_exists(&self, function_name: &str, block_name: &str) -> bool {
        if let Some(func) = self.module.function(function_name) {
            func.block(&super::block::BasicBlockId::new(block_name)).is_some()
        } else {
            false
        }
    }

    pub fn function_count(&self) -> usize {
        self.module.function_count()
    }

    pub fn build(self) -> IRModule {
        self.module
    }

    pub fn fmt_display(&self) -> String {
        self.module.fmt_display()
    }
}
