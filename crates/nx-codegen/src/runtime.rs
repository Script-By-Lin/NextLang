use cranelift_codegen::ir::{types, AbiParam, Signature};
use cranelift_codegen::isa::CallConv;
use cranelift_module::{FuncId, Linkage, Module};
use cranelift_object::ObjectModule;

#[derive(Clone, Copy)]
pub struct RuntimeSymbols {
    pub vout_str: FuncId,
    pub vout_int: FuncId,
    pub vout_float: FuncId,
    pub vout_bool: FuncId,
    pub vout_char: FuncId,
    pub vout_space: FuncId,
    pub vout_newline: FuncId,
    pub input: FuncId,
    pub to_int: FuncId,
    pub to_float: FuncId,
    pub len_str: FuncId,
    pub alloc: FuncId,
    pub free: FuncId,
    pub panic: FuncId,
    pub str_concat: FuncId,
    pub str_eq: FuncId,
}

impl RuntimeSymbols {
    pub fn declare(module: &mut ObjectModule, call_conv: CallConv) -> Result<Self, String> {
        let ptr_ty = types::I64;

        // void nx_rt_vout_str(const char *str)
        let mut sig_vout_str = Signature::new(call_conv);
        sig_vout_str.params.push(AbiParam::new(ptr_ty));
        let vout_str = module
            .declare_function("nx_rt_vout_str", Linkage::Import, &sig_vout_str)
            .map_err(|e| e.to_string())?;

        // void nx_rt_vout_int(int64_t val)
        let mut sig_vout_int = Signature::new(call_conv);
        sig_vout_int.params.push(AbiParam::new(types::I64));
        let vout_int = module
            .declare_function("nx_rt_vout_int", Linkage::Import, &sig_vout_int)
            .map_err(|e| e.to_string())?;

        // void nx_rt_vout_float(double val)
        let mut sig_vout_float = Signature::new(call_conv);
        sig_vout_float.params.push(AbiParam::new(types::F64));
        let vout_float = module
            .declare_function("nx_rt_vout_float", Linkage::Import, &sig_vout_float)
            .map_err(|e| e.to_string())?;

        // void nx_rt_vout_bool(int8_t val)
        let mut sig_vout_bool = Signature::new(call_conv);
        sig_vout_bool.params.push(AbiParam::new(types::I8));
        let vout_bool = module
            .declare_function("nx_rt_vout_bool", Linkage::Import, &sig_vout_bool)
            .map_err(|e| e.to_string())?;

        // void nx_rt_vout_char(int32_t ch)
        let mut sig_vout_char = Signature::new(call_conv);
        sig_vout_char.params.push(AbiParam::new(types::I32));
        let vout_char = module
            .declare_function("nx_rt_vout_char", Linkage::Import, &sig_vout_char)
            .map_err(|e| e.to_string())?;

        // void nx_rt_vout_space(void)
        let sig_vout_space = Signature::new(call_conv);
        let vout_space = module
            .declare_function("nx_rt_vout_space", Linkage::Import, &sig_vout_space)
            .map_err(|e| e.to_string())?;

        // void nx_rt_vout_newline(void)
        let sig_vout_newline = Signature::new(call_conv);
        let vout_newline = module
            .declare_function("nx_rt_vout_newline", Linkage::Import, &sig_vout_newline)
            .map_err(|e| e.to_string())?;

        // const char* nx_rt_input(const char *prompt)
        let mut sig_input = Signature::new(call_conv);
        sig_input.params.push(AbiParam::new(ptr_ty));
        sig_input.returns.push(AbiParam::new(ptr_ty));
        let input = module
            .declare_function("nx_rt_input", Linkage::Import, &sig_input)
            .map_err(|e| e.to_string())?;

        // int64_t nx_rt_to_int(const char *str)
        let mut sig_to_int = Signature::new(call_conv);
        sig_to_int.params.push(AbiParam::new(ptr_ty));
        sig_to_int.returns.push(AbiParam::new(types::I64));
        let to_int = module
            .declare_function("nx_rt_to_int", Linkage::Import, &sig_to_int)
            .map_err(|e| e.to_string())?;

        // double nx_rt_to_float(const char *str)
        let mut sig_to_float = Signature::new(call_conv);
        sig_to_float.params.push(AbiParam::new(ptr_ty));
        sig_to_float.returns.push(AbiParam::new(types::F64));
        let to_float = module
            .declare_function("nx_rt_to_float", Linkage::Import, &sig_to_float)
            .map_err(|e| e.to_string())?;

        // int64_t nx_rt_len_str(const char *str)
        let mut sig_len = Signature::new(call_conv);
        sig_len.params.push(AbiParam::new(ptr_ty));
        sig_len.returns.push(AbiParam::new(types::I64));
        let len_str = module
            .declare_function("nx_rt_len_str", Linkage::Import, &sig_len)
            .map_err(|e| e.to_string())?;

        // void* nx_rt_alloc(int64_t size)
        let mut sig_alloc = Signature::new(call_conv);
        sig_alloc.params.push(AbiParam::new(types::I64));
        sig_alloc.returns.push(AbiParam::new(ptr_ty));
        let alloc = module
            .declare_function("nx_rt_alloc", Linkage::Import, &sig_alloc)
            .map_err(|e| e.to_string())?;

        // void nx_rt_free(void *ptr)
        let mut sig_free = Signature::new(call_conv);
        sig_free.params.push(AbiParam::new(ptr_ty));
        let free = module
            .declare_function("nx_rt_free", Linkage::Import, &sig_free)
            .map_err(|e| e.to_string())?;

        // void nx_rt_panic(const char *msg)
        let mut sig_panic = Signature::new(call_conv);
        sig_panic.params.push(AbiParam::new(ptr_ty));
        let panic = module
            .declare_function("nx_rt_panic", Linkage::Import, &sig_panic)
            .map_err(|e| e.to_string())?;

        // const char* nx_rt_str_concat(const char *a, const char *b)
        let mut sig_str_concat = Signature::new(call_conv);
        sig_str_concat.params.push(AbiParam::new(ptr_ty));
        sig_str_concat.params.push(AbiParam::new(ptr_ty));
        sig_str_concat.returns.push(AbiParam::new(ptr_ty));
        let str_concat = module
            .declare_function("nx_rt_str_concat", Linkage::Import, &sig_str_concat)
            .map_err(|e| e.to_string())?;

        // int8_t nx_rt_str_eq(const char *a, const char *b)
        let mut sig_str_eq = Signature::new(call_conv);
        sig_str_eq.params.push(AbiParam::new(ptr_ty));
        sig_str_eq.params.push(AbiParam::new(ptr_ty));
        sig_str_eq.returns.push(AbiParam::new(types::I8));
        let str_eq = module
            .declare_function("nx_rt_str_eq", Linkage::Import, &sig_str_eq)
            .map_err(|e| e.to_string())?;

        Ok(Self {
            vout_str,
            vout_int,
            vout_float,
            vout_bool,
            vout_char,
            vout_space,
            vout_newline,
            input,
            to_int,
            to_float,
            len_str,
            alloc,
            free,
            panic,
            str_concat,
            str_eq,
        })
    }
}
