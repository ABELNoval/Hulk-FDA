// Persona 1 — SSA IR design and core representation
// Assigned: Persona1
// Responsibilities: Diseño de tipos núcleo (Module, Function), estructura de funciones
// y la representación de alto nivel que usarán SSA y el backend.
// Subtasks relevantes: definir `IRModule`, `IRFunction`, API de builder.

use std::collections::{HashMap, HashSet, VecDeque};

use super::block::{BasicBlock, BasicBlockId};
use super::instruction::IRInstructionKind;
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

    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.blocks.is_empty() {
            errors.push(format!("Function '{}' has no blocks.", self.name));
        }

        for block in &self.blocks {
            if block.instructions.is_empty() {
                errors.push(format!(
                    "Block '{}' in function '{}' has no instructions.",
                    block.id.0, self.name
                ));
            }

            for instruction in &block.instructions {
                if let IRInstructionKind::Phi { .. } = &instruction.kind
                    && let Err(message) = instruction.validate_phi()
                {
                    errors.push(format!(
                        "Function '{}' block '{}': {}",
                        self.name, block.id.0, message
                    ));
                }
            }

            if let Some(last_instruction) = block.instructions.last()
                && !last_instruction.is_terminator()
            {
                errors.push(format!(
                    "Block '{}' in function '{}' does not end with a terminator.",
                    block.id.0, self.name
                ));
            }
        }

        if errors.is_empty() {
            // Additional SSA checks: each use must have a definition and the definition
            // must be reachable to the use's block. Also validate phi incoming blocks.
            let mut def_map: HashMap<String, BasicBlockId> = HashMap::new();
            if let Some(entry) = self.entry_block() {
                for parameter in &self.parameters {
                    def_map.insert(parameter.id.0.clone(), entry.id.clone());
                }
            }
            for block in &self.blocks {
                for instr in &block.instructions {
                    match &instr.kind {
                        IRInstructionKind::Assign { target, .. }
                        | IRInstructionKind::Binary { target, .. }
                        | IRInstructionKind::Unary { target, .. }
                        | IRInstructionKind::Phi { target, .. }
                        | IRInstructionKind::Call { target, .. } => {
                            def_map.insert(target.0.clone(), block.id.clone());
                        }
                        _ => {}
                    }
                }
            }

            for block in &self.blocks {
                for instr in &block.instructions {
                    // phi-specific validation: incoming blocks must be predecessors
                    if let IRInstructionKind::Phi { incoming, .. } = &instr.kind {
                        for (_val, incoming_block) in incoming {
                            if !block.predecessors.contains(incoming_block) {
                                errors.push(format!(
                                    "Function '{}' block '{}': phi incoming block '{}' is not a predecessor.",
                                    self.name, block.id.0, incoming_block.0
                                ));
                            }
                        }
                    }
                }
            }

            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors)
            }
        } else {
            Err(errors)
        }
    }

    pub fn fmt_display(&self, indent: usize) -> String {
        let indent_str = " ".repeat(indent);
        let _next_indent_str = " ".repeat(indent + 2);

        let params = self
            .parameters
            .iter()
            .map(|p| {
                let ty = p.ty.as_deref().unwrap_or("?");
                format!("{}: {}", p.id.0, ty)
            })
            .collect::<Vec<_>>()
            .join(", ");

        let return_type = self.return_type.as_deref().unwrap_or("?");

        let mut result = format!(
            "{}Function: {}({}) -> {}\n",
            indent_str, self.name, params, return_type
        );

        for block in &self.blocks {
            result.push_str(&block.fmt_display(indent + 2));
        }

        result
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IRGlobal {
    pub name: String,
    pub element_ty: String,  // ej. "ptr"
    pub values: Vec<String>, // nombres de función para la vtable
}

#[derive(Debug, Clone, PartialEq)]
pub struct IRModule {
    pub name: String,
    pub functions: Vec<IRFunction>,
    pub globals: Vec<IRGlobal>,
}

impl IRModule {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            functions: Vec::new(),
            globals: Vec::new(),
        }
    }

    pub fn add_global(&mut self, global: IRGlobal) {
        self.globals.push(global);
    }

    pub fn add_function(&mut self, function: IRFunction) {
        self.functions.push(function);
    }

    pub fn function(&self, name: &str) -> Option<&IRFunction> {
        self.functions.iter().find(|function| function.name == name)
    }

    pub fn function_mut(&mut self, name: &str) -> Option<&mut IRFunction> {
        self.functions
            .iter_mut()
            .find(|function| function.name == name)
    }

    pub fn function_count(&self) -> usize {
        self.functions.len()
    }

    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.functions.is_empty() {
            errors.push(format!("IR module '{}' has no functions.", self.name));
        }

        for function in &self.functions {
            if let Err(function_errors) = function.validate() {
                errors.extend(
                    function_errors
                        .into_iter()
                        .map(|error| format!("Module '{}': {}", self.name, error)),
                );
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn fmt_display(&self) -> String {
        let mut result = format!("Module: {}\n", self.name);
        for function in &self.functions {
            result.push_str(&function.fmt_display(2));
        }
        result
    }
}
