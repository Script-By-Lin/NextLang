#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JsTarget {
    #[default]
    Node,
    Browser,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModuleFormat {
    #[default]
    CommonJS,
    ESModule,
}

#[derive(Debug, Clone)]
pub struct CompilerConfig {
    pub target: JsTarget,
    pub module_format: ModuleFormat,
    pub emit_runtime_helpers: bool,
    pub auto_invoke_main: bool,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            target: JsTarget::Node,
            module_format: ModuleFormat::CommonJS,
            emit_runtime_helpers: true,
            auto_invoke_main: true,
        }
    }
}

impl CompilerConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_target(mut self, target: JsTarget) -> Self {
        self.target = target;
        self
    }

    pub fn with_module_format(mut self, format: ModuleFormat) -> Self {
        self.module_format = format;
        self
    }

    pub fn with_auto_invoke_main(mut self, invoke: bool) -> Self {
        self.auto_invoke_main = invoke;
        self
    }
}
