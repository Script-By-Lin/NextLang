use crate::ir::*;
use std::fmt::Write;

/// Pretty-prints a `NirModule` into human-readable textual NIR.
pub fn print_module(module: &NirModule) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "; NIR Module: {}", module.name);

    if !module.string_literals.is_empty() {
        let _ = writeln!(out, "\n; String Literals:");
        for (label, s) in &module.string_literals {
            let _ = writeln!(out, "  const {} = {:?};", label, s);
        }
    }

    if !module.classes.is_empty() {
        let _ = writeln!(out, "\n; Classes:");
        for (name, layout) in &module.classes {
            let parent_str = layout
                .parent
                .as_ref()
                .map(|p| format!(" extends {}", p))
                .unwrap_or_default();
            let _ = writeln!(
                out,
                "  class {}{} (size: {}B) {{",
                name, parent_str, layout.total_size_bytes
            );
            for (field, ty) in &layout.fields {
                let offset = layout.field_offsets.get(field).copied().unwrap_or(0);
                let _ = writeln!(out, "    +{}: {:?} (offset: {})", field, ty, offset);
            }
            for m in &layout.methods {
                let _ = writeln!(out, "    method {}", m);
            }
            let _ = writeln!(out, "  }}");
        }
    }

    for func in &module.functions {
        let _ = writeln!(out);
        print_function(&mut out, func);
    }

    out
}

pub fn print_function(out: &mut String, func: &NirFunction) {
    let params: Vec<String> = func
        .params
        .iter()
        .map(|(name, ty)| format!("{}: {:?}", name, ty))
        .collect();

    let entry_tag = if func.is_entry { " [entry]" } else { "" };
    let _ = writeln!(
        out,
        "fn {}({}) -> {:?}{} {{",
        func.name,
        params.join(", "),
        func.ret_ty,
        entry_tag
    );

    for block in &func.blocks {
        let _ = writeln!(out, "{}:", block.label);
        for inst in &block.instructions {
            let _ = writeln!(out, "    {}", format_instruction(inst));
        }
        let _ = writeln!(out, "    {}", format_terminator(&block.terminator));
    }

    let _ = writeln!(out, "}}");
}

fn format_instruction(inst: &NirInst) -> String {
    match inst {
        NirInst::Alloca { dest, ty } => format!("{} = alloca {:?}", dest, ty),
        NirInst::Load { dest, src_ptr, ty } => format!("{} = load {:?}, *{}", dest, ty, src_ptr),
        NirInst::Store { dest_ptr, src } => format!("store {}, *{}", format_operand(src), dest_ptr),
        NirInst::GetFieldPtr {
            dest,
            base_ptr,
            byte_offset,
            ty,
        } => format!(
            "{} = getfieldptr {:?}, *{} + {}B",
            dest, ty, base_ptr, byte_offset
        ),
        NirInst::BinOp {
            dest,
            op,
            lhs,
            rhs,
            ty,
        } => format!(
            "{} = binop {:?} {:?}, {}, {}",
            dest,
            op,
            ty,
            format_operand(lhs),
            format_operand(rhs)
        ),
        NirInst::Cmp { dest, op, lhs, rhs } => format!(
            "{} = cmp {:?}, {}, {}",
            dest,
            op,
            format_operand(lhs),
            format_operand(rhs)
        ),
        NirInst::UnaryOp {
            dest,
            op,
            operand,
            ty,
        } => format!(
            "{} = unop {:?} {:?}, {}",
            dest,
            op,
            ty,
            format_operand(operand)
        ),
        NirInst::Call {
            dest,
            callee,
            args,
            ret_ty,
        } => {
            let args_str: Vec<String> = args.iter().map(format_operand).collect();
            if let Some(d) = dest {
                format!(
                    "{} = call {}({}) -> {:?}",
                    d,
                    callee,
                    args_str.join(", "),
                    ret_ty
                )
            } else {
                format!("call {}({}) -> {:?}", callee, args_str.join(", "), ret_ty)
            }
        }
        NirInst::Alloc { dest, size_bytes } => {
            format!("{} = alloc({})", dest, format_operand(size_bytes))
        }
        NirInst::Free { ptr } => format!("free({})", format_operand(ptr)),
        NirInst::Vout { args } => {
            let args_str: Vec<String> = args
                .iter()
                .map(|(op, ty)| format!("{}: {:?}", format_operand(op), ty))
                .collect();
            format!("vout({})", args_str.join(", "))
        }
        NirInst::Input { dest, prompt } => {
            if let Some(p) = prompt {
                format!("{} = input({:?})", dest, p)
            } else {
                format!("{} = input()", dest)
            }
        }
        NirInst::ToInt { dest, src, src_ty } => {
            format!("{} = to_int({}: {:?})", dest, format_operand(src), src_ty)
        }
        NirInst::ToFloat { dest, src, src_ty } => {
            format!("{} = to_float({}: {:?})", dest, format_operand(src), src_ty)
        }
        NirInst::Len { dest, src, src_ty } => {
            format!("{} = len({}: {:?})", dest, format_operand(src), src_ty)
        }
    }
}

fn format_terminator(term: &NirTerminator) -> String {
    match term {
        NirTerminator::Return(Some(op)) => format!("ret {}", format_operand(op)),
        NirTerminator::Return(None) => "ret".to_string(),
        NirTerminator::Branch(target) => format!("br label %{}", target),
        NirTerminator::CondBranch {
            cond,
            then_block,
            else_block,
        } => format!(
            "br {}, label %{}, label %{}",
            format_operand(cond),
            then_block,
            else_block
        ),
        NirTerminator::Panic(msg) => format!("panic {:?}", msg),
    }
}

fn format_operand(op: &NirOperand) -> String {
    match op {
        NirOperand::Var(name) => format!("%{}", name),
        NirOperand::Const(c) => match c {
            NirConst::Int(n, ty) => format!("{}: {:?}", n, ty),
            NirConst::Float(f, ty) => format!("{}: {:?}", f, ty),
            NirConst::Bool(b) => format!("{}", b),
            NirConst::Char(ch) => format!("{:?}", ch),
            NirConst::String(s) => format!("{:?}", s),
            NirConst::Null => "null".to_string(),
        },
    }
}
