use std::collections::HashMap;

use crate::ir::block::BasicBlockId;
use crate::ir::instruction::{IRInstruction, IRInstructionKind, IROperand};
use crate::ir::module::IRFunction;
use crate::ir::naming::SSAValueGenerator;
use crate::ir::value::IRValueId;

pub fn run_ssa_renaming(module: &mut crate::ir::module::IRModule) {
    for function in &mut module.functions {
        place_phi_nodes(function);
        renaming_for_function(function);
    }
}

fn place_phi_nodes(function: &mut crate::ir::module::IRFunction) {
    use std::collections::{HashSet, VecDeque};

    // collect block ids and maps
    let blocks: Vec<_> = function.blocks.iter().map(|b| b.id.clone()).collect();
    if blocks.is_empty() {
        return;
    }

    let entry = blocks.first().unwrap().clone();

    // build predecessor and successor maps
    let mut preds: HashMap<_, Vec<_>> = HashMap::new();
    let mut succs: HashMap<_, Vec<_>> = HashMap::new();
    for b in &blocks {
        if let Some(block) = function.block(b) {
            preds.insert(b.clone(), block.predecessors.clone());
            succs.insert(b.clone(), block.successors.clone());
        }
    }

    // compute dominator sets (iterative)
    let mut dom: HashMap<BasicBlockId, HashSet<BasicBlockId>> = HashMap::new();
    for b in &blocks {
        let mut s = HashSet::new();
        if b == &entry {
            s.insert(b.clone());
        } else {
            for x in &blocks {
                s.insert(x.clone());
            }
        }
        dom.insert(b.clone(), s);
    }

    let mut changed = true;
    while changed {
        changed = false;
        for b in &blocks {
            if b == &entry {
                continue;
            }
            let mut new_dom: Option<HashSet<BasicBlockId>> = None;
            for p in preds.get(b).unwrap_or(&Vec::new()) {
                if let Some(pdom) = dom.get(p) {
                    if let Some(existing) = &mut new_dom {
                        let inter: HashSet<_> = existing.intersection(pdom).cloned().collect();
                        *existing = inter;
                    } else {
                        new_dom = Some(pdom.clone());
                    }
                }
            }
            let mut new_set = new_dom.unwrap_or_default();
            new_set.insert(b.clone());
            if new_set != *dom.get(b).unwrap() {
                dom.insert(b.clone(), new_set);
                changed = true;
            }
        }
    }

    // compute immediate dominators (idom)
    let mut idom: HashMap<BasicBlockId, BasicBlockId> = HashMap::new();
    for b in &blocks {
        if b == &entry {
            continue;
        }
        let d = dom.get(b).unwrap();
        // candidates are dominators excluding b
        let mut candidates: Vec<_> = d.iter().filter(|x| *x != b).cloned().collect();
        // choose the one with largest dominator set size
        candidates.sort_by_key(|c| dom.get(c).map(|s| s.len()).unwrap_or(0));
        if let Some(immediate) = candidates.pop() {
            idom.insert(b.clone(), immediate);
        }
    }

    // build dominator tree children
    let mut dom_children: HashMap<BasicBlockId, Vec<BasicBlockId>> = HashMap::new();
    for (n, i) in &idom {
        dom_children.entry(i.clone()).or_default().push(n.clone());
    }

    // compute dominance frontiers
    let mut df: HashMap<BasicBlockId, HashSet<BasicBlockId>> = HashMap::new();
    for n in &blocks {
        df.insert(n.clone(), HashSet::new());
    }

    // local frontiers
    for n in &blocks {
        for s in succs.get(n).unwrap_or(&Vec::new()) {
            if idom.get(s) != Some(n) {
                df.get_mut(n).unwrap().insert(s.clone());
            }
        }
    }

    // propagate from children up
    // process nodes in post-order of dominator tree
    fn post_order(
        node: &BasicBlockId,
        children: &HashMap<BasicBlockId, Vec<BasicBlockId>>,
        order: &mut Vec<BasicBlockId>,
    ) {
        if let Some(ch) = children.get(node) {
            for c in ch {
                post_order(c, children, order);
            }
        }
        order.push(node.clone());
    }

    let mut order = Vec::new();
    post_order(&entry, &dom_children, &mut order);
    for n in order.iter().rev() {
        if let Some(children) = dom_children.get(n) {
            for c in children {
                let c_df = df.get(c).cloned().unwrap_or_default();
                for w in c_df {
                    if idom.get(&w) != Some(n) {
                        df.get_mut(n).unwrap().insert(w);
                    }
                }
            }
        }
    }

    // collect def sites per variable
    let mut defsites: HashMap<String, HashSet<BasicBlockId>> = HashMap::new();
    // parameters: treat parameter id as variable name, defined in entry
    for param in &function.parameters {
        let varname = param.id.0.clone();
        defsites.entry(varname).or_default().insert(entry.clone());
    }

    for b in &blocks {
        if let Some(block) = function.block(b) {
            for instr in &block.instructions {
                if let IRInstructionKind::Assign {
                    target: _,
                    value: _,
                    original,
                } = &instr.kind
                    && let Some(var) = original
                {
                    defsites.entry(var.clone()).or_default().insert(b.clone());
                }
            }
        }
    }

    // place phi nodes per variable using classic algorithm
    for (var, sites) in defsites.into_iter() {
        let mut work: VecDeque<BasicBlockId> = VecDeque::new();
        let mut has_phi: HashSet<BasicBlockId> = HashSet::new();
        for s in &sites {
            work.push_back(s.clone());
        }

        while let Some(n) = work.pop_front() {
            if let Some(n_df) = df.get(&n) {
                for y in n_df.iter() {
                    if !has_phi.contains(y) {
                        // insert phi in block y
                        if let Some(block_mut) = function.block_mut(y) {
                            let incoming = block_mut
                                .predecessors
                                .iter()
                                .map(|p| (IRValueId::new(var.clone()), p.clone()))
                                .collect();
                            let phi_target =
                                crate::ir::value::IRValueId::new("%phi".to_string() + &y.0);
                            let phi_instr = IRInstruction::new(IRInstructionKind::Phi {
                                target: phi_target.clone(),
                                incoming,
                                original: Some(var.clone()),
                            });
                            block_mut.instructions.insert(0, phi_instr);
                        }
                        has_phi.insert(y.clone());
                        if !sites.contains(y) {
                            work.push_back(y.clone());
                        }
                    }
                }
            }
        }
    }
}

fn remap_operand(
    op: &IROperand,
    map: &mut HashMap<String, String>,
    svgen: &mut SSAValueGenerator,
) -> IROperand {
    match op {
        IROperand::Value(id) => {
            if let Some(new) = map.get(&id.0) {
                IROperand::Value(IRValueId::new(new.clone()))
            } else {
                // assign a new name for referenced-but-unseen value
                let new_name = svgen.next_value();
                map.insert(id.0.clone(), new_name.clone());
                IROperand::Value(IRValueId::new(new_name))
            }
        }
        IROperand::Integer(i) => IROperand::Integer(*i),
        IROperand::Float(f) => IROperand::Float(*f),
        IROperand::Boolean(b) => IROperand::Boolean(*b),
        IROperand::Text(s) => IROperand::Text(s.clone()),
    }
}

fn renaming_for_function(function: &mut IRFunction) {
    let mut svgen = SSAValueGenerator::new();
    let mut map: HashMap<String, String> = HashMap::new();

    // map parameters first
    for param in &mut function.parameters {
        let old = param.id.0.clone();
        let new = svgen.next_parameter();
        map.insert(old.clone(), new.clone());
        param.id = IRValueId::new(new);
    }

    for block in &mut function.blocks {
        for instr in &mut block.instructions {
            // remap uses inside the instruction
            match &mut instr.kind {
                IRInstructionKind::Assign {
                    target: _,
                    value,
                    original: _,
                } => {
                    *value = remap_operand(value, &mut map, &mut svgen);
                }
                IRInstructionKind::Binary { left, right, .. } => {
                    *left = remap_operand(left, &mut map, &mut svgen);
                    *right = remap_operand(right, &mut map, &mut svgen);
                }
                IRInstructionKind::Unary { operand, .. } => {
                    *operand = remap_operand(operand, &mut map, &mut svgen);
                }
                IRInstructionKind::Phi { incoming, .. } => {
                    for (v, _b) in incoming.iter_mut() {
                        if let Some(new) = map.get(&v.0) {
                            *v = IRValueId::new(new.clone());
                        } else {
                            let new = svgen.next_value();
                            map.insert(v.0.clone(), new.clone());
                            *v = IRValueId::new(new);
                        }
                    }
                }
                IRInstructionKind::Branch { condition, .. } => {
                    *condition = remap_operand(condition, &mut map, &mut svgen);
                }
                IRInstructionKind::Return(Some(op)) => {
                    *op = remap_operand(op, &mut map, &mut svgen);
                }
                IRInstructionKind::Return(None) => {}
                IRInstructionKind::Call {
                    arguments,
                    target: _,
                    original: _,
                    ..
                } => {
                    for arg in arguments.iter_mut() {
                        *arg = remap_operand(arg, &mut map, &mut svgen);
                    }
                }
                _ => {}
            }

            // now remap the defined target if any
            if let Some(target) = instr.defines_value() {
                let old = target.0.clone();
                let new = svgen.next_value();
                map.insert(old.clone(), new.clone());

                // replace the target inside the instruction kind
                match &mut instr.kind {
                    IRInstructionKind::Assign { target, .. }
                    | IRInstructionKind::Binary { target, .. }
                    | IRInstructionKind::Unary { target, .. }
                    | IRInstructionKind::Phi { target, .. } => {
                        *target = IRValueId::new(new);
                    }
                    IRInstructionKind::Call {
                        target: Some(t), ..
                    } => {
                        *t = IRValueId::new(new);
                    }
                    _ => {}
                }
            }
        }
    }
}
