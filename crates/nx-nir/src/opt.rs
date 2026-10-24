use crate::ir::*;
use std::collections::{HashMap, HashSet};

/// Optimization level for NIR passes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum OptLevel {
    #[default]
    O0,
    O1,
    O2,
    O3,
}

impl OptLevel {
    pub fn from_u32(n: u32) -> Self {
        match n {
            0 => OptLevel::O0,
            1 => OptLevel::O1,
            2 => OptLevel::O2,
            _ => OptLevel::O3,
        }
    }
}

/// Run NIR optimization passes according to `level`.
pub fn optimize_module(module: &mut NirModule, level: OptLevel) {
    if level == OptLevel::O0 {
        return;
    }

    for func in &mut module.functions {
        optimize_function(func, level);
    }
}

fn optimize_function(func: &mut NirFunction, level: OptLevel) {
    let mut changed = true;
    let mut passes = 0;

    while changed && passes < 10 {
        changed = false;
        passes += 1;

        if fold_constants(func) {
            changed = true;
        }

        if simplify_branches(func) {
            changed = true;
        }

        if eliminate_unreachable_blocks(func) {
            changed = true;
        }

        if level >= OptLevel::O2 && eliminate_dead_allocas(func) {
            changed = true;
        }
    }
}

/// Constant folding pass: computes operations on constants at compile time.
fn fold_constants(func: &mut NirFunction) -> bool {
    let mut changed = false;
    let mut const_map: HashMap<String, NirConst> = HashMap::new();

    for block in &mut func.blocks {
        let mut new_instructions = Vec::new();

        for mut inst in block.instructions.drain(..) {
            substitute_inst(&mut inst, &const_map);

            match inst {
                NirInst::BinOp {
                    dest,
                    op,
                    lhs,
                    rhs,
                    ty,
                } => {
                    let lhs_c = resolve_const(&lhs, &const_map);
                    let rhs_c = resolve_const(&rhs, &const_map);

                    if let (Some(NirConst::Int(l, _)), Some(NirConst::Int(r, _))) = (lhs_c, rhs_c) {
                        let res = match op {
                            NirBinaryOp::Add => Some(l.wrapping_add(r)),
                            NirBinaryOp::Sub => Some(l.wrapping_sub(r)),
                            NirBinaryOp::Mul => Some(l.wrapping_mul(r)),
                            NirBinaryOp::Div if r != 0 => Some(l.wrapping_div(r)),
                            NirBinaryOp::Mod if r != 0 => Some(l.wrapping_rem(r)),
                            NirBinaryOp::BitAnd => Some(l & r),
                            NirBinaryOp::BitOr => Some(l | r),
                            NirBinaryOp::BitXor => Some(l ^ r),
                            NirBinaryOp::Shl => Some(l.wrapping_shl(r as u32)),
                            NirBinaryOp::Shr => Some(l.wrapping_shr(r as u32)),
                            _ => None,
                        };

                        if let Some(val) = res {
                            const_map.insert(dest.clone(), NirConst::Int(val, ty.clone()));
                            changed = true;
                            continue;
                        }
                    }

                    new_instructions.push(NirInst::BinOp {
                        dest,
                        op,
                        lhs,
                        rhs,
                        ty,
                    });
                }
                NirInst::Cmp { dest, op, lhs, rhs } => {
                    let lhs_c = resolve_const(&lhs, &const_map);
                    let rhs_c = resolve_const(&rhs, &const_map);

                    if let (Some(NirConst::Int(l, _)), Some(NirConst::Int(r, _))) = (lhs_c, rhs_c) {
                        let res = match op {
                            NirCmpOp::Eq => l == r,
                            NirCmpOp::Ne => l != r,
                            NirCmpOp::Lt => l < r,
                            NirCmpOp::Le => l <= r,
                            NirCmpOp::Gt => l > r,
                            NirCmpOp::Ge => l >= r,
                        };
                        const_map.insert(dest.clone(), NirConst::Bool(res));
                        changed = true;
                        continue;
                    }

                    new_instructions.push(NirInst::Cmp { dest, op, lhs, rhs });
                }
                NirInst::UnaryOp {
                    dest,
                    op,
                    operand,
                    ty,
                } => {
                    let op_c = resolve_const(&operand, &const_map);
                    if let Some(NirConst::Bool(b)) = op_c {
                        if op == NirUnaryOp::Not {
                            const_map.insert(dest.clone(), NirConst::Bool(!b));
                            changed = true;
                            continue;
                        }
                    } else if let Some(NirConst::Int(n, _)) = op_c {
                        if op == NirUnaryOp::Neg {
                            const_map.insert(dest.clone(), NirConst::Int(-n, ty.clone()));
                            changed = true;
                            continue;
                        }
                        if op == NirUnaryOp::BitNot {
                            const_map.insert(dest.clone(), NirConst::Int(!n, ty.clone()));
                            changed = true;
                            continue;
                        }
                    }
                    new_instructions.push(NirInst::UnaryOp {
                        dest,
                        op,
                        operand,
                        ty,
                    });
                }
                other => new_instructions.push(other),
            }
        }

        block.instructions = new_instructions;

        match &mut block.terminator {
            NirTerminator::Return(Some(ref mut op)) => substitute_operand(op, &const_map),
            NirTerminator::CondBranch { ref mut cond, .. } => substitute_operand(cond, &const_map),
            _ => {}
        }
    }

    changed
}

fn substitute_operand(op: &mut NirOperand, map: &HashMap<String, NirConst>) {
    if let NirOperand::Var(name) = op {
        if let Some(c) = map.get(name) {
            *op = NirOperand::Const(c.clone());
        }
    }
}

fn substitute_inst(inst: &mut NirInst, map: &HashMap<String, NirConst>) {
    match inst {
        NirInst::Store { src, .. } => substitute_operand(src, map),
        NirInst::BinOp { lhs, rhs, .. } => {
            substitute_operand(lhs, map);
            substitute_operand(rhs, map);
        }
        NirInst::Cmp { lhs, rhs, .. } => {
            substitute_operand(lhs, map);
            substitute_operand(rhs, map);
        }
        NirInst::UnaryOp { operand, .. } => substitute_operand(operand, map),
        NirInst::Call { args, .. } => {
            for a in args {
                substitute_operand(a, map);
            }
        }
        NirInst::Vout { args } => {
            for (a, _) in args {
                substitute_operand(a, map);
            }
        }
        NirInst::Alloc { size_bytes, .. } => substitute_operand(size_bytes, map),
        NirInst::Free { ptr } => substitute_operand(ptr, map),
        NirInst::ToInt { src, .. } => substitute_operand(src, map),
        NirInst::ToFloat { src, .. } => substitute_operand(src, map),
        NirInst::Len { src, .. } => substitute_operand(src, map),
        _ => {}
    }
}

fn resolve_const(op: &NirOperand, map: &HashMap<String, NirConst>) -> Option<NirConst> {
    match op {
        NirOperand::Const(c) => Some(c.clone()),
        NirOperand::Var(name) => map.get(name).cloned(),
    }
}

/// Simplify conditional branches with known constant conditions.
fn simplify_branches(func: &mut NirFunction) -> bool {
    let mut changed = false;

    for block in &mut func.blocks {
        if let NirTerminator::CondBranch {
            cond: NirOperand::Const(NirConst::Bool(b)),
            ref then_block,
            ref else_block,
        } = block.terminator
        {
            let target = if b {
                then_block.clone()
            } else {
                else_block.clone()
            };
            block.terminator = NirTerminator::Branch(target);
            changed = true;
        }
    }

    changed
}

/// Eliminate blocks that are unreachable from the entry block.
fn eliminate_unreachable_blocks(func: &mut NirFunction) -> bool {
    if func.blocks.is_empty() {
        return false;
    }

    let mut reachable = HashSet::new();
    let mut worklist = vec![func.blocks[0].label.clone()];
    reachable.insert(func.blocks[0].label.clone());

    let block_map: HashMap<String, &NirBlock> =
        func.blocks.iter().map(|b| (b.label.clone(), b)).collect();

    while let Some(label) = worklist.pop() {
        if let Some(block) = block_map.get(&label) {
            match &block.terminator {
                NirTerminator::Branch(target) => {
                    if reachable.insert(target.clone()) {
                        worklist.push(target.clone());
                    }
                }
                NirTerminator::CondBranch {
                    then_block,
                    else_block,
                    ..
                } => {
                    if reachable.insert(then_block.clone()) {
                        worklist.push(then_block.clone());
                    }
                    if reachable.insert(else_block.clone()) {
                        worklist.push(else_block.clone());
                    }
                }
                _ => {}
            }
        }
    }

    let initial_len = func.blocks.len();
    func.blocks.retain(|b| reachable.contains(&b.label));
    initial_len != func.blocks.len()
}

/// Remove allocas that are never loaded or stored.
fn eliminate_dead_allocas(_func: &mut NirFunction) -> bool {
    // Basic dead alloca elimination placeholder for O2+
    false
}
