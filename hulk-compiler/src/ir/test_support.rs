use super::block::{BasicBlock, BasicBlockId};
use super::instruction::IRInstruction;
use super::lowering::{IRLoweringError, IRLoweringResult};
use super::module::{IRFunction, IRModule};
use super::value::{IRValue, IRValueKind};

#[derive(Debug, Clone)]
pub struct IrTestHarness {
    module: IRModule,
}

impl IrTestHarness {
    pub fn new(module_name: impl Into<String>) -> Self {
        Self {
            module: IRModule::new(module_name),
        }
    }

    pub fn module(&self) -> &IRModule {
        &self.module
    }

    pub fn module_mut(&mut self) -> &mut IRModule {
        &mut self.module
    }

    pub fn into_module(self) -> IRModule {
        self.module
    }

    pub fn with_function(mut self, function_name: impl Into<String>) -> Self {
        self.module.add_function(IRFunction::new(function_name));
        self
    }

    pub fn ensure_function(&mut self, function_name: impl Into<String>) -> &mut IRFunction {
        let function_name = function_name.into();

        if self.module.function(&function_name).is_none() {
            self.module
                .add_function(IRFunction::new(function_name.clone()));
        }

        self.module
            .function_mut(&function_name)
            .expect("function must exist after insertion")
    }

    pub fn add_block(&mut self, function_name: impl Into<String>, block_name: impl Into<String>) {
        let function = self.ensure_function(function_name);
        function.add_block(BasicBlock::new(block_name));
    }

    pub fn add_instruction(
        &mut self,
        function_name: impl Into<String>,
        block_name: impl Into<String>,
        instruction: IRInstruction,
    ) {
        let function_name = function_name.into();
        let block_name = block_name.into();
        let function = self.ensure_function(function_name);

        if function
            .block(&BasicBlockId::new(block_name.clone()))
            .is_none()
        {
            function.add_block(BasicBlock::new(block_name.clone()));
        }

        let block = function
            .block_mut(&BasicBlockId::new(block_name))
            .expect("block must exist after insertion");
        block.push_instruction(instruction);
    }

    pub fn add_parameter(
        &mut self,
        function_name: impl Into<String>,
        value_name: impl Into<String>,
        ty: impl Into<String>,
    ) {
        let function = self.ensure_function(function_name);
        function
            .parameters
            .push(IRValue::new(value_name, IRValueKind::Parameter).with_type(ty));
    }

    pub fn set_return_type(
        &mut self,
        function_name: impl Into<String>,
        return_type: impl Into<String>,
    ) {
        let function = self.ensure_function(function_name);
        function.return_type = Some(return_type.into());
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedModule {
    pub name: String,
    pub functions: Vec<NormalizedFunction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedFunction {
    pub name: String,
    pub return_type: Option<String>,
    pub parameters: Vec<NormalizedValue>,
    pub blocks: Vec<NormalizedBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedBlock {
    pub id: String,
    pub predecessors: Vec<String>,
    pub successors: Vec<String>,
    pub instructions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedValue {
    pub id: String,
    pub kind: String,
    pub ty: Option<String>,
}

pub fn normalize_module(module: &IRModule) -> NormalizedModule {
    NormalizedModule {
        name: module.name.clone(),
        functions: module.functions.iter().map(normalize_function).collect(),
    }
}

pub fn render_module(module: &IRModule) -> String {
    let normalized = normalize_module(module);
    format!("{normalized:#?}")
}

pub fn assert_module_eq(actual: &IRModule, expected: &IRModule) {
    assert_eq!(normalize_module(actual), normalize_module(expected));
}

pub fn assert_rendered_module_contains(module: &IRModule, expected_fragment: &str) {
    let rendered = render_module(module);
    assert!(
        rendered.contains(expected_fragment),
        "expected fragment `{}` not found in rendered module:\n{}",
        expected_fragment,
        rendered
    );
}

pub fn assert_lowering_ok(result: Result<IRModule, IRLoweringError>) -> IRModule {
    match result {
        Ok(module) => module,
        Err(error) => panic!("expected lowering to succeed, got error: {}", error),
    }
}

pub fn assert_lowering_error(result: Result<IRModule, IRLoweringError>) -> IRLoweringError {
    match result {
        Ok(_) => panic!("expected lowering to fail, but it succeeded"),
        Err(error) => error,
    }
}

pub fn assert_lowering_result(result: IRLoweringResult) -> IRLoweringResult {
    result
}

fn normalize_function(function: &IRFunction) -> NormalizedFunction {
    NormalizedFunction {
        name: function.name.clone(),
        return_type: function.return_type.clone(),
        parameters: function.parameters.iter().map(normalize_value).collect(),
        blocks: function.blocks.iter().map(normalize_block).collect(),
    }
}

fn normalize_block(block: &BasicBlock) -> NormalizedBlock {
    NormalizedBlock {
        id: block.id.0.clone(),
        predecessors: block
            .predecessors
            .iter()
            .map(|predecessor| predecessor.0.clone())
            .collect(),
        successors: block
            .successors
            .iter()
            .map(|successor| successor.0.clone())
            .collect(),
        instructions: block
            .instructions
            .iter()
            .map(|instruction| format!("{:?}", instruction.kind))
            .collect(),
    }
}

fn normalize_value(value: &IRValue) -> NormalizedValue {
    NormalizedValue {
        id: value.id.0.clone(),
        kind: format!("{:?}", value.kind),
        ty: value.ty.clone(),
    }
}
