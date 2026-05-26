// Persona 2 — Control Flow Graph and basic blocks
// Assigned: Persona2
// Responsibilities: implementación del CFG, creación/gestión de bloques básicos,
// seguimiento de predecesores/sucesores y utilidades de validación.
// Subtasks relevantes: APIs de creación/lookup/insert, traversals y validaciones.

use super::instruction::IRInstruction;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BasicBlockId(pub String);

impl BasicBlockId {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BasicBlock {
    pub id: BasicBlockId,
    pub instructions: Vec<IRInstruction>,
    pub predecessors: Vec<BasicBlockId>,
    pub successors: Vec<BasicBlockId>,
}

impl BasicBlock {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: BasicBlockId::new(id),
            instructions: Vec::new(),
            predecessors: Vec::new(),
            successors: Vec::new(),
        }
    }

    pub fn push_instruction(&mut self, instruction: IRInstruction) {
        self.instructions.push(instruction);
    }

    pub fn add_predecessor(&mut self, predecessor: BasicBlockId) {
        if !self.predecessors.contains(&predecessor) {
            self.predecessors.push(predecessor);
        }
    }

    pub fn add_successor(&mut self, successor: BasicBlockId) {
        if !self.successors.contains(&successor) {
            self.successors.push(successor);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }

    pub fn instruction_count(&self) -> usize {
        self.instructions.len()
    }

    pub fn fmt_display(&self, indent: usize) -> String {
        let indent_str = " ".repeat(indent);
        let next_indent_str = " ".repeat(indent + 2);
        let mut result = format!("{}Block {}:\n", indent_str, self.id.0);

        for instr in &self.instructions {
            result.push_str(&format!(
                "{}{}\n",
                next_indent_str,
                instr.fmt_display()
            ));
        }

        result
    }
}
