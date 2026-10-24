use crate::config::CodegenConfig;
use crate::runtime::RuntimeSymbols;
use cranelift_codegen::ir::{
    types, AbiParam, InstBuilder, MemFlagsData, Signature, StackSlotData, StackSlotKind, Value,
};
use cranelift_codegen::isa;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{default_libcall_names, DataDescription, FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use nx_nir::*;
use std::collections::HashMap;
use target_lexicon::Triple;

pub struct NativeCompiler {
    module: ObjectModule,
    runtime: RuntimeSymbols,
    func_ids: HashMap<String, FuncId>,
    string_data: HashMap<String, cranelift_module::DataId>,
}

impl NativeCompiler {
    pub fn new(module_name: &str, config: &CodegenConfig) -> Result<Self, String> {
        let mut flag_builder = settings::builder();
        let opt_str = match config.opt_level {
            OptLevel::O0 => "none",
            OptLevel::O1 | OptLevel::O2 | OptLevel::O3 => "speed",
        };
        flag_builder
            .set("opt_level", opt_str)
            .map_err(|e| format!("failed to set opt_level: {}", e))?;

        let flags = settings::Flags::new(flag_builder);
        let host_triple = Triple::host();
        let isa = isa::lookup(host_triple)
            .map_err(|e| format!("unsupported target ISA: {}", e))?
            .finish(flags)
            .map_err(|e| format!("failed to finish ISA: {}", e))?;

        let call_conv = isa.default_call_conv();
        let builder = ObjectBuilder::new(isa, module_name, default_libcall_names())
            .map_err(|e| format!("failed to create object builder: {}", e))?;

        let mut module = ObjectModule::new(builder);
        let runtime = RuntimeSymbols::declare(&mut module, call_conv)?;

        Ok(Self {
            module,
            runtime,
            func_ids: HashMap::new(),
            string_data: HashMap::new(),
        })
    }

    pub fn compile_module(mut self, nir: &NirModule) -> Result<Vec<u8>, String> {
        let call_conv = self.module.isa().default_call_conv();

        // Pass 1: Emit string literals as null-terminated static data
        for (label, content) in &nir.string_literals {
            let data_id = self
                .module
                .declare_data(label, Linkage::Local, false, false)
                .map_err(|e| e.to_string())?;

            let mut desc = DataDescription::new();
            let mut bytes = content.as_bytes().to_vec();
            bytes.push(0); // Null-terminator for C strings
            desc.define(bytes.into_boxed_slice());

            self.module
                .define_data(data_id, &desc)
                .map_err(|e| e.to_string())?;

            self.string_data.insert(label.clone(), data_id);
        }

        // Pass 2: Declare all functions
        for func in &nir.functions {
            let mut sig = Signature::new(call_conv);
            for (_, pty) in &func.params {
                sig.params.push(AbiParam::new(map_cl_type(pty)));
            }
            if func.ret_ty != NirType::Void {
                sig.returns.push(AbiParam::new(map_cl_type(&func.ret_ty)));
            }

            let func_id = self
                .module
                .declare_function(&func.name, Linkage::Export, &sig)
                .map_err(|e| e.to_string())?;

            self.func_ids.insert(func.name.clone(), func_id);
        }

        // Pass 3: Define each function body
        let mut fn_ctx = FunctionBuilderContext::new();
        for func in &nir.functions {
            self.compile_function(func, &mut fn_ctx)?;
        }

        // Finish object module and emit bytes
        let product = self.module.finish();
        let object_bytes = product
            .emit()
            .map_err(|e| format!("object emission failed: {}", e))?;

        Ok(object_bytes)
    }

    fn compile_function(
        &mut self,
        func: &NirFunction,
        fn_ctx: &mut FunctionBuilderContext,
    ) -> Result<(), String> {
        let func_id = *self.func_ids.get(&func.name).unwrap();
        let mut cl_func = cranelift_codegen::ir::Function::new();
        cl_func.signature = self
            .module
            .declarations()
            .get_function_decl(func_id)
            .signature
            .clone();

        let mut builder = FunctionBuilder::new(&mut cl_func, fn_ctx);

        let mut block_map = HashMap::new();
        for block in &func.blocks {
            let cl_block = builder.create_block();
            block_map.insert(block.label.clone(), cl_block);
        }

        if func.blocks.is_empty() {
            builder.finalize(self.module.target_config());
            return Ok(());
        }

        let entry_block = *block_map.get(&func.blocks[0].label).unwrap();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);

        let mut val_map: HashMap<String, Value> = HashMap::new();
        for (i, (pname, _)) in func.params.iter().enumerate() {
            let p_val = builder.block_params(entry_block)[i];
            val_map.insert(pname.clone(), p_val);
        }

        let mut stack_slots: HashMap<String, (cranelift_codegen::ir::StackSlot, types::Type)> =
            HashMap::new();

        for block in &func.blocks {
            let cl_block = *block_map.get(&block.label).unwrap();
            builder.switch_to_block(cl_block);

            for inst in &block.instructions {
                self.compile_instruction(inst, &mut builder, &mut val_map, &mut stack_slots)?;
            }

            self.compile_terminator(&block.terminator, &mut builder, &block_map, &val_map)?;
        }

        builder.seal_all_blocks();
        builder.finalize(self.module.target_config());

        let mut ctx = self.module.make_context();
        ctx.func = cl_func;
        if let Err(e) = self.module.define_function(func_id, &mut ctx) {
            eprintln!("Cranelift IR for {}:\n{}", func.name, ctx.func.display());
            return Err(format!("failed to define function {}: {:?}", func.name, e));
        }

        Ok(())
    }

    fn compile_instruction(
        &mut self,
        inst: &NirInst,
        builder: &mut FunctionBuilder,
        val_map: &mut HashMap<String, Value>,
        stack_slots: &mut HashMap<String, (cranelift_codegen::ir::StackSlot, types::Type)>,
    ) -> Result<(), String> {
        match inst {
            NirInst::Alloca { dest, ty } => {
                let cl_ty = map_cl_type(ty);
                let size = cl_ty.bytes().max(8);
                let slot = builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    size,
                    0,
                ));
                stack_slots.insert(dest.clone(), (slot, cl_ty));
            }
            NirInst::Load { dest, src_ptr, ty } => {
                let cl_ty = map_cl_type(ty);
                let val = if let Some((slot, slot_ty)) = stack_slots.get(src_ptr) {
                    builder.ins().stack_load(types::I64, *slot_ty, *slot, 0)
                } else if let Some(ptr_val) = val_map.get(src_ptr) {
                    builder.ins().load(cl_ty, MemFlagsData::new(), *ptr_val, 0)
                } else {
                    builder.ins().iconst(types::I64, 0)
                };
                val_map.insert(dest.clone(), val);
            }
            NirInst::Store { dest_ptr, src } => {
                let mut val = self.resolve_operand(src, builder, val_map)?;
                if let Some((slot, target_ty)) = stack_slots.get(dest_ptr) {
                    let cur_ty = val_type(val, builder);
                    if cur_ty != *target_ty && cur_ty.is_int() && target_ty.is_int() {
                        val = if cur_ty.bytes() > target_ty.bytes() {
                            builder.ins().ireduce(*target_ty, val)
                        } else {
                            builder.ins().sextend(*target_ty, val)
                        };
                    }
                    builder.ins().stack_store(types::I64, val, *slot, 0);
                } else if let Some(ptr_val) = val_map.get(dest_ptr) {
                    builder.ins().store(MemFlagsData::new(), val, *ptr_val, 0);
                }
            }
            NirInst::GetFieldPtr {
                dest,
                base_ptr,
                byte_offset,
                ..
            } => {
                let base_val = if let Some((slot, _)) = stack_slots.get(base_ptr) {
                    builder.ins().stack_addr(types::I64, *slot, 0)
                } else if let Some(v) = val_map.get(base_ptr) {
                    *v
                } else {
                    builder.ins().iconst(types::I64, 0)
                };
                let off_val = builder.ins().iconst(types::I64, *byte_offset as i64);
                let addr = builder.ins().iadd(base_val, off_val);
                val_map.insert(dest.clone(), addr);
            }
            NirInst::BinOp {
                dest,
                op,
                lhs,
                rhs,
                ty,
            } => {
                let mut l_val = self.resolve_operand(lhs, builder, val_map)?;
                let mut r_val = self.resolve_operand(rhs, builder, val_map)?;

                let res = if *ty == NirType::Ptr && *op == NirBinaryOp::Add {
                    let concat_ref = self
                        .module
                        .declare_func_in_func(self.runtime.str_concat, builder.func);
                    let call = builder.ins().call(concat_ref, &[l_val, r_val]);
                    builder.inst_results(call)[0]
                } else if ty.is_float() {
                    let l_ty = val_type(l_val, builder);
                    let r_ty = val_type(r_val, builder);
                    if l_ty != r_ty {
                        if l_ty == types::F32 {
                            l_val = builder.ins().fpromote(types::F64, l_val);
                        } else {
                            r_val = builder.ins().fpromote(types::F64, r_val);
                        }
                    }
                    match op {
                        NirBinaryOp::Add => builder.ins().fadd(l_val, r_val),
                        NirBinaryOp::Sub => builder.ins().fsub(l_val, r_val),
                        NirBinaryOp::Mul => builder.ins().fmul(l_val, r_val),
                        NirBinaryOp::Div => builder.ins().fdiv(l_val, r_val),
                        _ => l_val,
                    }
                } else {
                    let l_ty = val_type(l_val, builder);
                    let r_ty = val_type(r_val, builder);
                    if l_ty != r_ty && l_ty.is_int() && r_ty.is_int() {
                        if l_ty.bytes() < r_ty.bytes() {
                            l_val = builder.ins().sextend(r_ty, l_val);
                        } else {
                            r_val = builder.ins().sextend(l_ty, r_val);
                        }
                    }
                    match op {
                        NirBinaryOp::Add => builder.ins().iadd(l_val, r_val),
                        NirBinaryOp::Sub => builder.ins().isub(l_val, r_val),
                        NirBinaryOp::Mul => builder.ins().imul(l_val, r_val),
                        NirBinaryOp::Div => builder.ins().sdiv(l_val, r_val),
                        NirBinaryOp::Mod => builder.ins().srem(l_val, r_val),
                        NirBinaryOp::BitAnd => builder.ins().band(l_val, r_val),
                        NirBinaryOp::BitOr => builder.ins().bor(l_val, r_val),
                        NirBinaryOp::BitXor => builder.ins().bxor(l_val, r_val),
                        NirBinaryOp::Shl => builder.ins().ishl(l_val, r_val),
                        NirBinaryOp::Shr => builder.ins().sshr(l_val, r_val),
                    }
                };
                val_map.insert(dest.clone(), res);
            }
            NirInst::Cmp { dest, op, lhs, rhs } => {
                let mut l_val = self.resolve_operand(lhs, builder, val_map)?;
                let mut r_val = self.resolve_operand(rhs, builder, val_map)?;

                let l_ty = val_type(l_val, builder);
                let r_ty = val_type(r_val, builder);
                if l_ty != r_ty && l_ty.is_int() && r_ty.is_int() {
                    if l_ty.bytes() < r_ty.bytes() {
                        l_val = builder.ins().sextend(r_ty, l_val);
                    } else {
                        r_val = builder.ins().sextend(l_ty, r_val);
                    }
                }

                let cond = match op {
                    NirCmpOp::Eq => cranelift_codegen::ir::condcodes::IntCC::Equal,
                    NirCmpOp::Ne => cranelift_codegen::ir::condcodes::IntCC::NotEqual,
                    NirCmpOp::Lt => cranelift_codegen::ir::condcodes::IntCC::SignedLessThan,
                    NirCmpOp::Le => cranelift_codegen::ir::condcodes::IntCC::SignedLessThanOrEqual,
                    NirCmpOp::Gt => cranelift_codegen::ir::condcodes::IntCC::SignedGreaterThan,
                    NirCmpOp::Ge => {
                        cranelift_codegen::ir::condcodes::IntCC::SignedGreaterThanOrEqual
                    }
                };
                let cmp_res = builder.ins().icmp(cond, l_val, r_val);
                val_map.insert(dest.clone(), cmp_res);
            }
            NirInst::UnaryOp {
                dest,
                op,
                operand,
                ty,
            } => {
                let val = self.resolve_operand(operand, builder, val_map)?;
                let res = match op {
                    NirUnaryOp::Neg => {
                        if ty.is_float() {
                            builder.ins().fneg(val)
                        } else {
                            builder.ins().ineg(val)
                        }
                    }
                    NirUnaryOp::Not => {
                        let v_ty = val_type(val, builder);
                        let zero = builder.ins().iconst(v_ty, 0);
                        builder.ins().icmp(
                            cranelift_codegen::ir::condcodes::IntCC::Equal,
                            val,
                            zero,
                        )
                    }
                    NirUnaryOp::BitNot => builder.ins().bnot(val),
                };
                val_map.insert(dest.clone(), res);
            }
            NirInst::Call {
                dest, callee, args, ..
            } => {
                let func_id = *self
                    .func_ids
                    .get(callee)
                    .ok_or_else(|| format!("unknown function: {}", callee))?;
                let func_ref = self.module.declare_func_in_func(func_id, builder.func);
                let sig = self
                    .module
                    .declarations()
                    .get_function_decl(func_id)
                    .signature
                    .clone();

                let mut arg_vals = Vec::new();
                for (i, a) in args.iter().enumerate() {
                    let mut v = self.resolve_operand(a, builder, val_map)?;
                    if let Some(param_abi) = sig.params.get(i) {
                        let cur_ty = val_type(v, builder);
                        let target_ty = param_abi.value_type;
                        if cur_ty != target_ty && cur_ty.is_int() && target_ty.is_int() {
                            v = if cur_ty.bytes() > target_ty.bytes() {
                                builder.ins().ireduce(target_ty, v)
                            } else {
                                builder.ins().sextend(target_ty, v)
                            };
                        }
                    }
                    arg_vals.push(v);
                }

                let call_inst = builder.ins().call(func_ref, &arg_vals);
                if let Some(d) = dest {
                    let results = builder.inst_results(call_inst);
                    if let Some(res) = results.first() {
                        val_map.insert(d.clone(), *res);
                    }
                }
            }
            NirInst::Vout { args } => {
                for (i, (arg_op, arg_ty)) in args.iter().enumerate() {
                    if i > 0 {
                        let space_ref = self
                            .module
                            .declare_func_in_func(self.runtime.vout_space, builder.func);
                        builder.ins().call(space_ref, &[]);
                    }

                    let val = self.resolve_operand(arg_op, builder, val_map)?;

                    match arg_ty {
                        NirType::Ptr => {
                            let str_ref = self
                                .module
                                .declare_func_in_func(self.runtime.vout_str, builder.func);
                            builder.ins().call(str_ref, &[val]);
                        }
                        NirType::F32 | NirType::F64 => {
                            let f_val = if arg_ty == &NirType::F32 {
                                builder.ins().fpromote(types::F64, val)
                            } else {
                                val
                            };
                            let f_ref = self
                                .module
                                .declare_func_in_func(self.runtime.vout_float, builder.func);
                            builder.ins().call(f_ref, &[f_val]);
                        }
                        NirType::Bool => {
                            let b_ref = self
                                .module
                                .declare_func_in_func(self.runtime.vout_bool, builder.func);
                            builder.ins().call(b_ref, &[val]);
                        }
                        NirType::Char => {
                            let c_ref = self
                                .module
                                .declare_func_in_func(self.runtime.vout_char, builder.func);
                            builder.ins().call(c_ref, &[val]);
                        }
                        _ => {
                            let cur_ty = val_type(val, builder);
                            let int_val = if cur_ty != types::I64 {
                                builder.ins().sextend(types::I64, val)
                            } else {
                                val
                            };
                            let int_ref = self
                                .module
                                .declare_func_in_func(self.runtime.vout_int, builder.func);
                            builder.ins().call(int_ref, &[int_val]);
                        }
                    }
                }
                let nl_ref = self
                    .module
                    .declare_func_in_func(self.runtime.vout_newline, builder.func);
                builder.ins().call(nl_ref, &[]);
            }
            NirInst::Input { dest, prompt } => {
                let prompt_val = if let Some(p) = prompt {
                    let label = format!("__prompt_{}", dest);
                    let data_id = self
                        .module
                        .declare_data(&label, Linkage::Local, false, false)
                        .map_err(|e| e.to_string())?;
                    let mut desc = DataDescription::new();
                    let mut bytes = p.as_bytes().to_vec();
                    bytes.push(0);
                    desc.define(bytes.into_boxed_slice());
                    self.module
                        .define_data(data_id, &desc)
                        .map_err(|e| e.to_string())?;

                    let local_data = self.module.declare_data_in_func(data_id, builder.func);
                    builder.ins().symbol_value(types::I64, local_data)
                } else {
                    builder.ins().iconst(types::I64, 0)
                };

                let in_ref = self
                    .module
                    .declare_func_in_func(self.runtime.input, builder.func);
                let call_inst = builder.ins().call(in_ref, &[prompt_val]);
                let res = builder.inst_results(call_inst)[0];
                val_map.insert(dest.clone(), res);
            }
            NirInst::ToInt { dest, src, .. } => {
                let s_val = self.resolve_operand(src, builder, val_map)?;
                let to_int_ref = self
                    .module
                    .declare_func_in_func(self.runtime.to_int, builder.func);
                let call_inst = builder.ins().call(to_int_ref, &[s_val]);
                let res = builder.inst_results(call_inst)[0];
                val_map.insert(dest.clone(), res);
            }
            NirInst::ToFloat { dest, src, .. } => {
                let s_val = self.resolve_operand(src, builder, val_map)?;
                let to_fl_ref = self
                    .module
                    .declare_func_in_func(self.runtime.to_float, builder.func);
                let call_inst = builder.ins().call(to_fl_ref, &[s_val]);
                let res = builder.inst_results(call_inst)[0];
                val_map.insert(dest.clone(), res);
            }
            NirInst::Len { dest, src, .. } => {
                let s_val = self.resolve_operand(src, builder, val_map)?;
                let len_ref = self
                    .module
                    .declare_func_in_func(self.runtime.len_str, builder.func);
                let call_inst = builder.ins().call(len_ref, &[s_val]);
                let res = builder.inst_results(call_inst)[0];
                val_map.insert(dest.clone(), res);
            }
            NirInst::Alloc { dest, size_bytes } => {
                let s_val = self.resolve_operand(size_bytes, builder, val_map)?;
                let cur_ty = val_type(s_val, builder);
                let s_val = if cur_ty != types::I64 {
                    builder.ins().sextend(types::I64, s_val)
                } else {
                    s_val
                };
                let alloc_ref = self
                    .module
                    .declare_func_in_func(self.runtime.alloc, builder.func);
                let call_inst = builder.ins().call(alloc_ref, &[s_val]);
                let res = builder.inst_results(call_inst)[0];
                val_map.insert(dest.clone(), res);
            }
            NirInst::Free { ptr } => {
                let p_val = self.resolve_operand(ptr, builder, val_map)?;
                let free_ref = self
                    .module
                    .declare_func_in_func(self.runtime.free, builder.func);
                builder.ins().call(free_ref, &[p_val]);
            }
        }
        Ok(())
    }

    fn compile_terminator(
        &mut self,
        term: &NirTerminator,
        builder: &mut FunctionBuilder,
        block_map: &HashMap<String, cranelift_codegen::ir::Block>,
        val_map: &HashMap<String, Value>,
    ) -> Result<(), String> {
        match term {
            NirTerminator::Return(Some(op)) => {
                let mut val = self.resolve_operand(op, builder, val_map)?;
                if let Some(ret_abi) = builder.func.signature.returns.first() {
                    let cur_ty = val_type(val, builder);
                    let target_ty = ret_abi.value_type;
                    if cur_ty != target_ty && cur_ty.is_int() && target_ty.is_int() {
                        val = if cur_ty.bytes() > target_ty.bytes() {
                            builder.ins().ireduce(target_ty, val)
                        } else {
                            builder.ins().sextend(target_ty, val)
                        };
                    }
                }
                builder.ins().return_(&[val]);
            }
            NirTerminator::Return(None) => {
                let ret_ty = builder.func.signature.returns.first().map(|r| r.value_type);
                if let Some(target_ty) = ret_ty {
                    let zero = if target_ty.is_float() {
                        builder.ins().f64const(0.0)
                    } else {
                        builder.ins().iconst(target_ty, 0)
                    };
                    builder.ins().return_(&[zero]);
                } else {
                    builder.ins().return_(&[]);
                }
            }
            NirTerminator::Branch(target) => {
                let b = *block_map.get(target).unwrap();
                builder.ins().jump(b, &[]);
            }
            NirTerminator::CondBranch {
                cond,
                then_block,
                else_block,
            } => {
                let cond_val = self.resolve_operand(cond, builder, val_map)?;
                let then_b = *block_map.get(then_block).unwrap();
                let else_b = *block_map.get(else_block).unwrap();
                builder.ins().brif(cond_val, then_b, &[], else_b, &[]);
            }
            NirTerminator::Panic(msg) => {
                let label = format!("__panic_{}", msg.len());
                let data_id = self
                    .module
                    .declare_data(&label, Linkage::Local, false, false)
                    .map_err(|e| e.to_string())?;
                let mut desc = DataDescription::new();
                let mut bytes = msg.as_bytes().to_vec();
                bytes.push(0);
                desc.define(bytes.into_boxed_slice());
                self.module
                    .define_data(data_id, &desc)
                    .map_err(|e| e.to_string())?;

                let local_data = self.module.declare_data_in_func(data_id, builder.func);
                let p_val = builder.ins().symbol_value(types::I64, local_data);

                let panic_ref = self
                    .module
                    .declare_func_in_func(self.runtime.panic, builder.func);
                builder.ins().call(panic_ref, &[p_val]);
                builder
                    .ins()
                    .trap(cranelift_codegen::ir::TrapCode::user(1).unwrap());
            }
        }
        Ok(())
    }

    fn resolve_operand(
        &mut self,
        op: &NirOperand,
        builder: &mut FunctionBuilder,
        val_map: &HashMap<String, Value>,
    ) -> Result<Value, String> {
        match op {
            NirOperand::Var(name) => val_map
                .get(name)
                .copied()
                .ok_or_else(|| format!("unresolved variable in codegen: {}", name)),
            NirOperand::Const(c) => match c {
                NirConst::Int(n, ty) => Ok(builder.ins().iconst(map_cl_type(ty), *n)),
                NirConst::Float(f, ty) => {
                    if ty == &NirType::F32 {
                        Ok(builder.ins().f32const(*f as f32))
                    } else {
                        Ok(builder.ins().f64const(*f))
                    }
                }
                NirConst::Bool(b) => {
                    let v = if *b { 1 } else { 0 };
                    Ok(builder.ins().iconst(types::I8, v))
                }
                NirConst::Char(ch) => Ok(builder.ins().iconst(types::I32, *ch as u32 as i64)),
                NirConst::String(label) => {
                    let data_id = self
                        .string_data
                        .get(label)
                        .ok_or_else(|| format!("unknown string literal: {}", label))?;
                    let local_data = self.module.declare_data_in_func(*data_id, builder.func);
                    Ok(builder.ins().symbol_value(types::I64, local_data))
                }
                NirConst::Null => Ok(builder.ins().iconst(types::I64, 0)),
            },
        }
    }
}

fn map_cl_type(ty: &NirType) -> types::Type {
    match ty {
        NirType::I8 | NirType::U8 | NirType::Bool => types::I8,
        NirType::I16 | NirType::U16 => types::I16,
        NirType::I32 | NirType::U32 | NirType::Char => types::I32,
        NirType::I64 | NirType::U64 | NirType::Ptr => types::I64,
        NirType::I128 | NirType::U128 => types::I128,
        NirType::F32 => types::F32,
        NirType::F64 => types::F64,
        NirType::Void => types::I64,
        NirType::Struct { .. } => types::I64,
    }
}

fn val_type(val: Value, builder: &FunctionBuilder) -> types::Type {
    builder.func.dfg.value_type(val)
}
