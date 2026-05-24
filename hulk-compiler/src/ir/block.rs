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
}
