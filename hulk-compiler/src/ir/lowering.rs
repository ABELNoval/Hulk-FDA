// Persona 3 — AST lowering into IR
// Assigned: Persona3
// Responsibilities: implementar la pasarela que convierte `Program` (AST)
// en `IRModule`. Maneja expresiones, variables, control flow y conecta con el
// builder sin exponer detalles internos.

use std::collections::HashMap;

use crate::ir::module::IRGlobal;
use crate::parser::ast::{
    BinaryOperator, DeclarationKind, Expr, ExprKind, FunctionDeclaration, Literal, Parameter,
    Program, TypeDeclaration, TypeMember, TypeReference, UnaryOperator,
};
use crate::semantic::type_system::NormalizedType;
use crate::utils::errors::span::Span;

use super::block::{BasicBlock, BasicBlockId};
use super::instruction::{IRBinaryOp, IRInstruction, IRInstructionKind, IROperand, IRUnaryOp};
use super::module::{IRFunction, IRModule};
use super::naming::{IRNaming, SSAValueGenerator};
use super::value::{IRValue, IRValueId, IRValueKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IRLoweringError {
    pub message: String,
    pub location: Option<String>,
}

impl IRLoweringError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            location: None,
        }
    }

    pub fn at(message: impl Into<String>, location: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            location: Some(location.into()),
        }
    }
}

impl std::fmt::Display for IRLoweringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.location {
            Some(location) => write!(f, "{} ({})", self.message, location),
            None => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for IRLoweringError {}

pub trait IRLoweringContext {
    fn lower_program(&mut self, program: &Program) -> Result<IRModule, IRLoweringError>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct IRLoweringResult {
    pub module: IRModule,
    pub warnings: Vec<String>,
}

impl IRLoweringResult {
    pub fn new(module: IRModule) -> Self {
        Self {
            module,
            warnings: Vec::new(),
        }
    }

    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }
}

#[derive(Debug, Clone)]
struct LoopContext {
    continue_block: BasicBlockId,
    break_block: BasicBlockId,
}

#[derive(Debug)]
pub struct IRBuilder {
    module: IRModule,
    current_function: Option<String>,
    current_block: Option<BasicBlockId>,
    current_method: Option<(String, String)>,
    value_generator: SSAValueGenerator,
    block_counter: usize,
    scopes: Vec<HashMap<String, IRValueId>>,
    loop_stack: Vec<LoopContext>,
    expr_types: HashMap<usize, NormalizedType>,
    type_layouts: HashMap<String, Vec<(String, String)>>, // type_name -> [(field_name, field_type)]
    type_parents: HashMap<String, String>,
    type_methods: HashMap<String, Vec<String>>, // type -> [method names]
    type_vtables: HashMap<String, Vec<(String, String)>>, // type -> [(method, mangled)]
    method_sigs: HashMap<String, (Option<String>, Vec<String>)>, // mangled -> (ret, param_tys)
}

impl IRBuilder {
    pub fn new(module_name: impl Into<String>) -> Self {
        Self {
            module: IRModule::new(module_name),
            current_function: None,
            current_block: None,
            value_generator: SSAValueGenerator::new(),
            block_counter: 0,
            scopes: Vec::new(),
            loop_stack: Vec::new(),
            expr_types: HashMap::new(),
            type_layouts: HashMap::new(),
            type_parents: HashMap::new(),
            type_methods: HashMap::new(),
            type_vtables: HashMap::new(),
            method_sigs: HashMap::new(),
            current_method: None,
        }
    }

    fn collect_type_methods(&mut self, program: &Program) {
        for decl in &program.declarations {
            if let DeclarationKind::Type(td) = &decl.kind {
                let methods: Vec<String> = td
                    .members
                    .iter()
                    .filter_map(|m| {
                        if let TypeMember::Method(method) = m {
                            Some(method.name.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                self.type_methods.insert(td.name.clone(), methods);
            }
        }
    }

    fn build_vtable_list(&self, type_name: &str) -> Vec<(String, String)> {
        let mut result = Vec::new();
        if let Some(parent) = self.type_parents.get(type_name) {
            result = self.build_vtable_list(parent);
        }
        if let Some(own) = self.type_methods.get(type_name) {
            for method in own {
                let mangled = format!("{}_{}", type_name, method);
                if let Some(pos) = result.iter().position(|(m, _)| m == method) {
                    result[pos] = (method.clone(), mangled); // override
                } else {
                    result.push((method.clone(), mangled));
                }
            }
        }
        result
    }

    fn emit_vtable_globals(&mut self) {
        for (type_name, methods) in &self.type_vtables {
            if methods.is_empty() {
                continue;
            }
            let global_name = format!("__vtable_{}", type_name);
            let func_names: Vec<String> = methods.iter().map(|(_, m)| m.clone()).collect();
            self.module.add_global(IRGlobal {
                name: global_name,
                element_ty: "ptr".to_string(),
                values: func_names,
            });
        }
    }

    fn get_vtable_index(&self, type_name: &str, method: &str) -> Option<usize> {
        self.type_vtables
            .get(type_name)?
            .iter()
            .position(|(m, _)| m == method)
    }

    fn get_method_signature(
        &self,
        type_name: &str,
        method: &str,
    ) -> Option<(Option<String>, Vec<String>)> {
        let owner = self.find_method_owner(type_name, method)?;
        let mangled = format!("{}_{}", owner, method);
        self.method_sigs.get(&mangled).cloned()
    }

    pub fn create_function(
        &mut self,
        name: impl Into<String>,
        parameters: Vec<IRValue>,
        return_type: Option<String>,
    ) -> Result<(), IRLoweringError> {
        let name_str = name.into();

        if self.module.function(&name_str).is_some() {
            return Err(IRLoweringError::new(format!(
                "Function '{}' already exists",
                name_str
            )));
        }

        let mut func = IRFunction::new(name_str.clone());
        func.parameters = parameters;
        func.return_type = return_type;

        self.module.add_function(func);
        self.current_function = Some(name_str);
        self.current_block = None;
        self.scopes.clear();
        self.loop_stack.clear();
        Ok(())
    }

    pub fn create_block_in_function(
        &mut self,
        function_name: impl AsRef<str>,
        block_name: impl Into<String>,
    ) -> Result<(), IRLoweringError> {
        let func_name = function_name.as_ref();
        let block_name_str = block_name.into();

        let func = self
            .module
            .function_mut(func_name)
            .ok_or_else(|| IRLoweringError::new(format!("Function '{}' not found", func_name)))?;

        if func.block(&BasicBlockId::new(&block_name_str)).is_some() {
            return Err(IRLoweringError::new(format!(
                "Block '{}' already exists in function '{}'",
                block_name_str, func_name
            )));
        }

        let block = BasicBlock::new(block_name_str);
        func.add_block(block);
        Ok(())
    }

    pub fn add_instruction_to_block(
        &mut self,
        function_name: impl AsRef<str>,
        block_name: impl AsRef<str>,
        instruction: IRInstruction,
    ) -> Result<(), IRLoweringError> {
        let func_name = function_name.as_ref();
        let block_name_str = block_name.as_ref();

        let func = self
            .module
            .function_mut(func_name)
            .ok_or_else(|| IRLoweringError::new(format!("Function '{}' not found", func_name)))?;

        let block_id = BasicBlockId::new(block_name_str);
        let block = func.block_mut(&block_id).ok_or_else(|| {
            IRLoweringError::new(format!(
                "Block '{}' not found in function '{}'",
                block_name_str, func_name
            ))
        })?;

        block.push_instruction(instruction);
        Ok(())
    }

    pub fn set_current_function(&mut self, name: impl Into<String>) {
        self.current_function = Some(name.into());
        self.current_block = None;
    }

    pub fn current_function(&self) -> Option<&str> {
        self.current_function.as_deref()
    }

    pub fn function_exists(&self, name: &str) -> bool {
        self.module.function(name).is_some()
    }

    pub fn block_exists(&self, function_name: &str, block_name: &str) -> bool {
        match self.module.function(function_name) {
            Some(func) => func.block(&BasicBlockId::new(block_name)).is_some(),
            None => false,
        }
    }

    pub fn function_count(&self) -> usize {
        self.module.function_count()
    }

    pub fn build(self) -> IRModule {
        self.module
    }

    pub fn fmt_display(&self) -> String {
        self.module.fmt_display()
    }

    /// Set pre-computed expression types produced by the semantic analyzer.
    pub fn set_expr_types(&mut self, types: std::collections::HashMap<usize, NormalizedType>) {
        self.expr_types = types;
    }

    pub fn lower_program(&mut self, program: &Program) -> Result<IRModule, IRLoweringError> {
        self.reset();
        self.module = IRModule::new(IRNaming::module_name("lowered"));

        // 1. Registrar layouts y firmas (sin todavía emitir constructores/métodos)
        for decl in &program.declarations {
            if let DeclarationKind::Type(td) = &decl.kind {
                self.lower_type_declaration(td)?; // extrae lo que antes hacía lower_type_declaration
            }
        }

        // 2. Recolectar métodos y construir vtables
        self.collect_type_methods(program);
        let type_names: Vec<String> = self.type_methods.keys().cloned().collect();
        for name in &type_names {
            let vtable = self.build_vtable_list(name);
            self.type_vtables.insert(name.clone(), vtable);
        }
        self.emit_vtable_globals();

        // 3. Ahora sí emitir funciones, métodos y constructores
        for decl in &program.declarations {
            match &decl.kind {
                DeclarationKind::Function(f) => self.lower_function_declaration(f)?,
                DeclarationKind::Type(td) => {
                    for member in &td.members {
                        if let TypeMember::Method(m) = member {
                            self.lower_method_declaration(&td.name, m)?;
                        }
                    }
                    self.lower_constructor(td)?;
                }
                _ => {}
            }
        }

        if program.entry_expression.is_some() {
            self.lower_entry_function(program)?;
        }

        Ok(self.module.clone())
    }

    fn reset(&mut self) {
        self.current_function = None;
        self.current_block = None;
        self.current_method = None;
        self.value_generator.reset();
        self.block_counter = 0;
        self.scopes.clear();
        self.loop_stack.clear();
        self.type_methods.clear();
        self.type_vtables.clear();
        self.method_sigs.clear();
    }

    fn lower_entry_function(&mut self, program: &Program) -> Result<(), IRLoweringError> {
        let entry_name = "__entry".to_string();

        self.create_function(entry_name.clone(), Vec::new(), Some("i64".to_string()))?;

        self.create_block_in_function(&entry_name, "entry")?;

        self.set_current_block(entry_name.clone(), "entry")?;

        self.push_scope();

        match &program.entry_expression {
            Some(entry_expression) => {
                let _ = self.lower_expr(entry_expression)?;
            }

            None => {
                // No top-level entry expression
            }
        }

        if !self.current_block_terminated()? {
            self.emit(IRInstruction::new(IRInstructionKind::Return(Some(
                IROperand::Integer(0),
            ))))?;
        }

        self.pop_scope();

        Ok(())
    }

    fn lower_function_declaration(
        &mut self,
        function: &FunctionDeclaration,
    ) -> Result<(), IRLoweringError> {
        let parameters = function
            .parameters
            .iter()
            .enumerate()
            .map(|(index, parameter)| self.lower_parameter(parameter, index))
            .collect::<Result<Vec<_>, _>>()?;

        // EXTRAER ANTES de mover parameters a create_function
        let param_bindings: Vec<(String, IRValueId)> = parameters
            .iter()
            .map(|p| (p.id.0.clone(), p.id.clone()))
            .collect();

        let return_type = function
            .return_type
            .as_ref()
            .map(|t| t.display_name())
            .or_else(|| {
                self.expr_types
                    .get(&function.body.id)
                    .map(|t| t.to_string())
            });

        self.create_function(function.name.clone(), parameters, return_type)?;

        self.create_block_in_function(&function.name, "entry")?;
        self.set_current_block(function.name.clone(), "entry")?;
        self.push_scope();

        // Usar param_bindings (valores propios, no borrow de parameters)
        for (ast_param, (_, ir_param_id)) in function.parameters.iter().zip(param_bindings.iter()) {
            self.define_variable(&ast_param.name, ir_param_id.clone());
        }

        let body_value = self.lower_expr(&function.body)?;
        if !self.current_block_terminated()? {
            self.emit(IRInstruction::new(IRInstructionKind::Return(Some(
                IROperand::Value(body_value),
            ))))?;
        }

        self.pop_scope();
        Ok(())
    }

    fn lower_parameter(
        &mut self,
        parameter: &Parameter,
        index: usize,
    ) -> Result<IRValue, IRLoweringError> {
        let parameter_name = if parameter.name.is_empty() {
            IRNaming::parameter_name(index)
        } else {
            parameter.name.clone()
        };

        let mut value = IRValue::parameter(parameter_name).with_span(parameter.span.clone());
        value.ty = parameter
            .annotation
            .as_ref()
            .map(|type_ref| type_ref.display_name());
        Ok(value)
    }

    fn lower_type_declaration(
        &mut self,
        type_decl: &TypeDeclaration,
    ) -> Result<(), IRLoweringError> {
        let mut fields = Vec::new();
        if let Some(parent_ref) = &type_decl.inherits {
            let parent_name = parent_ref.display_name();
            if let Some(parent_fields) = self.type_layouts.get(&parent_name).cloned() {
                for (name, ty) in parent_fields {
                    fields.push((name, ty));
                }
            }
            self.type_parents
                .insert(type_decl.name.clone(), parent_name);
        }
        // Mapa de parámetros del tipo para inferir tipos de atributos
        let param_types: std::collections::HashMap<String, String> = type_decl
            .parameters
            .iter()
            .filter_map(|p| {
                p.annotation
                    .as_ref()
                    .map(|a| (p.name.clone(), a.display_name()))
            })
            .collect();

        // Todos los atributos explícitos son campos accesibles
        for member in &type_decl.members {
            if let TypeMember::Attribute(a) = member {
                let field_type = a
                    .annotation
                    .as_ref()
                    .map(|ann| ann.display_name())
                    .unwrap_or_else(|| {
                        // Inferir tipo del inicializador si es un identificador conocido
                        if let ExprKind::Identifier(name) = &a.initializer.kind
                            && let Some(ty) = param_types.get(name)
                        {
                            return ty.clone();
                        }
                        // Fallback a expr_types del semantic analyzer
                        self.expr_types
                            .get(&a.initializer.id)
                            .map(|t| t.to_string())
                            .unwrap_or("ptr".to_string())
                    });
                fields.push((a.name.clone(), field_type));
            }
        }
        self.type_layouts.insert(type_decl.name.clone(), fields);

        // Registrar firmas de métodos para poder usarlas en CallIndirect
        for member in &type_decl.members {
            if let TypeMember::Method(method) = member {
                let mangled = format!("{}_{}", type_decl.name, method.name);
                let ret = method.return_type.as_ref().map(|t| t.display_name());
                let params = method
                    .parameters
                    .iter()
                    .map(|p| {
                        p.annotation
                            .as_ref()
                            .map(|a| a.display_name())
                            .unwrap_or("ptr".to_string())
                    })
                    .collect();
                self.method_sigs.insert(mangled, (ret, params));
            }
        }
        Ok(())
    }

    fn find_method_owner(&self, type_name: &str, method_name: &str) -> Option<String> {
        let key = format!("{}_{}", type_name, method_name);
        if self.module.function(&key).is_some() {
            return Some(type_name.to_string());
        }
        if let Some(parent) = self.type_parents.get(type_name) {
            return self.find_method_owner(parent, method_name);
        }
        None
    }

    fn lower_method_declaration(
        &mut self,
        type_name: &str,
        method: &FunctionDeclaration,
    ) -> Result<(), IRLoweringError> {
        let mangled_name = format!("{}_{}", type_name, method.name);

        // Construir parámetros IR: self primero, luego los del AST
        let mut parameters = vec![IRValue::parameter("self").with_type("ptr")];
        for (idx, param) in method.parameters.iter().enumerate() {
            parameters.push(self.lower_parameter(param, idx + 1)?);
        }

        // Extraer (nombre, id) ANTES de mover parameters a create_function
        let scope_bindings: Vec<(String, IRValueId)> = parameters
            .iter()
            .map(|p| (p.id.0.clone(), p.id.clone()))
            .collect();

        let return_type = method
            .return_type
            .as_ref()
            .map(|t| t.display_name())
            .or_else(|| self.expr_types.get(&method.body.id).map(|t| t.to_string()));
        self.create_function(mangled_name.clone(), parameters, return_type)?;
        self.create_block_in_function(&mangled_name, "entry")?;
        self.set_current_block(mangled_name.clone(), "entry")?;
        self.push_scope();

        // Definir self y parámetros en el scope
        for (name, id) in scope_bindings {
            self.define_variable(&name, id);
        }
        self.current_method = Some((type_name.to_string(), method.name.clone()));
        let body_value = self.lower_expr(&method.body)?;
        if !self.current_block_terminated()? {
            self.emit(IRInstruction::new(IRInstructionKind::Return(Some(
                IROperand::Value(body_value),
            ))))?;
        }

        self.pop_scope();
        Ok(())
    }

    fn lower_constructor(&mut self, type_decl: &TypeDeclaration) -> Result<(), IRLoweringError> {
        let ctor_name = format!("new_{}", type_decl.name);

        let parameters: Vec<IRValue> = type_decl
            .parameters
            .iter()
            .enumerate()
            .map(|(i, p)| self.lower_parameter(p, i))
            .collect::<Result<_, _>>()?;

        let param_bindings: Vec<(String, IRValueId)> = parameters
            .iter()
            .map(|p| (p.id.0.clone(), p.id.clone()))
            .collect();

        self.create_function(&ctor_name, parameters, Some("ptr".to_string()))?;
        self.create_block_in_function(&ctor_name, "entry")?;
        self.set_current_block(ctor_name.clone(), "entry")?;
        self.push_scope();

        for (name, id) in param_bindings {
            self.define_variable(&name, id);
        }

        // Campos heredados del padre
        let parent_fields = if let Some(parent_ref) = &type_decl.inherits {
            let parent_name = parent_ref.display_name();
            self.type_layouts
                .get(&parent_name)
                .cloned()
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        // Atributos explícitos propios
        let own_attrs: Vec<_> = type_decl
            .members
            .iter()
            .filter_map(|m| {
                if let TypeMember::Attribute(a) = m {
                    Some(a)
                } else {
                    None
                }
            })
            .collect();

        let total_fields = parent_fields.len() + own_attrs.len();
        let total_size = std::cmp::max(8, (total_fields as i64 + 1) * 8);

        // Allocar
        let obj_ptr = self.fresh_value();
        self.emit(IRInstruction::new(IRInstructionKind::Call {
            target: obj_ptr.clone(),
            callee: "hulk_alloc".to_string(),
            arguments: vec![IROperand::Integer(total_size)],
            original: None,
        }))?;

        // Evaluar parent_arguments
        let parent_arg_values: Vec<IRValueId> = type_decl
            .parent_arguments
            .iter()
            .map(|arg| self.lower_expr(arg))
            .collect::<Result<_, _>>()?;

        // Store vtable pointer at object offset 0
        let vtable_global = format!("__vtable_{}", type_decl.name);
        let vtable_addr = self.fresh_value();
        self.emit(IRInstruction::new(IRInstructionKind::Assign {
            target: vtable_addr.clone(),
            value: IROperand::Global(vtable_global),
            original: None,
        }))?;
        self.emit(IRInstruction::new(IRInstructionKind::Store {
            address: IROperand::Value(obj_ptr.clone()),
            value: IROperand::Value(vtable_addr),
        }))?;

        // 1. Guardar campos heredados
        for (idx, (_, field_type)) in parent_fields.iter().enumerate() {
            let offset = (idx + 1) as i64;
            let gep_target = self.fresh_value();
            self.emit(IRInstruction::new(IRInstructionKind::GetElementPtr {
                target: gep_target.clone(),
                base: IROperand::Value(obj_ptr.clone()),
                indices: vec![IROperand::Integer(offset)],
                element_type: field_type.clone(),
            }))?;
            let arg_value = parent_arg_values
                .get(idx)
                .cloned()
                .unwrap_or_else(|| self.fresh_value());
            self.emit(IRInstruction::new(IRInstructionKind::Store {
                address: IROperand::Value(gep_target),
                value: IROperand::Value(arg_value),
            }))?;
        }

        let param_types: std::collections::HashMap<String, String> = type_decl
            .parameters
            .iter()
            .filter_map(|p| {
                p.annotation
                    .as_ref()
                    .map(|a| (p.name.clone(), a.display_name()))
            })
            .collect();

        // 2. Guardar atributos propios
        for (idx, attr) in own_attrs.iter().enumerate() {
            let offset = ((parent_fields.len() + idx) + 1) as i64;
            let field_type = attr
                .annotation
                .as_ref()
                .map(|a| a.display_name())
                .unwrap_or_else(|| {
                    // Inferir tipo del inicializador si es un identificador cocido
                    if let ExprKind::Identifier(name) = &attr.initializer.kind
                        && let Some(ty) = param_types.get(name)
                    {
                        return ty.clone();
                    }
                    self.expr_types
                        .get(&attr.initializer.id)
                        .map(|t| t.to_string())
                        .unwrap_or("ptr".to_string())
                });
            let gep_target = self.fresh_value();
            self.emit(IRInstruction::new(IRInstructionKind::GetElementPtr {
                target: gep_target.clone(),
                base: IROperand::Value(obj_ptr.clone()),
                indices: vec![IROperand::Integer(offset)],
                element_type: field_type.clone(),
            }))?;
            let init_value = self.lower_expr(&attr.initializer)?;
            self.emit(IRInstruction::new(IRInstructionKind::Store {
                address: IROperand::Value(gep_target),
                value: IROperand::Value(init_value),
            }))?;
        }

        self.emit(IRInstruction::new(IRInstructionKind::Return(Some(
            IROperand::Value(obj_ptr),
        ))))?;

        self.pop_scope();
        Ok(())
    }

    fn lower_expr(&mut self, expr: &Expr) -> Result<IRValueId, IRLoweringError> {
        match &expr.kind {
            ExprKind::Literal(literal) => self.lower_literal(literal),
            ExprKind::Identifier(name) => self.lookup_variable(name).ok_or_else(|| {
                Self::error_at(
                    expr.span.clone(),
                    format!("Variable '{}' is not defined", name),
                )
            }),
            ExprKind::Unary { operator, operand } => self.lower_unary(operator, operand, expr),
            ExprKind::Binary {
                left,
                operator,
                right,
            } => self.lower_binary(left, operator, right, expr),
            ExprKind::Block(expressions) => self.lower_block(expressions, expr),
            ExprKind::Call { callee, arguments } => self.lower_call(callee, arguments),
            ExprKind::Assignment { target, value } => self.lower_assignment(target, value, expr),
            ExprKind::If {
                condition,
                then_expr,
                elif_parts,
                else_expr,
            } => self.lower_if(condition, then_expr, elif_parts, else_expr, expr),
            ExprKind::While { condition, body } => self.lower_while(condition, body, expr),
            ExprKind::For {
                variable,
                iterable,
                body,
            } => self.lower_for(variable, iterable, body, expr),
            ExprKind::Let {
                name,
                annotation,
                value,
                body,
            } => self.lower_let(name, annotation, value, body, expr),
            ExprKind::MemberAccess { object, member } => {
                self.lower_member_access(object, member, expr)
            }
            ExprKind::IndexAccess { object, index } => self.lower_index_access(object, index, expr),
            ExprKind::TypeCheck {
                expr: inner,
                type_ref,
            } => self.lower_type_check(inner, type_ref, expr),
            ExprKind::TypeCast {
                expr: inner,
                type_ref,
            } => self.lower_type_cast(inner, type_ref, expr),
            ExprKind::New {
                type_ref,
                arguments,
            } => self.lower_new(type_ref, arguments, expr),
            ExprKind::Self_ => self.lower_self(expr),
            ExprKind::Base => self.lower_base(expr),
            ExprKind::VectorLiteral(elements) => self.lower_vector_literal(elements, expr),
            ExprKind::VectorComprehension {
                element_expr,
                binding,
                iterable,
            } => self.lower_vector_comprehension(element_expr, binding, iterable, expr),
        }
    }

    fn lower_literal(&mut self, literal: &Literal) -> Result<IRValueId, IRLoweringError> {
        match literal {
            Literal::Number(number) => self.emit_constant(IROperand::Float(*number), None),
            Literal::String(text) => self.emit_constant(IROperand::Text(text.clone()), None),
            Literal::Boolean(value) => self.emit_constant_boolean(*value, IRValueKind::Temporary),
            Literal::Pi => self.emit_constant(IROperand::Float(std::f64::consts::PI), None),
            Literal::E => self.emit_constant(IROperand::Float(std::f64::consts::E), None),
        }
    }

    fn lower_unary(
        &mut self,
        operator: &UnaryOperator,
        operand: &Expr,
        _expr: &Expr,
    ) -> Result<IRValueId, IRLoweringError> {
        let operand_value = self.lower_expr(operand)?;

        match operator {
            UnaryOperator::Minus | UnaryOperator::Not => {
                let target = self.fresh_value();
                let op = match operator {
                    UnaryOperator::Minus => IRUnaryOp::Neg,
                    UnaryOperator::Not => IRUnaryOp::Not,
                };
                self.emit(IRInstruction::new(IRInstructionKind::Unary {
                    target: target.clone(),
                    op,
                    operand: IROperand::Value(operand_value),
                }))?;
                Ok(target)
            }
        }
    }

    fn lower_binary(
        &mut self,
        left: &Expr,
        operator: &BinaryOperator,
        right: &Expr,
        _expr: &Expr,
    ) -> Result<IRValueId, IRLoweringError> {
        let left_value = self.lower_expr(left)?;
        let right_value = self.lower_expr(right)?;
        let left_operand = IROperand::Value(left_value.clone());
        let right_operand = IROperand::Value(right_value.clone());

        let target = self.fresh_value();
        let instruction = match operator {
            BinaryOperator::Add => IRInstructionKind::Binary {
                target: target.clone(),
                op: IRBinaryOp::Add,
                left: left_operand.clone(),
                right: right_operand.clone(),
            },
            BinaryOperator::Subtract => IRInstructionKind::Binary {
                target: target.clone(),
                op: IRBinaryOp::Sub,
                left: left_operand.clone(),
                right: right_operand.clone(),
            },
            BinaryOperator::Multiply => IRInstructionKind::Binary {
                target: target.clone(),
                op: IRBinaryOp::Mul,
                left: left_operand.clone(),
                right: right_operand.clone(),
            },
            BinaryOperator::Divide => IRInstructionKind::Binary {
                target: target.clone(),
                op: IRBinaryOp::Div,
                left: left_operand.clone(),
                right: right_operand.clone(),
            },
            BinaryOperator::And => IRInstructionKind::Binary {
                target: target.clone(),
                op: IRBinaryOp::And,
                left: left_operand.clone(),
                right: right_operand.clone(),
            },
            BinaryOperator::Or => IRInstructionKind::Binary {
                target: target.clone(),
                op: IRBinaryOp::Or,
                left: left_operand.clone(),
                right: right_operand.clone(),
            },
            BinaryOperator::Equal => IRInstructionKind::Binary {
                target: target.clone(),
                op: IRBinaryOp::Eq,
                left: left_operand.clone(),
                right: right_operand.clone(),
            },
            BinaryOperator::NotEqual => IRInstructionKind::Binary {
                target: target.clone(),
                op: IRBinaryOp::Ne,
                left: left_operand.clone(),
                right: right_operand.clone(),
            },
            BinaryOperator::Less => IRInstructionKind::Binary {
                target: target.clone(),
                op: IRBinaryOp::Lt,
                left: left_operand.clone(),
                right: right_operand.clone(),
            },
            BinaryOperator::LessEqual => IRInstructionKind::Binary {
                target: target.clone(),
                op: IRBinaryOp::Le,
                left: left_operand.clone(),
                right: right_operand.clone(),
            },
            BinaryOperator::Greater => IRInstructionKind::Binary {
                target: target.clone(),
                op: IRBinaryOp::Gt,
                left: left_operand.clone(),
                right: right_operand.clone(),
            },
            BinaryOperator::GreaterEqual => IRInstructionKind::Binary {
                target: target.clone(),
                op: IRBinaryOp::Ge,
                left: left_operand.clone(),
                right: right_operand.clone(),
            },
            BinaryOperator::Power => IRInstructionKind::Binary {
                target: target.clone(),
                op: IRBinaryOp::Pow,
                left: left_operand.clone(),
                right: right_operand.clone(),
            },
            BinaryOperator::Modulo => IRInstructionKind::Binary {
                target: target.clone(),
                op: IRBinaryOp::Mod,
                left: left_operand.clone(),
                right: right_operand.clone(),
            },
            BinaryOperator::Concat => IRInstructionKind::Binary {
                target: target.clone(),
                op: IRBinaryOp::Concat,
                left: left_operand.clone(),
                right: right_operand.clone(),
            },
            BinaryOperator::Concatenate => IRInstructionKind::Binary {
                target: target.clone(),
                op: IRBinaryOp::Concatenate,
                left: left_operand.clone(),
                right: right_operand.clone(),
            },
        };

        self.emit(IRInstruction::new(instruction))?;
        Ok(target)
    }

    fn lower_block(
        &mut self,
        expressions: &[Expr],
        _expr: &Expr,
    ) -> Result<IRValueId, IRLoweringError> {
        self.push_scope();
        let mut last_value = self.emit_constant_boolean(false, IRValueKind::Temporary)?;

        for child in expressions {
            last_value = self.lower_expr(child)?;
            if self.current_block_terminated()? {
                break;
            }
        }

        self.pop_scope();
        if expressions.is_empty() {
            self.emit_constant_boolean(false, IRValueKind::Temporary)
        } else {
            Ok(last_value)
        }
    }

    fn lower_call(&mut self, callee: &Expr, args: &[Expr]) -> Result<IRValueId, IRLoweringError> {
        if let ExprKind::MemberAccess { object, member } = &callee.kind {
            let obj_val = self.lower_expr(object)?;
            let obj_type = self.expr_types.get(&object.id).cloned();

            if let Some(NormalizedType::Named(type_name)) = obj_type {
                // Si el método está en la vtable, usar dispatch dinámico
                if let Some(method_idx) = self.get_vtable_index(&type_name, member) {
                    // 1. Cargar puntero a vtable desde offset 0 del objeto
                    let vtable_ptr = self.fresh_value();
                    self.emit(IRInstruction::new(IRInstructionKind::Load {
                        target: vtable_ptr.clone(),
                        address: IROperand::Value(obj_val.clone()),
                        ty: "ptr".to_string(),
                    }))?;

                    // 2. GEP al slot del método (element_type = ptr)
                    let slot_ptr = self.fresh_value();
                    self.emit(IRInstruction::new(IRInstructionKind::GetElementPtr {
                        target: slot_ptr.clone(),
                        base: IROperand::Value(vtable_ptr),
                        indices: vec![IROperand::Integer(method_idx as i64)],
                        element_type: "ptr".to_string(),
                    }))?;

                    // 3. Cargar el puntero a función
                    let method_ptr = self.fresh_value();
                    self.emit(IRInstruction::new(IRInstructionKind::Load {
                        target: method_ptr.clone(),
                        address: IROperand::Value(slot_ptr),
                        ty: "ptr".to_string(),
                    }))?;

                    // 4. Preparar argumentos (self primero)
                    let mut arg_values = vec![obj_val];
                    for arg in args {
                        arg_values.push(self.lower_expr(arg)?);
                    }

                    // 5. Obtener firma para el indirect call
                    let (ret, mut params) = self
                        .get_method_signature(&type_name, member)
                        .unwrap_or((None, vec![]));
                    let mut param_types = vec!["ptr".to_string()]; // self
                    param_types.append(&mut params);

                    let target = self.fresh_value();
                    self.emit(IRInstruction::new(IRInstructionKind::CallIndirect {
                        target: target.clone(),
                        callee_ptr: IROperand::Value(method_ptr),
                        arguments: arg_values.into_iter().map(IROperand::Value).collect(),
                        return_type: ret,
                        param_types,
                        original: Some(member.clone()),
                    }))?;
                    return Ok(target);
                }

                // Fallback a dispatch estático (built-ins, etc.)
                let owner = self
                    .find_method_owner(&type_name, member)
                    .unwrap_or_else(|| type_name.clone());
                let mangled_name = format!("{}_{}", owner, member);
                let mut arg_values = vec![obj_val];
                for arg in args {
                    arg_values.push(self.lower_expr(arg)?);
                }
                return self.emit_call_with_values(&mangled_name, arg_values);
            }

            // Fallback genérico
            let mut arg_values = vec![obj_val];
            for arg in args {
                arg_values.push(self.lower_expr(arg)?);
            }
            return self.emit_call_with_values(&format!("member.{}", member), arg_values);
        }

        // Caso especial: base(args) -> llamar al método del padre con el mismo nombre
        if let ExprKind::Base = &callee.kind {
            if let Some((type_name, method_name)) = &self.current_method
                && let Some(parent_name) = self.type_parents.get(type_name)
            {
                let mangled_name = format!("{}_{}", parent_name, method_name);
                let mut arg_values = vec![
                    self.lookup_variable("self")
                        .unwrap_or_else(|| self.fresh_value()),
                ];
                for arg in args {
                    arg_values.push(self.lower_expr(arg)?);
                }
                return self.emit_call_with_values(&mangled_name, arg_values);
            }
            return Err(IRLoweringError::new(
                "base() can only be used inside a method that overrides a parent method",
            ));
        }

        let callee_name = self.resolve_callee(callee)?;
        let arg_values = args
            .iter()
            .map(|a| self.lower_expr(a))
            .collect::<Result<Vec<_>, _>>()?;

        if callee_name == "print" {
            let arg_node = &args[0];

            // Use semantic annotation if available; otherwise fallback
            // to Unknown which maps to `print_object` at runtime.
            let arg_type = self
                .expr_types
                .get(&arg_node.id)
                .cloned()
                .unwrap_or(NormalizedType::Unknown);

            let runtime_print = match arg_type {
                NormalizedType::Number => "print_number",

                NormalizedType::String => "print_string",

                NormalizedType::Boolean => "print_bool",

                _ => "print_object",
            };

            return self.emit_call_with_values(runtime_print, arg_values);
        }

        self.emit_call_with_values(&callee_name, arg_values)
    }

    fn lower_assignment(
        &mut self,
        target: &Expr,
        value: &Expr,
        expr: &Expr,
    ) -> Result<IRValueId, IRLoweringError> {
        let value_id = self.lower_expr(value)?;

        match &target.kind {
            ExprKind::Identifier(name) => {
                let assigned = self
                    .lookup_variable(name)
                    .unwrap_or_else(|| self.fresh_value());
                self.emit(IRInstruction::new(IRInstructionKind::Assign {
                    target: assigned.clone(),
                    value: IROperand::Value(value_id),
                    original: Some(name.clone()),
                }))?;
                self.define_variable(name, assigned.clone());
                Ok(assigned)
            }
            ExprKind::MemberAccess { object, member } => {
                let object_value = self.lower_expr(object)?;

                // Clonar layout ANTES de mutar self
                let field_info: Option<(i64, String)> =
                    if let Some(NormalizedType::Named(type_name)) =
                        self.expr_types.get(&object.id).cloned()
                    {
                        self.type_layouts.get(&type_name).and_then(|fields| {
                            fields
                                .iter()
                                .enumerate()
                                .find(|(_, (name, _))| name == member)
                                .map(|(idx, (_, field_type))| {
                                    ((idx + 1) as i64, field_type.clone())
                                })
                        })
                    } else {
                        None
                    };

                if let Some((offset, field_type)) = field_info {
                    let gep_target = self.fresh_value();
                    self.emit(IRInstruction::new(IRInstructionKind::GetElementPtr {
                        target: gep_target.clone(),
                        base: IROperand::Value(object_value),
                        indices: vec![IROperand::Integer(offset)],
                        element_type: field_type.clone(),
                    }))?;
                    self.emit(IRInstruction::new(IRInstructionKind::Store {
                        address: IROperand::Value(gep_target),
                        value: IROperand::Value(value_id.clone()),
                    }))?;
                    // Devolver el valor asignado
                    Ok(value_id)
                } else {
                    Err(Self::error_at(
                        expr.span.clone(),
                        format!("Unknown field '{}' for assignment", member),
                    ))
                }
            }
            _ => Err(Self::error_at(
                expr.span.clone(),
                "Only identifier or member-access assignments are supported in IR lowering",
            )),
        }
    }

    fn lower_if(
        &mut self,
        condition: &Expr,
        then_expr: &Expr,
        elif_parts: &[(Expr, Expr)],
        else_expr: &Option<Box<Expr>>,
        expr: &Expr,
    ) -> Result<IRValueId, IRLoweringError> {
        if !elif_parts.is_empty() {
            let mut nested_else: Option<Expr> = else_expr.as_deref().cloned();
            for (elif_condition, elif_then) in elif_parts.iter().rev() {
                nested_else = Some(Expr::if_expr(
                    elif_condition.clone(),
                    elif_then.clone(),
                    Vec::new(),
                    nested_else,
                    expr.span.clone(),
                ));
            }

            let nested_else_boxed = nested_else.map(Box::new);
            return self.lower_if(condition, then_expr, &[], &nested_else_boxed, expr);
        }

        let condition_value = self.lower_expr(condition)?;
        let function_name = self.current_function_name()?.to_string();
        let current_block = self.current_block_id()?.clone();

        let then_name = self.fresh_block_name("if_then");
        let else_name = self.fresh_block_name("if_else");
        let merge_name = self.fresh_block_name("if_merge");

        self.create_block_in_function(&function_name, then_name.clone())?;
        self.create_block_in_function(&function_name, else_name.clone())?;
        self.create_block_in_function(&function_name, merge_name.clone())?;

        let then_block = BasicBlockId::new(then_name.clone());
        let else_block = BasicBlockId::new(else_name.clone());
        let merge_block = BasicBlockId::new(merge_name.clone());

        self.link_blocks(&function_name, &current_block, &then_block)?;
        self.link_blocks(&function_name, &current_block, &else_block)?;

        self.emit(IRInstruction::new(IRInstructionKind::Branch {
            condition: IROperand::Value(condition_value),
            then_block: then_block.clone(),
            else_block: else_block.clone(),
        }))?;

        let saved_scopes = self.scopes.clone();

        // THEN branch
        self.set_current_block(function_name.clone(), then_name)?;
        self.scopes = saved_scopes.clone();
        let then_value = self.lower_expr(then_expr)?;
        let then_exit = self.current_block_id()?.clone();
        let then_reaches = !self.current_block_terminated()?;
        if then_reaches {
            self.link_blocks(&function_name, &then_exit, &merge_block)?;
            self.emit(IRInstruction::new(IRInstructionKind::Jump {
                target: merge_block.clone(),
            }))?;
        }

        // ELSE branch
        self.set_current_block(function_name.clone(), else_name)?;
        self.scopes = saved_scopes;
        let else_value = match else_expr {
            Some(else_expr) => self.lower_expr(else_expr)?,
            None => self.emit_constant_boolean(false, IRValueKind::Temporary)?,
        };
        let else_exit = self.current_block_id()?.clone();
        let else_reaches = !self.current_block_terminated()?;
        if else_reaches {
            self.link_blocks(&function_name, &else_exit, &merge_block)?;
            self.emit(IRInstruction::new(IRInstructionKind::Jump {
                target: merge_block.clone(),
            }))?;
        }

        // MERGE: build phi only from branches that actually reach the merge block
        self.set_current_block(function_name, merge_name)?;
        let phi_target = self.fresh_value();
        let mut incoming = Vec::new();
        if then_reaches {
            incoming.push((then_value, then_exit));
        }
        if else_reaches {
            incoming.push((else_value, else_exit));
        }

        if incoming.len() >= 2 {
            self.emit(IRInstruction::new(IRInstructionKind::Phi {
                target: phi_target.clone(),
                incoming,
                original: None,
            }))?;
        } else if incoming.len() == 1 {
            self.emit(IRInstruction::new(IRInstructionKind::Assign {
                target: phi_target.clone(),
                value: IROperand::Value(incoming[0].0.clone()),
                original: None,
            }))?;
        } else {
            self.emit(IRInstruction::new(IRInstructionKind::Assign {
                target: phi_target.clone(),
                value: IROperand::Boolean(false),
                original: None,
            }))?;
        }

        Ok(phi_target)
    }
    fn lower_while(
        &mut self,
        condition: &Expr,
        body: &Expr,
        _expr: &Expr,
    ) -> Result<IRValueId, IRLoweringError> {
        let function_name = self.current_function_name()?.to_string();
        let current_block = self.current_block_id()?.clone();

        let cond_name = self.fresh_block_name("while_cond");
        let body_name = self.fresh_block_name("while_body");
        let exit_name = self.fresh_block_name("while_exit");

        self.create_block_in_function(&function_name, cond_name.clone())?;
        self.create_block_in_function(&function_name, body_name.clone())?;
        self.create_block_in_function(&function_name, exit_name.clone())?;

        let cond_block = BasicBlockId::new(cond_name.clone());
        let body_block = BasicBlockId::new(body_name.clone());
        let exit_block = BasicBlockId::new(exit_name.clone());

        self.link_blocks(&function_name, &current_block, &cond_block)?;
        self.emit(IRInstruction::new(IRInstructionKind::Jump {
            target: cond_block.clone(),
        }))?;

        self.set_current_block(function_name.clone(), cond_name)?;
        let condition_value = self.lower_expr(condition)?;
        self.link_blocks(&function_name, &cond_block, &body_block)?;
        self.link_blocks(&function_name, &cond_block, &exit_block)?;
        self.emit(IRInstruction::new(IRInstructionKind::Branch {
            condition: IROperand::Value(condition_value),
            then_block: body_block.clone(),
            else_block: exit_block.clone(),
        }))?;

        let loop_context = LoopContext {
            continue_block: cond_block.clone(),
            break_block: exit_block.clone(),
        };
        self.loop_stack.push(loop_context);

        self.set_current_block(function_name.clone(), body_name)?;
        self.push_scope();
        let _ = self.lower_expr(body)?;
        if !self.current_block_terminated()? {
            let body_current = self.current_block_id()?.clone();
            self.link_blocks(&function_name, &body_current, &cond_block)?;
            self.emit(IRInstruction::new(IRInstructionKind::Jump {
                target: cond_block,
            }))?;
        }
        // self.pop_scope();
        self.loop_stack.pop();

        self.set_current_block(function_name, exit_name)?;
        let result = self.emit_constant_boolean(false, IRValueKind::Temporary)?;
        Ok(result)
    }

    fn lower_for(
        &mut self,
        variable: &str,
        iterable: &Expr,
        body: &Expr,
        _expr: &Expr,
    ) -> Result<IRValueId, IRLoweringError> {
        let function_name = self.current_function_name()?.to_string();
        let current_block = self.current_block_id()?.clone();

        let iterable_value = self.lower_expr(iterable)?;
        let cond_name = self.fresh_block_name("for_cond");
        let body_name = self.fresh_block_name("for_body");
        let exit_name = self.fresh_block_name("for_exit");

        self.create_block_in_function(&function_name, cond_name.clone())?;
        self.create_block_in_function(&function_name, body_name.clone())?;
        self.create_block_in_function(&function_name, exit_name.clone())?;

        let cond_block = BasicBlockId::new(cond_name.clone());
        let body_block = BasicBlockId::new(body_name.clone());
        let exit_block = BasicBlockId::new(exit_name.clone());

        self.link_blocks(&function_name, &current_block, &cond_block)?;
        self.emit(IRInstruction::new(IRInstructionKind::Jump {
            target: cond_block.clone(),
        }))?;

        self.set_current_block(function_name.clone(), cond_name)?;
        let has_next = self.emit_call_with_values("iter_has_next", vec![iterable_value.clone()])?;
        self.link_blocks(&function_name, &cond_block, &body_block)?;
        self.link_blocks(&function_name, &cond_block, &exit_block)?;
        self.emit(IRInstruction::new(IRInstructionKind::Branch {
            condition: IROperand::Value(has_next),
            then_block: body_block.clone(),
            else_block: exit_block.clone(),
        }))?;

        self.loop_stack.push(LoopContext {
            continue_block: cond_block.clone(),
            break_block: exit_block.clone(),
        });

        self.set_current_block(function_name.clone(), body_name)?;
        self.push_scope();
        let next_item = self.emit_call_with_values("iter_next", vec![iterable_value])?;
        self.define_variable(variable, next_item);
        let _ = self.lower_expr(body)?;
        if !self.current_block_terminated()? {
            let body_current = self.current_block_id()?.clone();
            self.link_blocks(&function_name, &body_current, &cond_block)?;
            self.emit(IRInstruction::new(IRInstructionKind::Jump {
                target: cond_block,
            }))?;
        }
        // self.pop_scope();
        self.loop_stack.pop();

        self.set_current_block(function_name, exit_name)?;
        let result = self.emit_constant_boolean(false, IRValueKind::Temporary)?;
        Ok(result)
    }

    fn lower_let(
        &mut self,
        name: &str,
        annotation: &Option<TypeReference>,
        value: &Expr,
        body: &Expr,
        _expr: &Expr,
    ) -> Result<IRValueId, IRLoweringError> {
        let value_id = self.lower_expr(value)?;
        let target = self.fresh_value();
        let mut target_value = IRValue::new(target.0.clone(), IRValueKind::Temporary);

        target_value.ty = annotation.as_ref().map(|t| t.display_name());

        self.emit(IRInstruction::new(IRInstructionKind::Assign {
            target: target.clone(),

            value: IROperand::Value(value_id),

            original: None,
        }))?;

        self.scopes.push(HashMap::new());

        self.define_variable(name, target.clone());

        let body_result = self.lower_expr(body);

        self.scopes.pop();

        body_result
    }

    fn lower_member_access(
        &mut self,
        object: &Expr,
        member: &str,
        _expr: &Expr,
    ) -> Result<IRValueId, IRLoweringError> {
        let object_value = self.lower_expr(object)?;

        // Clonar los datos del layout ANTES de cualquier mutación de self
        let field_info: Option<(i64, String)> = if let Some(NormalizedType::Named(type_name)) =
            self.expr_types.get(&object.id).cloned()
        {
            self.type_layouts.get(&type_name).and_then(|fields| {
                fields
                    .iter()
                    .enumerate()
                    .find(|(_, (name, _))| name == member)
                    .map(|(idx, (_, field_type))| ((idx + 1) as i64, field_type.clone()))
            })
        } else {
            None
        };

        if let Some((offset, field_type)) = field_info {
            // GEP al campo
            let gep_target = self.fresh_value();
            self.emit(IRInstruction::new(IRInstructionKind::GetElementPtr {
                target: gep_target.clone(),
                base: IROperand::Value(object_value),
                indices: vec![IROperand::Integer(offset)],
                element_type: field_type.clone(),
            }))?;

            // Load del valor
            let load_target = self.fresh_value();
            self.emit(IRInstruction::new(IRInstructionKind::Load {
                target: load_target.clone(),
                address: IROperand::Value(gep_target),
                ty: field_type,
            }))?;

            return Ok(load_target);
        }

        // Fallback: llamada mágica
        self.emit_call_with_values(&format!("member.{}", member), vec![object_value])
    }
    fn lower_index_access(
        &mut self,
        object: &Expr,
        index: &Expr,
        _expr: &Expr,
    ) -> Result<IRValueId, IRLoweringError> {
        let object_value = self.lower_expr(object)?;
        let index_value = self.lower_expr(index)?;
        self.emit_call_with_values("index", vec![object_value, index_value])
    }

    fn lower_type_check(
        &mut self,
        inner: &Expr,
        type_ref: &TypeReference,
        _expr: &Expr,
    ) -> Result<IRValueId, IRLoweringError> {
        let value = self.lower_expr(inner)?;
        self.emit_call_with_values(&format!("is_{}", type_ref.display_name()), vec![value])
    }

    fn lower_type_cast(
        &mut self,
        inner: &Expr,
        type_ref: &TypeReference,
        _expr: &Expr,
    ) -> Result<IRValueId, IRLoweringError> {
        let value = self.lower_expr(inner)?;
        self.emit_call_with_values(&format!("as_{}", type_ref.display_name()), vec![value])
    }

    fn lower_new(
        &mut self,
        type_ref: &TypeReference,
        arguments: &[Expr],
        _expr: &Expr,
    ) -> Result<IRValueId, IRLoweringError> {
        let argument_values = arguments
            .iter()
            .map(|argument| self.lower_expr(argument))
            .collect::<Result<Vec<_>, _>>()?;
        self.emit_call_with_values(&format!("new_{}", type_ref.display_name()), argument_values)
    }

    fn lower_self(&mut self, expr: &Expr) -> Result<IRValueId, IRLoweringError> {
        self.lookup_variable("self").ok_or_else(|| {
            Self::error_at(expr.span.clone(), "'self' is not available in this scope")
        })
    }

    fn lower_base(&mut self, _expr: &Expr) -> Result<IRValueId, IRLoweringError> {
        let base_name = "base".to_string();
        self.emit_call_with_values(&base_name, Vec::new())
    }

    fn lower_vector_literal(
        &mut self,
        elements: &[Expr],
        _expr: &Expr,
    ) -> Result<IRValueId, IRLoweringError> {
        let element_values = elements
            .iter()
            .map(|element| self.lower_expr(element))
            .collect::<Result<Vec<_>, _>>()?;
        self.emit_call_with_values("vector_literal", element_values)
    }

    fn lower_vector_comprehension(
        &mut self,
        element_expr: &Expr,
        binding: &str,
        iterable: &Expr,
        _expr: &Expr,
    ) -> Result<IRValueId, IRLoweringError> {
        let iterable_value = self.lower_expr(iterable)?;
        let element_value = self.lower_expr(element_expr)?;
        self.emit_call_with_values("vector_comprehension", vec![iterable_value, element_value])?;

        let target = self.fresh_value();
        self.define_variable(binding, target.clone());
        Ok(target)
    }

    fn resolve_callee(&mut self, callee: &Expr) -> Result<String, IRLoweringError> {
        match &callee.kind {
            ExprKind::Identifier(name) => Ok(name.clone()),
            _ => Err(IRLoweringError::new(
                "Only identifier and member-access callees are supported",
            )),
        }
    }

    fn emit_call_with_values(
        &mut self,
        callee: &str,
        arguments: Vec<IRValueId>,
    ) -> Result<IRValueId, IRLoweringError> {
        let target = self.fresh_value();
        self.emit(IRInstruction::new(IRInstructionKind::Call {
            target: target.clone(),
            callee: callee.to_string(),
            arguments: arguments.into_iter().map(IROperand::Value).collect(),
            original: None,
        }))?;
        Ok(target)
    }

    fn emit_constant(
        &mut self,
        operand: IROperand,
        kind: Option<IRValueKind>,
    ) -> Result<IRValueId, IRLoweringError> {
        let target = self.fresh_value();
        let _value_kind = kind.unwrap_or(IRValueKind::Temporary);
        self.emit(IRInstruction::new(IRInstructionKind::Assign {
            target: target.clone(),
            value: operand,
            original: None,
        }))?;
        Ok(target)
    }

    fn emit_constant_boolean(
        &mut self,
        value: bool,
        kind: IRValueKind,
    ) -> Result<IRValueId, IRLoweringError> {
        self.emit_constant(IROperand::Boolean(value), Some(kind))
    }

    fn fresh_value(&mut self) -> IRValueId {
        IRValueId::new(self.value_generator.next_value())
    }

    fn fresh_block_name(&mut self, prefix: &str) -> String {
        let name = format!("{}_{}", prefix, self.block_counter);
        self.block_counter += 1;
        name
    }

    fn current_function_name(&self) -> Result<&str, IRLoweringError> {
        self.current_function
            .as_deref()
            .ok_or_else(|| IRLoweringError::new("No current function is active"))
    }

    fn current_block_id(&self) -> Result<&BasicBlockId, IRLoweringError> {
        self.current_block
            .as_ref()
            .ok_or_else(|| IRLoweringError::new("No current block is active"))
    }

    fn set_current_block(
        &mut self,
        function_name: impl AsRef<str>,
        block_name: impl AsRef<str>,
    ) -> Result<(), IRLoweringError> {
        let function_name = function_name.as_ref();
        let block_name = block_name.as_ref();

        let block_id = BasicBlockId::new(block_name);
        let function = self.module.function(function_name).ok_or_else(|| {
            IRLoweringError::new(format!("Function '{}' not found", function_name))
        })?;

        if function.block(&block_id).is_none() {
            return Err(IRLoweringError::new(format!(
                "Block '{}' not found in function '{}'",
                block_name, function_name
            )));
        }

        self.current_function = Some(function_name.to_string());
        self.current_block = Some(block_id);
        Ok(())
    }

    fn emit(&mut self, instruction: IRInstruction) -> Result<(), IRLoweringError> {
        let function_name = self.current_function_name()?.to_string();
        let block_name = self.current_block_id()?.0.clone();
        self.add_instruction_to_block(function_name, block_name, instruction)
    }

    fn link_blocks(
        &mut self,
        function_name: &str,
        from: &BasicBlockId,
        to: &BasicBlockId,
    ) -> Result<(), IRLoweringError> {
        let function = self.module.function_mut(function_name).ok_or_else(|| {
            IRLoweringError::new(format!("Function '{}' not found", function_name))
        })?;

        let from_block = function
            .block_mut(from)
            .ok_or_else(|| IRLoweringError::new(format!("Block '{}' not found", from.0)))?;
        from_block.add_successor(to.clone());

        let to_block = function
            .block_mut(to)
            .ok_or_else(|| IRLoweringError::new(format!("Block '{}' not found", to.0)))?;
        to_block.add_predecessor(from.clone());
        Ok(())
    }

    fn block_terminated(
        &self,
        function_name: &str,
        block_id: &BasicBlockId,
    ) -> Result<bool, IRLoweringError> {
        let function = self.module.function(function_name).ok_or_else(|| {
            IRLoweringError::new(format!("Function '{}' not found", function_name))
        })?;
        let block = function.block(block_id).ok_or_else(|| {
            IRLoweringError::new(format!("Block '{}' not found", block_id.0.clone()))
        })?;

        Ok(block
            .instructions
            .last()
            .map(|instr| instr.is_terminator())
            .unwrap_or(false))
    }

    fn current_block_terminated(&self) -> Result<bool, IRLoweringError> {
        let function_name = self.current_function_name()?;
        let block_id = self.current_block_id()?;
        self.block_terminated(function_name, block_id)
    }

    fn push_scope(&mut self) {
        let next_scope = self.scopes.last().cloned().unwrap_or_default();
        self.scopes.push(next_scope);
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn define_variable(&mut self, name: &str, value: IRValueId) {
        if self.scopes.is_empty() {
            self.scopes.push(HashMap::new());
        }

        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), value);
        }
    }

    fn lookup_variable(&self, name: &str) -> Option<IRValueId> {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Some(value.clone());
            }
        }
        None
    }

    fn error_at(span: Span, message: impl Into<String>) -> IRLoweringError {
        IRLoweringError::at(message.into(), span_location(&span))
    }
}

impl IRLoweringContext for IRBuilder {
    fn lower_program(&mut self, program: &Program) -> Result<IRModule, IRLoweringError> {
        IRBuilder::lower_program(self, program)
    }
}

fn span_location(span: &Span) -> String {
    format!(
        "{}:{}:{}-{}:{}",
        span.file, span.start_line, span.start_column, span.end_line, span.end_column
    )
}
