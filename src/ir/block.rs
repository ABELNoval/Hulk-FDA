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
            result.push_str(&format!("{}{}\n", next_indent_str, instr.fmt_display()));
        }

        result
    }
}

#[derive(Debug, Clone, Default)]
pub struct ControlFlowGraph {
    pub blocks: std::collections::HashMap<BasicBlockId, BasicBlock>,
    pub entry: Option<BasicBlockId>,
}

impl ControlFlowGraph {
    pub fn new() -> Self {
        Self {
            blocks: std::collections::HashMap::new(),
            entry: None,
        }
    }

    pub fn create_block(&mut self, name: impl Into<String>) -> BasicBlockId {
        let id = BasicBlockId::new(name);
        let block = BasicBlock::new(id.0.clone());

        if self.entry.is_none() {
            self.entry = Some(id.clone());
        }

        self.blocks.insert(id.clone(), block);
        id
    }

    pub fn insert_block(&mut self, block: BasicBlock) {
        if self.entry.is_none() {
            self.entry = Some(block.id.clone());
        }
        self.blocks.insert(block.id.clone(), block);
    }

    pub fn block(&self, id: &BasicBlockId) -> Option<&BasicBlock> {
        self.blocks.get(id)
    }

    pub fn block_mut(&mut self, id: &BasicBlockId) -> Option<&mut BasicBlock> {
        self.blocks.get_mut(id)
    }

    pub fn add_edge(&mut self, from: &BasicBlockId, to: &BasicBlockId) {
        if let Some(from_block) = self.blocks.get_mut(from) {
            from_block.add_successor(to.clone());
        }
        if let Some(to_block) = self.blocks.get_mut(to) {
            to_block.add_predecessor(from.clone());
        }
    }

    pub fn remove_edge(&mut self, from: &BasicBlockId, to: &BasicBlockId) {
        if let Some(from_block) = self.blocks.get_mut(from) {
            from_block.successors.retain(|s| s != to);
        }
        if let Some(to_block) = self.blocks.get_mut(to) {
            to_block.predecessors.retain(|p| p != from);
        }
    }

    pub fn predecessors(&self, id: &BasicBlockId) -> Option<&[BasicBlockId]> {
        self.blocks.get(id).map(|b| b.predecessors.as_slice())
    }

    pub fn successors(&self, id: &BasicBlockId) -> Option<&[BasicBlockId]> {
        self.blocks.get(id).map(|b| b.successors.as_slice())
    }

    pub fn build_if_else(
        &mut self,
        current: &BasicBlockId,
        then_name: impl Into<String>,
        else_name: impl Into<String>,
        merge_name: impl Into<String>,
    ) -> (BasicBlockId, BasicBlockId, BasicBlockId) {
        let then_block = self.create_block(then_name);
        let else_block = self.create_block(else_name);
        let merge_block = self.create_block(merge_name);

        self.add_edge(current, &then_block);
        self.add_edge(current, &else_block);
        self.add_edge(&then_block, &merge_block);
        self.add_edge(&else_block, &merge_block);

        (then_block, else_block, merge_block)
    }

    pub fn build_loop(
        &mut self,
        current: &BasicBlockId,
        cond_name: impl Into<String>,
        body_name: impl Into<String>,
        exit_name: impl Into<String>,
    ) -> (BasicBlockId, BasicBlockId, BasicBlockId) {
        let cond_block = self.create_block(cond_name);
        let body_block = self.create_block(body_name);
        let exit_block = self.create_block(exit_name);

        self.add_edge(current, &cond_block);
        self.add_edge(&cond_block, &body_block);
        self.add_edge(&cond_block, &exit_block); // Rama de salida del loop
        self.add_edge(&body_block, &cond_block); // Backedge para repetir

        (cond_block, body_block, exit_block)
    }

    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if !self.blocks.is_empty() {
            if let Some(entry) = &self.entry {
                if !self.blocks.contains_key(entry) {
                    errors.push(format!(
                        "Entry block '{}' does not exist in the graph.",
                        entry.0
                    ));
                }
            } else {
                errors.push("Control flow graph has blocks but no entry block is set.".to_string());
            }
        }

        for (id, block) in &self.blocks {
            // Validate predecessors
            for pred in &block.predecessors {
                if let Some(pred_block) = self.blocks.get(pred) {
                    if !pred_block.successors.contains(id) {
                        errors.push(format!(
                            "Edge mismatch: Block '{}' claims '{}' as predecessor, but '{}' doesn't list it as successor.",
                            id.0, pred.0, pred.0
                        ));
                    }
                } else {
                    errors.push(format!(
                        "Block '{}' has non-existent predecessor '{}'.",
                        id.0, pred.0
                    ));
                }
            }

            // Validate successors
            for succ in &block.successors {
                if let Some(succ_block) = self.blocks.get(succ) {
                    if !succ_block.predecessors.contains(id) {
                        errors.push(format!(
                            "Edge mismatch: Block '{}' claims '{}' as successor, but '{}' doesn't list it as predecessor.",
                            id.0, succ.0, succ.0
                        ));
                    }
                } else {
                    errors.push(format!(
                        "Block '{}' has non-existent successor '{}'.",
                        id.0, succ.0
                    ));
                }
            }
        }

        if let Some(entry) = &self.entry {
            let reachable: std::collections::HashSet<_> =
                self.dfs_traversal().into_iter().collect();
            for id in self.blocks.keys() {
                if !reachable.contains(id) {
                    errors.push(format!(
                        "Block '{}' is unreachable from entry '{}'.",
                        id.0, entry.0
                    ));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn dfs_traversal(&self) -> Vec<BasicBlockId> {
        let mut visited = std::collections::HashSet::new();
        let mut order = Vec::new();

        if let Some(entry) = &self.entry {
            self.dfs_recursive(entry, &mut visited, &mut order);
        }

        order
    }

    fn dfs_recursive(
        &self,
        current: &BasicBlockId,
        visited: &mut std::collections::HashSet<BasicBlockId>,
        order: &mut Vec<BasicBlockId>,
    ) {
        if visited.insert(current.clone()) {
            order.push(current.clone());
            if let Some(successors) = self.successors(current) {
                for succ in successors {
                    self.dfs_recursive(succ, visited, order);
                }
            }
        }
    }

    pub fn post_order_traversal(&self) -> Vec<BasicBlockId> {
        let mut visited = std::collections::HashSet::new();
        let mut order = Vec::new();

        if let Some(entry) = &self.entry {
            self.post_order_recursive(entry, &mut visited, &mut order);
        }

        order
    }

    fn post_order_recursive(
        &self,
        current: &BasicBlockId,
        visited: &mut std::collections::HashSet<BasicBlockId>,
        order: &mut Vec<BasicBlockId>,
    ) {
        if visited.insert(current.clone()) {
            if let Some(successors) = self.successors(current) {
                for succ in successors {
                    if !visited.contains(succ) {
                        self.post_order_recursive(succ, visited, order);
                    }
                }
            }
            order.push(current.clone());
        }
    }

    pub fn bfs_traversal(&self) -> Vec<BasicBlockId> {
        let mut visited = std::collections::HashSet::new();
        let mut order = Vec::new();
        let mut queue = std::collections::VecDeque::new();

        if let Some(entry) = &self.entry {
            queue.push_back(entry.clone());
            visited.insert(entry.clone());
        }

        while let Some(current) = queue.pop_front() {
            order.push(current.clone());
            if let Some(successors) = self.successors(&current) {
                for succ in successors {
                    if visited.insert(succ.clone()) {
                        queue.push_back(succ.clone());
                    }
                }
            }
        }

        order
    }
}
