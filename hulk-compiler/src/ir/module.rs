// Persona 1 — SSA IR design and core representation
// Assigned: Persona1
// Responsibilities: Diseño de tipos núcleo (Module, Function), estructura de funciones
// y la representación de alto nivel que usarán SSA y el backend.
// Subtasks relevantes: definir `IRModule`, `IRFunction`, API de builder.

use super::block::{BasicBlock, BasicBlockId};
use super::value::IRValue;

#[derive(Debug, Clone, PartialEq)]
pub struct IRFunction {
    pub name: String,
    pub parameters: Vec<IRValue>,
    pub return_type: Option<String>,
    pub blocks: Vec<BasicBlock>,
}

impl IRFunction {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            parameters: Vec::new(),
            return_type: None,
            blocks: Vec::new(),
        }
    }

    pub fn add_block(&mut self, block: BasicBlock) {
        self.blocks.push(block);
    }

    pub fn entry_block(&self) -> Option<&BasicBlock> {
        self.blocks.first()
    }

    pub fn block(&self, id: &BasicBlockId) -> Option<&BasicBlock> {
        self.blocks.iter().find(|block| &block.id == id)
    }

    pub fn block_mut(&mut self, id: &BasicBlockId) -> Option<&mut BasicBlock> {
        self.blocks.iter_mut().find(|block| &block.id == id)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IRModule {
    pub name: String,
    pub functions: Vec<IRFunction>,
}

impl IRModule {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            functions: Vec::new(),
        }
    }

    pub fn add_function(&mut self, function: IRFunction) {
        self.functions.push(function);
    }

    pub fn function(&self, name: &str) -> Option<&IRFunction> {
        self.functions.iter().find(|function| function.name == name)
    }

    pub fn function_mut(&mut self, name: &str) -> Option<&mut IRFunction> {
        self.functions.iter_mut().find(|function| function.name == name)
    }
}
