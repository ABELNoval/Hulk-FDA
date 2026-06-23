#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodegenTarget {
    LlvmIr,
    LlvmBitcode,
    Native,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenContext {
    pub module_name: String,
    pub target: CodegenTarget,
    pub emit_comments: bool,
    pub target_triple: Option<String>,
    pub data_layout: Option<String>,
}

impl CodegenContext {
    pub const DEFAULT_TARGET_TRIPLE: &'static str = "x86_64-unknown-linux-gnu";
    pub const DEFAULT_DATA_LAYOUT: &'static str =
        "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128";

    pub fn new(module_name: impl Into<String>, target: CodegenTarget) -> Self {
        Self {
            module_name: module_name.into(),
            target,
            emit_comments: true,
            target_triple: None,
            data_layout: None,
        }
    }

    pub fn with_target_triple(mut self, triple: impl Into<String>) -> Self {
        self.target_triple = Some(triple.into());
        self
    }

    pub fn with_data_layout(mut self, layout: impl Into<String>) -> Self {
        self.data_layout = Some(layout.into());
        self
    }

    pub fn resolved_target_triple(&self) -> &str {
        self.target_triple
            .as_deref()
            .unwrap_or(Self::DEFAULT_TARGET_TRIPLE)
    }

    pub fn resolved_data_layout(&self) -> &str {
        self.data_layout
            .as_deref()
            .unwrap_or(Self::DEFAULT_DATA_LAYOUT)
    }

    pub fn without_comments(mut self) -> Self {
        self.emit_comments = false;
        self
    }
}

// =============================================================================
// LLVM Context, Module & Builder Lifecycle Management
// =============================================================================
//
// These types manage the lifecycle of LLVM compilation components:
// - LlvmContext: Global context for LLVM operations (reusable, thread-safe intent)
// - LlvmModule: Represents an LLVM module being built
// - LlvmBuilder: Accumulates IR generation state for the current module
//
// The ownership model ensures:
// 1. A module is tied to a specific context
// 2. A builder can only modify its associated module
// 3. Resources are cleaned up when dropped (future: via RAII with inkwell)
//

/// Global LLVM context.
/// Can be reused across multiple modules.
/// In the future, this will wrap `llvm::Context` from inkwell.
#[derive(Debug, Clone)]
pub struct LlvmContext {
    id: u64,
}

impl LlvmContext {
    /// Create a new LLVM context.
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static CONTEXT_COUNTER: AtomicU64 = AtomicU64::new(0);

        Self {
            id: CONTEXT_COUNTER.fetch_add(1, Ordering::SeqCst),
        }
    }

    /// Get the unique ID of this context.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Validate that this context is still valid.
    pub fn is_valid(&self) -> bool {
        true // TODO: check if context has been disposed
    }
}

impl Default for LlvmContext {
    fn default() -> Self {
        Self::new()
    }
}

/// LLVM Module state: represents a module being built.
/// Tracks all the data layout, target triple, and symbols registered so far.
#[derive(Debug, Clone)]
pub struct LlvmModule {
    /// Unique context this module belongs to.
    context_id: u64,

    /// Name of the module.
    name: String,

    /// Target triple (e.g., "x86_64-unknown-linux-gnu").
    target_triple: String,

    /// Data layout string.
    /// Example: "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128"
    data_layout: String,

    /// Symbol table: function names -> (return type, param types)
    /// Used to detect symbol conflicts and validate call sites.
    symbols: std::collections::HashMap<String, (String, Vec<String>)>,
    /// Declared prototypes for external or forward-declared symbols.
    /// Map: name -> (return_type, param_types, is_defined)
    prototypes: std::collections::HashMap<String, (String, Vec<String>, bool)>,
    /// Runtime declarations (external functions provided by the runtime)
    runtime_decls: std::collections::HashMap<String, (String, Vec<String>)>,

    /// Whether the module has been finalized (can no longer add symbols).
    finalized: bool,
}

impl LlvmModule {
    /// Create a new LLVM module.
    pub fn new(context: &LlvmContext, name: impl Into<String>) -> Self {
        Self {
            context_id: context.id(),
            name: name.into(),
            target_triple: CodegenContext::DEFAULT_TARGET_TRIPLE.to_string(),
            data_layout: CodegenContext::DEFAULT_DATA_LAYOUT.to_string(),
            symbols: std::collections::HashMap::new(),
            runtime_decls: std::collections::HashMap::new(),
            prototypes: std::collections::HashMap::new(),
            finalized: false,
        }
    }

    /// Get the module name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the target triple.
    pub fn target_triple(&self) -> &str {
        &self.target_triple
    }

    /// Set the target triple.
    pub fn set_target_triple(&mut self, triple: impl Into<String>) {
        if !self.finalized {
            self.target_triple = triple.into();
        }
    }

    /// Get the data layout string.
    pub fn data_layout(&self) -> &str {
        &self.data_layout
    }

    /// Set the data layout string.
    pub fn set_data_layout(&mut self, layout: impl Into<String>) {
        if !self.finalized {
            self.data_layout = layout.into();
        }
    }

    /// Register a function symbol in the module.
    /// Returns error if symbol already exists or module is finalized.
    pub fn register_symbol(
        &mut self,
        name: impl Into<String>,
        return_type: impl Into<String>,
        param_types: Vec<String>,
    ) -> Result<(), String> {
        let name_str = name.into();

        if self.finalized {
            return Err("Module is finalized; cannot register new symbols".to_string());
        }

        // For backwards compatibility treat this as defining the symbol
        self.define_symbol(name_str, return_type.into(), param_types)
    }

    /// Declare a prototype for an external function (no body).
    pub fn declare_symbol(
        &mut self,
        name: impl Into<String>,
        return_type: impl Into<String>,
        param_types: Vec<String>,
    ) -> Result<(), String> {
        let name_str = name.into();

        if self.finalized {
            return Err("Module is finalized; cannot register new symbols".to_string());
        }

        if self.runtime_decls.contains_key(&name_str) || self.symbols.contains_key(&name_str) {
            return Err(format!("Symbol '{}' already exists in module", name_str));
        }

        if let Some((ret, params, _)) = self.prototypes.get(&name_str) {
            if ret != &return_type.into() || params != &param_types {
                return Err(format!("Conflicting prototype for symbol '{}'", name_str));
            }
            // already declared; no-op
            return Ok(());
        }

        self.prototypes
            .insert(name_str, (return_type.into(), param_types, false));
        Ok(())
    }

    /// Define a symbol (provide its implementation). If a prototype exists, mark as defined.
    pub fn define_symbol(
        &mut self,
        name: impl Into<String>,
        return_type: impl Into<String>,
        param_types: Vec<String>,
    ) -> Result<(), String> {
        let name_str = name.into();

        if self.finalized {
            return Err("Module is finalized; cannot register new symbols".to_string());
        }

        if self.runtime_decls.contains_key(&name_str) {
            return Err(format!(
                "Symbol '{}' conflicts with runtime declaration",
                name_str
            ));
        }

        if self.symbols.contains_key(&name_str) {
            return Err(format!("Symbol '{}' already defined in module", name_str));
        }

        // Normalize inputs to avoid moving `return_type` multiple times
        let return_type_str = return_type.into();

        // If a prototype exists, verify signature matches
        if let Some((ret, params, _defined)) = self.prototypes.get(&name_str) {
            if ret != &return_type_str || params != &param_types {
                return Err(format!("Conflicting definition for symbol '{}'", name_str));
            }
            // mark prototype as defined and mirror into symbols
            self.prototypes
                .insert(name_str.clone(), (ret.clone(), params.clone(), true));
        }

        self.symbols
            .insert(name_str, (return_type_str, param_types));
        Ok(())
    }

    /// Register an external runtime declaration (e.g. print, malloc)
    pub fn register_runtime_decl(
        &mut self,
        name: impl Into<String>,
        return_type: impl Into<String>,
        param_types: Vec<String>,
    ) -> Result<(), String> {
        let name_str = name.into();

        if self.finalized {
            return Err(
                "Module is finalized; cannot register new runtime declarations".to_string(),
            );
        }

        if self.runtime_decls.contains_key(&name_str) || self.symbols.contains_key(&name_str) {
            return Err(format!("Symbol '{}' already exists in module", name_str));
        }

        self.runtime_decls
            .insert(name_str, (return_type.into(), param_types));
        Ok(())
    }

    /// Get runtime declarations
    pub fn runtime_decls(&self) -> &std::collections::HashMap<String, (String, Vec<String>)> {
        &self.runtime_decls
    }

    /// Get declared prototypes (including whether they are defined).
    pub fn prototypes(&self) -> &std::collections::HashMap<String, (String, Vec<String>, bool)> {
        &self.prototypes
    }

    /// Add a set of default runtime declarations used by lowering.
    pub fn add_default_runtime_decls(&mut self) {
        // print(ptr) -> void
        let _ = self.register_runtime_decl("print_number", "double", vec!["double".to_string()]);
        let _ = self.register_runtime_decl("print_string", "ptr", vec!["ptr".to_string()]);
        let _ = self.register_runtime_decl("print_bool", "i1", vec!["i1".to_string()]);

        let _ = self.register_runtime_decl("hulk_alloc", "ptr", vec!["i64".to_string()]);
        // hulk_free(ptr) -> void
        let _ = self.register_runtime_decl("hulk_free", "void", vec!["ptr".to_string()]);
        // hulk_strlen(ptr) -> i64
        let _ = self.register_runtime_decl("hulk_strlen", "i64", vec!["ptr".to_string()]);
    }

    /// Look up a function symbol.
    pub fn lookup_symbol(&self, name: &str) -> Option<(&str, &[String])> {
        self.symbols
            .get(name)
            .map(|(ret_ty, param_tys)| (ret_ty.as_str(), param_tys.as_slice()))
    }

    /// Get all registered symbols.
    pub fn symbols(&self) -> &std::collections::HashMap<String, (String, Vec<String>)> {
        &self.symbols
    }

    /// Finalize the module: prevent further symbol registration.
    pub fn finalize(&mut self) {
        self.finalized = true;
    }

    /// Check if the module is finalized.
    pub fn is_finalized(&self) -> bool {
        self.finalized
    }

    /// Verify the module is associated with the given context.
    pub fn verify_context(&self, context: &LlvmContext) -> Result<(), String> {
        if self.context_id != context.id() {
            return Err("Module context mismatch".to_string());
        }
        Ok(())
    }
}

/// LLVM Builder: accumulates IR generation state for a module.
/// Tracks the current function, block, and instruction stream.
#[derive(Debug)]
pub struct LlvmBuilder {
    /// The module this builder is associated with.
    module: LlvmModule,

    /// Current function being built (if any).
    current_function: Option<String>,

    /// Current basic block (if any).
    current_block: Option<String>,

    /// Instruction buffer.
    instructions: Vec<String>,
}

impl LlvmBuilder {
    /// Create a new builder for a module.
    pub fn new(module: LlvmModule) -> Self {
        Self {
            module,
            current_function: None,
            current_block: None,
            instructions: Vec::new(),
        }
    }

    /// Get a reference to the module.
    pub fn module(&self) -> &LlvmModule {
        &self.module
    }

    /// Get a mutable reference to the module.
    pub fn module_mut(&mut self) -> &mut LlvmModule {
        &mut self.module
    }

    /// Begin building a new function.
    pub fn begin_function(
        &mut self,
        name: impl Into<String>,
        return_type: impl Into<String>,
        param_types: Vec<String>,
    ) -> Result<(), String> {
        let name_str = name.into();
        let return_str = return_type.into();

        if self.current_function.is_some() {
            return Err("Cannot begin a function while another is in progress".to_string());
        }

        self.module
            .register_symbol(&name_str, &return_str, param_types)?;

        self.current_function = Some(name_str);
        self.current_block = None;
        self.instructions.clear();

        Ok(())
    }

    /// End the current function.
    pub fn end_function(&mut self) -> Result<String, String> {
        if self.current_function.is_none() {
            return Err("No function in progress".to_string());
        }

        let func_name = self.current_function.take();
        let instructions = std::mem::take(&mut self.instructions);
        self.current_block = None;

        Ok(format!(
            "define {{}}@{}(...) {{\n{}\n}}\n",
            func_name.unwrap(),
            instructions.join("\n")
        ))
    }

    /// Begin a new basic block.
    pub fn begin_block(&mut self, label: impl Into<String>) -> Result<(), String> {
        if self.current_function.is_none() {
            return Err("Cannot begin a block outside a function".to_string());
        }

        self.current_block = Some(label.into());
        Ok(())
    }

    /// Emit an instruction.
    pub fn emit_instruction(&mut self, instruction: impl Into<String>) -> Result<(), String> {
        if self.current_block.is_none() {
            return Err("Cannot emit instruction outside a block".to_string());
        }

        self.instructions.push(instruction.into());
        Ok(())
    }

    /// Get the current function name.
    pub fn current_function(&self) -> Option<&str> {
        self.current_function.as_deref()
    }

    /// Get the current block label.
    pub fn current_block(&self) -> Option<&str> {
        self.current_block.as_deref()
    }

    /// Consume the builder and return the module.
    pub fn finish(mut self) -> Result<LlvmModule, String> {
        if self.current_function.is_some() {
            return Err("Cannot finish builder while a function is in progress".to_string());
        }

        self.module.finalize();
        Ok(self.module)
    }
}
