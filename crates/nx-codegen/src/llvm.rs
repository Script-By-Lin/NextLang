use nx_nir::*;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::Command;

#[derive(Default)]
pub struct LlvmCompiler;

impl LlvmCompiler {
    pub fn new() -> Self {
        Self
    }

    /// Detect host target triple using clang or fallback
    pub fn host_target_triple() -> String {
        if let Ok(output) = Command::new("clang").arg("-dumpmachine").output() {
            if output.status.success() {
                let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !s.is_empty() {
                    return s;
                }
            }
        }
        "x86_64-unknown-linux-gnu".to_string()
    }

    /// Lowers a `NirModule` into valid LLVM IR (.ll textual format).
    pub fn emit_llvm_ir(&self, nir: &NirModule) -> String {
        let mut out = String::new();
        let target_triple = Self::host_target_triple();

        out.push_str(&format!("; NextLang Module: {}\n", nir.name));
        out.push_str(&format!("target triple = \"{}\"\n\n", target_triple));

        // 1. Struct type definitions
        if !nir.classes.is_empty() {
            out.push_str("; Struct / Class Types\n");
            for (name, layout) in &nir.classes {
                let field_types: Vec<&'static str> = layout
                    .fields
                    .iter()
                    .map(|(_, ty)| map_llvm_type(ty))
                    .collect();
                out.push_str(&format!(
                    "%struct.{} = type {{ {} }}\n",
                    name,
                    field_types.join(", ")
                ));
            }
            out.push('\n');
        }

        // 2. String literals as global constants
        let mut string_literal_map: HashMap<String, (String, usize)> = HashMap::new();
        let mut additional_strings: Vec<(String, String, usize)> = Vec::new();
        let mut extra_str_counter = 0;

        if !nir.string_literals.is_empty() {
            out.push_str("; String Literals\n");
            for (label, content) in &nir.string_literals {
                let (escaped, len) = escape_llvm_string(content.as_bytes());
                out.push_str(&format!(
                    "@{} = private unnamed_addr constant [{} x i8] c\"{}\", align 1\n",
                    label, len, escaped
                ));
                string_literal_map.insert(label.clone(), (label.clone(), len));
            }
            out.push('\n');
        }

        // Pre-scan for Input prompts or Panics that need additional string constants
        for func in &nir.functions {
            for block in &func.blocks {
                for inst in &block.instructions {
                    if let NirInst::Input {
                        prompt: Some(ref p),
                        ..
                    } = inst
                    {
                        let sym = format!("str_extra_{}", extra_str_counter);
                        extra_str_counter += 1;
                        let (escaped, len) = escape_llvm_string(p.as_bytes());
                        additional_strings.push((sym, escaped, len));
                    }
                }
                if let NirTerminator::Panic(ref msg) = block.terminator {
                    let sym = format!("str_extra_{}", extra_str_counter);
                    extra_str_counter += 1;
                    let (escaped, len) = escape_llvm_string(msg.as_bytes());
                    additional_strings.push((sym, escaped, len));
                }
            }
        }

        if !additional_strings.is_empty() {
            for (sym, escaped, len) in &additional_strings {
                out.push_str(&format!(
                    "@{} = private unnamed_addr constant [{} x i8] c\"{}\", align 1\n",
                    sym, len, escaped
                ));
            }
            out.push('\n');
        }

        // 3. Runtime function declarations
        out.push_str("; Runtime Function Declarations\n");
        out.push_str("declare void @nx_rt_vout_str(ptr)\n");
        out.push_str("declare void @nx_rt_vout_int(i64)\n");
        out.push_str("declare void @nx_rt_vout_float(double)\n");
        out.push_str("declare void @nx_rt_vout_bool(i1)\n");
        out.push_str("declare void @nx_rt_vout_char(i32)\n");
        out.push_str("declare void @nx_rt_vout_space()\n");
        out.push_str("declare void @nx_rt_vout_newline()\n");
        out.push_str("declare ptr @nx_rt_input(ptr)\n");
        out.push_str("declare i64 @nx_rt_to_int(ptr)\n");
        out.push_str("declare double @nx_rt_to_float(ptr)\n");
        out.push_str("declare i64 @nx_rt_len_str(ptr)\n");
        out.push_str("declare ptr @nx_rt_alloc(i64)\n");
        out.push_str("declare void @nx_rt_free(ptr)\n");
        out.push_str("declare void @nx_rt_panic(ptr)\n");
        out.push_str("declare ptr @nx_rt_str_concat(ptr, ptr)\n");
        out.push_str("declare i1 @nx_rt_str_eq(ptr, ptr)\n\n");

        // 4. Systems primitives declarations
        out.push_str("; Phase 8 Systems Primitives\n");
        out.push_str("declare ptr @buffer_new(i64)\n");
        out.push_str("declare void @buffer_free(ptr)\n");
        out.push_str("declare i64 @buffer_size(ptr)\n");
        out.push_str("declare i8 @buffer_get_u8(ptr, i64)\n");
        out.push_str("declare void @buffer_set_u8(ptr, i64, i8)\n");
        out.push_str("declare i32 @buffer_get_u32_le(ptr, i64)\n");
        out.push_str("declare void @buffer_set_u32_le(ptr, i64, i32)\n");
        out.push_str("declare i32 @buffer_get_i32_le(ptr, i64)\n");
        out.push_str("declare void @buffer_set_i32_le(ptr, i64, i32)\n");
        out.push_str("declare i64 @file_open(ptr, ptr)\n");
        out.push_str("declare i1 @file_close(i64)\n");
        out.push_str("declare ptr @file_read_all(ptr)\n");
        out.push_str("declare i1 @file_write(ptr, ptr)\n");
        out.push_str("declare i1 @file_exists(ptr)\n");
        out.push_str("declare i1 @file_remove(ptr)\n");
        out.push_str("declare ptr @env_get(ptr)\n");
        out.push_str("declare i1 @env_set(ptr, ptr)\n");
        out.push_str("declare ptr @get_cwd()\n");
        out.push_str("declare i32 @get_pid()\n");
        out.push_str("declare ptr @ptr_offset(ptr, i64)\n");
        out.push_str("declare i8 @ptr_read_u8(ptr)\n");
        out.push_str("declare void @ptr_write_u8(ptr, i8)\n\n");

        // Phase 9 Standard Library Declarations
        out.push_str("; Phase 9 Standard Library Primitives\n");
        out.push_str("declare double @math_abs(double)\n");
        out.push_str("declare i64 @int_abs(i64)\n");
        out.push_str("declare double @math_sqrt(double)\n");
        out.push_str("declare double @math_pow(double, double)\n");
        out.push_str("declare double @math_floor(double)\n");
        out.push_str("declare double @math_ceil(double)\n");
        out.push_str("declare double @math_min(double, double)\n");
        out.push_str("declare double @math_max(double, double)\n");
        out.push_str("declare double @math_sin(double)\n");
        out.push_str("declare double @math_cos(double)\n");
        out.push_str("declare double @math_random()\n");
        out.push_str("declare i64 @str_len(ptr)\n");
        out.push_str("declare ptr @str_upper(ptr)\n");
        out.push_str("declare ptr @str_lower(ptr)\n");
        out.push_str("declare ptr @str_trim(ptr)\n");
        out.push_str("declare ptr @str_sub(ptr, i64, i64)\n");
        out.push_str("declare i1 @str_contains(ptr, ptr)\n");
        out.push_str("declare i1 @str_starts_with(ptr, ptr)\n");
        out.push_str("declare i1 @str_ends_with(ptr, ptr)\n");
        out.push_str("declare i64 @time_now_ms()\n");
        out.push_str("declare void @time_sleep_ms(i64)\n");
        out.push_str("declare ptr @hex_encode(ptr)\n");
        out.push_str("declare ptr @hex_decode(ptr)\n");
        out.push_str("declare ptr @base64_encode(ptr)\n");
        out.push_str("declare ptr @base64_decode(ptr)\n");
        out.push_str("declare ptr @json_parse(ptr)\n");
        out.push_str("declare ptr @json_stringify(ptr)\n");
        out.push_str("declare void @nx_rt_assert(i1, ptr)\n");
        out.push_str("declare void @assert(i1, ptr)\n");
        out.push_str("declare void @nx_rt_assert_eq(ptr, ptr, ptr)\n");
        out.push_str("declare void @assert_eq(ptr, ptr, ptr)\n\n");

        // 5. External C functions (e.g., abs, puts, etc.)
        let mut declared_externs: HashSet<String> = HashSet::new();
        let internal_funcs: HashSet<String> =
            nir.functions.iter().map(|f| f.name.clone()).collect();
        let known_runtime: HashSet<&str> = [
            "nx_rt_vout_str",
            "nx_rt_vout_int",
            "nx_rt_vout_float",
            "nx_rt_vout_bool",
            "nx_rt_vout_char",
            "nx_rt_vout_space",
            "nx_rt_vout_newline",
            "nx_rt_input",
            "nx_rt_to_int",
            "nx_rt_to_float",
            "nx_rt_len_str",
            "nx_rt_alloc",
            "nx_rt_free",
            "nx_rt_panic",
            "nx_rt_str_concat",
            "nx_rt_str_eq",
            "buffer_new",
            "buffer_free",
            "buffer_size",
            "buffer_get_u8",
            "buffer_set_u8",
            "buffer_get_u32_le",
            "buffer_set_u32_le",
            "buffer_get_i32_le",
            "buffer_set_i32_le",
            "file_open",
            "file_close",
            "file_read_all",
            "file_write",
            "file_exists",
            "file_remove",
            "env_get",
            "env_set",
            "get_cwd",
            "get_pid",
            "ptr_offset",
            "ptr_read_u8",
            "ptr_write_u8",
            "math_abs",
            "int_abs",
            "math_sqrt",
            "math_pow",
            "math_floor",
            "math_ceil",
            "math_min",
            "math_max",
            "math_sin",
            "math_cos",
            "math_random",
            "str_len",
            "str_upper",
            "str_lower",
            "str_trim",
            "str_sub",
            "str_contains",
            "str_starts_with",
            "str_ends_with",
            "time_now_ms",
            "time_sleep_ms",
            "hex_encode",
            "hex_decode",
            "base64_encode",
            "base64_decode",
            "json_parse",
            "json_stringify",
            "nx_rt_assert",
            "assert",
            "nx_rt_assert_eq",
            "assert_eq",
        ]
        .into_iter()
        .collect();

        for func in &nir.functions {
            for block in &func.blocks {
                for inst in &block.instructions {
                    if let NirInst::Call { callee, ret_ty, .. } = inst {
                        if !internal_funcs.contains(callee)
                            && !known_runtime.contains(callee.as_str())
                            && !declared_externs.contains(callee)
                        {
                            declared_externs.insert(callee.clone());
                            let ret_str = map_llvm_type(ret_ty);
                            out.push_str(&format!("declare {} @{}(...)\n", ret_str, callee));
                        }
                    }
                }
            }
        }
        if !declared_externs.is_empty() {
            out.push('\n');
        }

        // 6. Function definitions
        let mut prompt_index = 0;
        for func in &nir.functions {
            self.emit_function(&mut out, func, &mut prompt_index, &additional_strings);
            out.push('\n');
        }

        out
    }

    fn emit_function(
        &self,
        out: &mut String,
        func: &NirFunction,
        prompt_index: &mut usize,
        additional_strings: &[(String, String, usize)],
    ) {
        let ret_ty_str = if func.is_entry && func.name == "main" {
            "i32"
        } else {
            map_llvm_type(&func.ret_ty)
        };

        let param_strs: Vec<String> = func
            .params
            .iter()
            .map(|(pname, pty)| format!("{} %{}", map_llvm_type(pty), pname))
            .collect();

        let linkage = if func.name == "main" { "" } else { "internal " };
        out.push_str(&format!(
            "define {}{} @{}({}) {{\n",
            linkage,
            ret_ty_str,
            func.name,
            param_strs.join(", ")
        ));

        // Track local types for SSA temps and pointers
        let mut temp_types: HashMap<String, NirType> = HashMap::new();
        let mut ptr_types: HashMap<String, NirType> = HashMap::new();

        for (pname, pty) in &func.params {
            temp_types.insert(pname.clone(), pty.clone());
        }

        // Pre-scan block instructions to record alloca and temp types
        for block in &func.blocks {
            for inst in &block.instructions {
                match inst {
                    NirInst::Alloca { dest, ty } => {
                        temp_types.insert(dest.clone(), NirType::Ptr);
                        ptr_types.insert(dest.clone(), ty.clone());
                    }
                    NirInst::Load { dest, ty, .. } => {
                        temp_types.insert(dest.clone(), ty.clone());
                    }
                    NirInst::BinOp { dest, ty, .. } => {
                        temp_types.insert(dest.clone(), ty.clone());
                    }
                    NirInst::Cmp { dest, .. } => {
                        temp_types.insert(dest.clone(), NirType::Bool);
                    }
                    NirInst::UnaryOp { dest, ty, .. } => {
                        temp_types.insert(dest.clone(), ty.clone());
                    }
                    NirInst::GetFieldPtr { dest, ty, .. } => {
                        temp_types.insert(dest.clone(), NirType::Ptr);
                        ptr_types.insert(dest.clone(), ty.clone());
                    }
                    NirInst::Call {
                        dest: Some(d),
                        ret_ty,
                        ..
                    } => {
                        temp_types.insert(d.clone(), ret_ty.clone());
                    }
                    NirInst::Alloc { dest, .. } => {
                        temp_types.insert(dest.clone(), NirType::Ptr);
                    }
                    NirInst::Input { dest, .. } => {
                        temp_types.insert(dest.clone(), NirType::Ptr);
                    }
                    NirInst::ToInt { dest, .. } => {
                        temp_types.insert(dest.clone(), NirType::I64);
                    }
                    NirInst::ToFloat { dest, .. } => {
                        temp_types.insert(dest.clone(), NirType::F64);
                    }
                    NirInst::Len { dest, .. } => {
                        temp_types.insert(dest.clone(), NirType::I64);
                    }
                    _ => {}
                }
            }
        }

        for block in &func.blocks {
            out.push_str(&format!("{}:\n", block.label));

            for inst in &block.instructions {
                self.emit_instruction(
                    out,
                    inst,
                    &mut temp_types,
                    &ptr_types,
                    prompt_index,
                    additional_strings,
                );
            }

            self.emit_terminator(
                out,
                &block.terminator,
                &func.ret_ty,
                func.is_entry && func.name == "main",
                &temp_types,
                prompt_index,
                additional_strings,
            );
        }

        out.push_str("}\n");
    }

    fn emit_instruction(
        &self,
        out: &mut String,
        inst: &NirInst,
        temp_types: &mut HashMap<String, NirType>,
        ptr_types: &HashMap<String, NirType>,
        prompt_index: &mut usize,
        additional_strings: &[(String, String, usize)],
    ) {
        match inst {
            NirInst::Alloca { dest, ty } => {
                out.push_str(&format!("  %{} = alloca {}\n", dest, map_llvm_type(ty)));
            }
            NirInst::Load { dest, src_ptr, ty } => {
                out.push_str(&format!(
                    "  %{} = load {}, ptr %{}\n",
                    dest,
                    map_llvm_type(ty),
                    src_ptr
                ));
            }
            NirInst::Store { dest_ptr, src } => {
                let pointee_ty = ptr_types.get(dest_ptr).cloned().unwrap_or(NirType::I64);
                let (src_val, src_ty) = format_operand_with_type(src, &pointee_ty, temp_types);
                out.push_str(&format!(
                    "  store {} {}, ptr %{}\n",
                    map_llvm_type(&src_ty),
                    src_val,
                    dest_ptr
                ));
            }
            NirInst::GetFieldPtr {
                dest,
                base_ptr,
                byte_offset,
                ..
            } => {
                out.push_str(&format!(
                    "  %{} = getelementptr inbounds i8, ptr %{}, i64 {}\n",
                    dest, base_ptr, byte_offset
                ));
            }
            NirInst::BinOp {
                dest,
                op,
                lhs,
                rhs,
                ty,
            } => {
                let llvm_ty = map_llvm_type(ty);
                let lhs_str = format_operand(lhs);
                let rhs_str = format_operand(rhs);

                let lhs_ty = infer_operand_ty(lhs, temp_types);
                let rhs_ty = infer_operand_ty(rhs, temp_types);

                if *ty == NirType::Ptr && *op == NirBinaryOp::Add {
                    if lhs_ty == NirType::Ptr && rhs_ty == NirType::Ptr {
                        out.push_str(&format!(
                            "  %{} = call ptr @nx_rt_str_concat(ptr {}, ptr {})\n",
                            dest, lhs_str, rhs_str
                        ));
                    } else if lhs_ty == NirType::Ptr {
                        out.push_str(&format!(
                            "  %{} = getelementptr inbounds i8, ptr {}, i64 {}\n",
                            dest, lhs_str, rhs_str
                        ));
                    } else {
                        out.push_str(&format!(
                            "  %{} = getelementptr inbounds i8, ptr {}, i64 {}\n",
                            dest, rhs_str, lhs_str
                        ));
                    }
                } else {
                    let opcode = match op {
                        NirBinaryOp::Add => {
                            if ty.is_float() {
                                "fadd"
                            } else {
                                "add"
                            }
                        }
                        NirBinaryOp::Sub => {
                            if ty.is_float() {
                                "fsub"
                            } else {
                                "sub"
                            }
                        }
                        NirBinaryOp::Mul => {
                            if ty.is_float() {
                                "fmul"
                            } else {
                                "mul"
                            }
                        }
                        NirBinaryOp::Div => {
                            if ty.is_float() {
                                "fdiv"
                            } else if ty.is_unsigned_int() {
                                "udiv"
                            } else {
                                "sdiv"
                            }
                        }
                        NirBinaryOp::Mod => {
                            if ty.is_float() {
                                "frem"
                            } else if ty.is_unsigned_int() {
                                "urem"
                            } else {
                                "srem"
                            }
                        }
                        NirBinaryOp::BitAnd => "and",
                        NirBinaryOp::BitOr => "or",
                        NirBinaryOp::BitXor => "xor",
                        NirBinaryOp::Shl => "shl",
                        NirBinaryOp::Shr => {
                            if ty.is_unsigned_int() {
                                "lshr"
                            } else {
                                "ashr"
                            }
                        }
                    };
                    out.push_str(&format!(
                        "  %{} = {} {} {}, {}\n",
                        dest, opcode, llvm_ty, lhs_str, rhs_str
                    ));
                }
            }
            NirInst::UnaryOp {
                dest,
                op,
                operand,
                ty,
            } => {
                let llvm_ty = map_llvm_type(ty);
                let op_str = format_operand(operand);
                match op {
                    NirUnaryOp::Neg => {
                        if ty.is_float() {
                            out.push_str(&format!("  %{} = fneg {} {}\n", dest, llvm_ty, op_str));
                        } else {
                            out.push_str(&format!("  %{} = sub {} 0, {}\n", dest, llvm_ty, op_str));
                        }
                    }
                    NirUnaryOp::Not => {
                        out.push_str(&format!("  %{} = xor i1 {}, true\n", dest, op_str));
                    }
                    NirUnaryOp::BitNot => {
                        out.push_str(&format!("  %{} = xor {} {}, -1\n", dest, llvm_ty, op_str));
                    }
                }
            }
            NirInst::Cmp { dest, op, lhs, rhs } => {
                let cmp_ty = infer_operand_ty(lhs, temp_types);
                let lhs_str = format_operand(lhs);
                let rhs_str = format_operand(rhs);

                if cmp_ty == NirType::Ptr && (*op == NirCmpOp::Eq || *op == NirCmpOp::Ne) {
                    if *op == NirCmpOp::Eq {
                        out.push_str(&format!(
                            "  %{} = call i1 @nx_rt_str_eq(ptr {}, ptr {})\n",
                            dest, lhs_str, rhs_str
                        ));
                    } else {
                        let eq_tmp = format!("eq_{}", prompt_index);
                        *prompt_index += 1;
                        out.push_str(&format!(
                            "  %{} = call i1 @nx_rt_str_eq(ptr {}, ptr {})\n",
                            eq_tmp, lhs_str, rhs_str
                        ));
                        out.push_str(&format!("  %{} = xor i1 %{}, true\n", dest, eq_tmp));
                    }
                } else if cmp_ty.is_float() {
                    let cond = match op {
                        NirCmpOp::Eq => "oeq",
                        NirCmpOp::Ne => "one",
                        NirCmpOp::Lt => "olt",
                        NirCmpOp::Le => "ole",
                        NirCmpOp::Gt => "ogt",
                        NirCmpOp::Ge => "oge",
                    };
                    out.push_str(&format!(
                        "  %{} = fcmp {} {} {}, {}\n",
                        dest,
                        cond,
                        map_llvm_type(&cmp_ty),
                        lhs_str,
                        rhs_str
                    ));
                } else {
                    let is_uns = cmp_ty.is_unsigned_int();
                    let cond = match op {
                        NirCmpOp::Eq => "eq",
                        NirCmpOp::Ne => "ne",
                        NirCmpOp::Lt => {
                            if is_uns {
                                "ult"
                            } else {
                                "slt"
                            }
                        }
                        NirCmpOp::Le => {
                            if is_uns {
                                "ule"
                            } else {
                                "sle"
                            }
                        }
                        NirCmpOp::Gt => {
                            if is_uns {
                                "ugt"
                            } else {
                                "sgt"
                            }
                        }
                        NirCmpOp::Ge => {
                            if is_uns {
                                "uge"
                            } else {
                                "sge"
                            }
                        }
                    };
                    out.push_str(&format!(
                        "  %{} = icmp {} {} {}, {}\n",
                        dest,
                        cond,
                        map_llvm_type(&cmp_ty),
                        lhs_str,
                        rhs_str
                    ));
                }
            }
            NirInst::Call {
                dest,
                callee,
                args,
                ret_ty,
            } => {
                let arg_strs: Vec<String> = args
                    .iter()
                    .map(|a| {
                        let ty = infer_operand_ty(a, temp_types);
                        format!("{} {}", map_llvm_type(&ty), format_operand(a))
                    })
                    .collect();

                let ret_str = map_llvm_type(ret_ty);
                if *ret_ty == NirType::Void {
                    out.push_str(&format!(
                        "  call void @{}({})\n",
                        callee,
                        arg_strs.join(", ")
                    ));
                } else if let Some(d) = dest {
                    out.push_str(&format!(
                        "  %{} = call {} @{}({})\n",
                        d,
                        ret_str,
                        callee,
                        arg_strs.join(", ")
                    ));
                } else {
                    out.push_str(&format!(
                        "  call {} @{}({})\n",
                        ret_str,
                        callee,
                        arg_strs.join(", ")
                    ));
                }
            }
            NirInst::Vout { args } => {
                for (i, (arg, ty)) in args.iter().enumerate() {
                    if i > 0 {
                        out.push_str("  call void @nx_rt_vout_space()\n");
                    }
                    let arg_str = format_operand(arg);
                    match ty {
                        NirType::Ptr => {
                            out.push_str(&format!(
                                "  call void @nx_rt_vout_str(ptr {})\n",
                                arg_str
                            ));
                        }
                        NirType::Bool => {
                            out.push_str(&format!(
                                "  call void @nx_rt_vout_bool(i1 {})\n",
                                arg_str
                            ));
                        }
                        NirType::Char => {
                            out.push_str(&format!(
                                "  call void @nx_rt_vout_char(i32 {})\n",
                                arg_str
                            ));
                        }
                        NirType::F32 | NirType::F64 => {
                            if *ty == NirType::F32 {
                                let ext_tmp = format!("ext_{}", prompt_index);
                                *prompt_index += 1;
                                out.push_str(&format!(
                                    "  %{} = fpext float {} to double\n",
                                    ext_tmp, arg_str
                                ));
                                out.push_str(&format!(
                                    "  call void @nx_rt_vout_float(double %{})\n",
                                    ext_tmp
                                ));
                            } else {
                                out.push_str(&format!(
                                    "  call void @nx_rt_vout_float(double {})\n",
                                    arg_str
                                ));
                            }
                        }
                        _ => {
                            if *ty != NirType::I64 && *ty != NirType::U64 {
                                let ext_tmp = format!("ext_{}", prompt_index);
                                *prompt_index += 1;
                                let cast = if ty.is_unsigned_int() { "zext" } else { "sext" };
                                out.push_str(&format!(
                                    "  %{} = {} {} {} to i64\n",
                                    ext_tmp,
                                    cast,
                                    map_llvm_type(ty),
                                    arg_str
                                ));
                                out.push_str(&format!(
                                    "  call void @nx_rt_vout_int(i64 %{})\n",
                                    ext_tmp
                                ));
                            } else {
                                out.push_str(&format!(
                                    "  call void @nx_rt_vout_int(i64 {})\n",
                                    arg_str
                                ));
                            }
                        }
                    }
                }
                out.push_str("  call void @nx_rt_vout_newline()\n");
            }
            NirInst::Alloc { dest, size_bytes } => {
                let sz_str = format_operand(size_bytes);
                out.push_str(&format!(
                    "  %{} = call ptr @nx_rt_alloc(i64 {})\n",
                    dest, sz_str
                ));
            }
            NirInst::Free { ptr } => {
                let ptr_str = format_operand(ptr);
                out.push_str(&format!("  call void @nx_rt_free(ptr {})\n", ptr_str));
            }
            NirInst::Input { dest, prompt } => {
                if prompt.is_some() {
                    let sym = &additional_strings[*prompt_index].0;
                    *prompt_index += 1;
                    out.push_str(&format!(
                        "  %{} = call ptr @nx_rt_input(ptr @{})\n",
                        dest, sym
                    ));
                } else {
                    out.push_str(&format!("  %{} = call ptr @nx_rt_input(ptr null)\n", dest));
                }
            }
            NirInst::ToInt { dest, src, .. } => {
                let src_str = format_operand(src);
                out.push_str(&format!(
                    "  %{} = call i64 @nx_rt_to_int(ptr {})\n",
                    dest, src_str
                ));
            }
            NirInst::ToFloat { dest, src, .. } => {
                let src_str = format_operand(src);
                out.push_str(&format!(
                    "  %{} = call double @nx_rt_to_float(ptr {})\n",
                    dest, src_str
                ));
            }
            NirInst::Len { dest, src, .. } => {
                let src_str = format_operand(src);
                out.push_str(&format!(
                    "  %{} = call i64 @nx_rt_len_str(ptr {})\n",
                    dest, src_str
                ));
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_terminator(
        &self,
        out: &mut String,
        term: &NirTerminator,
        func_ret_ty: &NirType,
        is_main: bool,
        temp_types: &HashMap<String, NirType>,
        prompt_index: &mut usize,
        additional_strings: &[(String, String, usize)],
    ) {
        match term {
            NirTerminator::Return(None) => {
                if is_main {
                    out.push_str("  ret i32 0\n");
                } else if *func_ret_ty == NirType::Void {
                    out.push_str("  ret void\n");
                } else if func_ret_ty.is_float() {
                    out.push_str(&format!("  ret {} 0.0\n", map_llvm_type(func_ret_ty)));
                } else if *func_ret_ty == NirType::Ptr
                    || matches!(func_ret_ty, NirType::Struct { .. })
                {
                    out.push_str("  ret ptr null\n");
                } else if *func_ret_ty == NirType::Bool {
                    out.push_str("  ret i1 false\n");
                } else {
                    out.push_str(&format!("  ret {} 0\n", map_llvm_type(func_ret_ty)));
                }
            }
            NirTerminator::Return(Some(val)) => {
                let ret_ty = if is_main { &NirType::I32 } else { func_ret_ty };
                let (val_str, _) = format_operand_with_type(val, ret_ty, temp_types);
                out.push_str(&format!("  ret {} {}\n", map_llvm_type(ret_ty), val_str));
            }
            NirTerminator::Branch(target) => {
                out.push_str(&format!("  br label %{}\n", target));
            }
            NirTerminator::CondBranch {
                cond,
                then_block,
                else_block,
            } => {
                out.push_str(&format!(
                    "  br i1 {}, label %{}, label %{}\n",
                    format_operand(cond),
                    then_block,
                    else_block
                ));
            }
            NirTerminator::Panic(_) => {
                let sym = &additional_strings[*prompt_index].0;
                *prompt_index += 1;
                out.push_str(&format!("  call void @nx_rt_panic(ptr @{})\n", sym));
                out.push_str("  unreachable\n");
            }
        }
    }

    /// Verifies the generated LLVM IR using `opt -passes=verify`.
    pub fn verify_ir(&self, ll_path: &Path) -> Result<(), String> {
        let opt_bin = if Command::new("opt").arg("--version").output().is_ok() {
            "opt"
        } else {
            return Ok(()); // Skip if opt is missing
        };

        let output = Command::new(opt_bin)
            .arg("-passes=verify")
            .arg("-disable-output")
            .arg(ll_path)
            .output()
            .map_err(|e| format!("failed to invoke {}: {}", opt_bin, e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("LLVM IR verification failed:\n{}", stderr.trim()));
        }

        Ok(())
    }

    /// Compiles an LLVM IR file (.ll) and links with the native runtime object into an executable.
    pub fn compile_and_link(
        &self,
        ll_path: &Path,
        runtime_obj: &Path,
        output_exe: &Path,
        opt_level: OptLevel,
    ) -> Result<(), String> {
        // Ensure output parent directory exists
        if let Some(parent) = output_exe.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("failed to create output directory: {}", e))?;
            }
        }

        let opt_flag = match opt_level {
            OptLevel::O0 => "-O0",
            OptLevel::O1 => "-O1",
            OptLevel::O2 => "-O2",
            OptLevel::O3 => "-O3",
        };

        let clang_bin = if Command::new("clang").arg("--version").output().is_ok() {
            "clang"
        } else {
            return Err("clang not found in PATH".to_string());
        };

        let output = Command::new(clang_bin)
            .arg(opt_flag)
            .arg("-no-pie")
            .arg(ll_path)
            .arg(runtime_obj)
            .arg("-lm")
            .arg("-o")
            .arg(output_exe)
            .output()
            .map_err(|e| format!("failed to invoke {}: {}", clang_bin, e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "clang LLVM compilation failed with status {:?}:\n{}",
                output.status.code(),
                stderr.trim()
            ));
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(output_exe) {
                let mut perms = meta.permissions();
                perms.set_mode(0o755);
                let _ = std::fs::set_permissions(output_exe, perms);
            }
        }

        Ok(())
    }
}

pub fn map_llvm_type(ty: &NirType) -> &'static str {
    match ty {
        NirType::I8 | NirType::U8 => "i8",
        NirType::I16 | NirType::U16 => "i16",
        NirType::I32 | NirType::U32 => "i32",
        NirType::I64 | NirType::U64 => "i64",
        NirType::I128 | NirType::U128 => "i128",
        NirType::F32 => "float",
        NirType::F64 => "double",
        NirType::Bool => "i1",
        NirType::Char => "i32",
        NirType::Ptr | NirType::Struct { .. } => "ptr",
        NirType::Void => "void",
    }
}

fn format_operand(op: &NirOperand) -> String {
    match op {
        NirOperand::Var(name) => format!("%{}", name),
        NirOperand::Const(NirConst::Int(v, _)) => format!("{}", v),
        NirOperand::Const(NirConst::Float(f, _)) => {
            if f.fract() == 0.0 {
                format!("{:.1}", f)
            } else {
                format!("{}", f)
            }
        }
        NirOperand::Const(NirConst::Bool(b)) => {
            if *b {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        NirOperand::Const(NirConst::Char(c)) => format!("{}", *c as u32),
        NirOperand::Const(NirConst::String(label)) => format!("@{}", label),
        NirOperand::Const(NirConst::Null) => "null".to_string(),
    }
}

fn format_operand_with_type(
    op: &NirOperand,
    expected_ty: &NirType,
    temp_types: &HashMap<String, NirType>,
) -> (String, NirType) {
    match op {
        NirOperand::Var(name) => {
            let ty = temp_types
                .get(name)
                .cloned()
                .unwrap_or_else(|| expected_ty.clone());
            (format!("%{}", name), ty)
        }
        NirOperand::Const(c) => match c {
            NirConst::Int(v, _) => (format!("{}", v), expected_ty.clone()),
            NirConst::Float(f, _) => {
                let s = if f.fract() == 0.0 {
                    format!("{:.1}", f)
                } else {
                    format!("{}", f)
                };
                (s, expected_ty.clone())
            }
            NirConst::Bool(b) => {
                let s = if *b { "true" } else { "false" };
                (s.to_string(), NirType::Bool)
            }
            NirConst::Char(ch) => (format!("{}", *ch as u32), NirType::Char),
            NirConst::String(label) => (format!("@{}", label), NirType::Ptr),
            NirConst::Null => ("null".to_string(), NirType::Ptr),
        },
    }
}

fn infer_operand_ty(op: &NirOperand, temp_types: &HashMap<String, NirType>) -> NirType {
    match op {
        NirOperand::Var(name) => temp_types.get(name).cloned().unwrap_or(NirType::I64),
        NirOperand::Const(c) => c.ty(),
    }
}

fn escape_llvm_string(bytes: &[u8]) -> (String, usize) {
    let mut out = String::new();
    let mut len = 0;
    for &b in bytes {
        len += 1;
        match b {
            b'\\' => out.push_str("\\5C"),
            b'"' => out.push_str("\\22"),
            b'\n' => out.push_str("\\0A"),
            b'\r' => out.push_str("\\0D"),
            b'\t' => out.push_str("\\09"),
            32..=126 => out.push(b as char),
            _ => out.push_str(&format!("\\{:02X}", b)),
        }
    }
    out.push_str("\\00");
    len += 1;
    (out, len)
}
