// Persona 3 — AST lowering into IR
// Assigned: Persona3
// Responsibilities: implementar la pasarela que convierte `Program` (AST)
// en `IRModule`. Maneja expresiones, variables, control flow y conecta con el
// builder sin exponer detalles internos.

use std::collections::HashMap;

use crate::parser::ast::{
    BinaryOperator, DeclarationKind, Expr, ExprKind, FunctionDeclaration, Literal, Parameter,
    Program, TypeReference, UnaryOperator, VariableDeclaration,
};
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
    value_generator: SSAValueGenerator,
    block_counter: usize,
    scopes: Vec<HashMap<String, IRValueId>>,
    loop_stack: Vec<LoopContext>,
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
        }
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

    pub fn lower_program(&mut self, program: &Program) -> Result<IRModule, IRLoweringError> {
        self.reset();
        self.module = IRModule::new(IRNaming::module_name("lowered"));

        for declaration in &program.declarations {
            if let DeclarationKind::Function(function) = &declaration.kind {
                self.lower_function_declaration(function)?;
            }
        }

        let has_entry_content = program.entry_expression.is_some()
            || program
                .declarations
                .iter()
                .any(|declaration| matches!(declaration.kind, DeclarationKind::Variable(_)));

        if has_entry_content {
            self.lower_entry_function(program)?;
        }

        Ok(self.module.clone())
    }

    fn reset(&mut self) {
        self.current_function = None;
        self.current_block = None;
        self.value_generator.reset();
        self.block_counter = 0;
        self.scopes.clear();
        self.loop_stack.clear();
    }

    fn lower_entry_function(&mut self, program: &Program) -> Result<(), IRLoweringError> {
        let entry_name = "__entry".to_string();

        self.create_function(entry_name.clone(), Vec::new(), None)?;
        self.create_block_in_function(&entry_name, "entry")?;
        self.set_current_block(entry_name.clone(), "entry")?;
        self.push_scope();

        for declaration in &program.declarations {
            if let DeclarationKind::Variable(variable) = &declaration.kind {
                self.lower_variable_declaration(variable)?;
            }
        }

        match &program.entry_expression {
            Some(entry_expression) => {
                let result = self.lower_expr(entry_expression)?;
                if !self.current_block_terminated()? {
                    self.emit(IRInstruction::new(IRInstructionKind::Return(Some(
                        IROperand::Value(result),
                    ))))?;
                }
            }
            None => {
                if !self.current_block_terminated()? {
                    let result = self.emit_constant_boolean(false, IRValueKind::Temporary)?;
                    self.emit(IRInstruction::new(IRInstructionKind::Return(Some(
                        IROperand::Value(result),
                    ))))?;
                }
            }
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

        self.create_function(
            function.name.clone(),
            parameters,
            function
                .return_type
                .as_ref()
                .map(|type_ref| type_ref.display_name()),
        )?;

        self.create_block_in_function(&function.name, "entry")?;
        self.set_current_block(function.name.clone(), "entry")?;
        self.push_scope();

        for parameter in &function.parameters {
            let value_id = IRValueId::new(parameter.name.clone());
            self.define_variable(&parameter.name, value_id);
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

    fn lower_variable_declaration(
        &mut self,
        variable: &VariableDeclaration,
    ) -> Result<IRValueId, IRLoweringError> {
        let value_id = match &variable.value {
            Some(initializer) => self.lower_expr(initializer)?,
            None => self.emit_constant_boolean(false, IRValueKind::Temporary)?,
        };

        let target = self.fresh_value();
        self.emit(IRInstruction::new(IRInstructionKind::Assign {
            target: target.clone(),
            value: IROperand::Value(value_id.clone()),
            original: None,
        }))?;
        self.define_variable(&variable.name, target.clone());
        Ok(target)
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
            ExprKind::Grouping(inner) => self.lower_expr(inner),
            ExprKind::Block(expressions) => self.lower_block(expressions, expr),
            ExprKind::Call { callee, arguments } => self.lower_call(callee, arguments, expr),
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
            } => self.lower_let(name, annotation, value.as_deref(), expr),
            ExprKind::Return(value) => self.lower_return(value.as_deref(), expr),
            ExprKind::Break => self.lower_break(expr),
            ExprKind::Continue => self.lower_continue(expr),
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
            ExprKind::Base { member } => self.lower_base(member.as_deref(), expr),
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
            UnaryOperator::Plus => Ok(operand_value),
            UnaryOperator::Minus | UnaryOperator::Not => {
                let target = self.fresh_value();
                let op = match operator {
                    UnaryOperator::Minus => IRUnaryOp::Neg,
                    UnaryOperator::Not => IRUnaryOp::Not,
                    UnaryOperator::Plus => unreachable!(),
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
            BinaryOperator::Add => Some(IRInstructionKind::Binary {
                target: target.clone(),
                op: IRBinaryOp::Add,
                left: left_operand.clone(),
                right: right_operand.clone(),
            }),
            BinaryOperator::Subtract => Some(IRInstructionKind::Binary {
                target: target.clone(),
                op: IRBinaryOp::Sub,
                left: left_operand.clone(),
                right: right_operand.clone(),
            }),
            BinaryOperator::Multiply => Some(IRInstructionKind::Binary {
                target: target.clone(),
                op: IRBinaryOp::Mul,
                left: left_operand.clone(),
                right: right_operand.clone(),
            }),
            BinaryOperator::Divide => Some(IRInstructionKind::Binary {
                target: target.clone(),
                op: IRBinaryOp::Div,
                left: left_operand.clone(),
                right: right_operand.clone(),
            }),
            BinaryOperator::And => Some(IRInstructionKind::Binary {
                target: target.clone(),
                op: IRBinaryOp::And,
                left: left_operand.clone(),
                right: right_operand.clone(),
            }),
            BinaryOperator::Or => Some(IRInstructionKind::Binary {
                target: target.clone(),
                op: IRBinaryOp::Or,
                left: left_operand.clone(),
                right: right_operand.clone(),
            }),
            BinaryOperator::Equal => Some(IRInstructionKind::Binary {
                target: target.clone(),
                op: IRBinaryOp::Eq,
                left: left_operand.clone(),
                right: right_operand.clone(),
            }),
            BinaryOperator::NotEqual => Some(IRInstructionKind::Binary {
                target: target.clone(),
                op: IRBinaryOp::Ne,
                left: left_operand.clone(),
                right: right_operand.clone(),
            }),
            BinaryOperator::Less => Some(IRInstructionKind::Binary {
                target: target.clone(),
                op: IRBinaryOp::Lt,
                left: left_operand.clone(),
                right: right_operand.clone(),
            }),
            BinaryOperator::LessEqual => Some(IRInstructionKind::Binary {
                target: target.clone(),
                op: IRBinaryOp::Le,
                left: left_operand.clone(),
                right: right_operand.clone(),
            }),
            BinaryOperator::Greater => Some(IRInstructionKind::Binary {
                target: target.clone(),
                op: IRBinaryOp::Gt,
                left: left_operand.clone(),
                right: right_operand.clone(),
            }),
            BinaryOperator::GreaterEqual => Some(IRInstructionKind::Binary {
                target: target.clone(),
                op: IRBinaryOp::Ge,
                left: left_operand.clone(),
                right: right_operand.clone(),
            }),
            BinaryOperator::Power => None,
            BinaryOperator::Modulo => None,
            BinaryOperator::Concat | BinaryOperator::Concatenate => None,
        };

        match instruction {
            Some(instruction) => {
                self.emit(IRInstruction::new(instruction))?;
                Ok(target)
            }
            None => {
                let callee = match operator {
                    BinaryOperator::Power => "pow",
                    BinaryOperator::Modulo => "mod",
                    BinaryOperator::Concat | BinaryOperator::Concatenate => "concat",
                    _ => unreachable!(),
                };
                self.emit_call_with_values(callee, vec![left_value, right_value])
            }
        }
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

    fn lower_call(
        &mut self,
        callee: &Expr,
        arguments: &[Expr],
        _expr: &Expr,
    ) -> Result<IRValueId, IRLoweringError> {
        let callee_name = self.resolve_callee(callee)?;
        let argument_values = arguments
            .iter()
            .map(|argument| self.lower_expr(argument))
            .collect::<Result<Vec<_>, _>>()?;

        self.emit_call_with_values(&callee_name, argument_values)
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
                let assigned = self.fresh_value();
                self.emit(IRInstruction::new(IRInstructionKind::Assign {
                    target: assigned.clone(),
                    value: IROperand::Value(value_id),
                    original: Some(name.clone()),
                }))?;
                self.define_variable(name, assigned.clone());
                Ok(assigned)
            }
            _ => Err(Self::error_at(
                expr.span.clone(),
                "Only identifier assignments are supported in IR lowering",
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

            if let Some(Expr {
                kind:
                    ExprKind::If {
                        condition,
                        then_expr,
                        elif_parts,
                        else_expr,
                    },
                ..
            }) = nested_else
            {
                return self.lower_if(&condition, &then_expr, &elif_parts, &else_expr, expr);
            }
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

        self.set_current_block(function_name.clone(), then_name)?;
        self.scopes = saved_scopes.clone();
        let then_value = self.lower_expr(then_expr)?;
        if !self.current_block_terminated()? {
            self.emit(IRInstruction::new(IRInstructionKind::Jump {
                target: merge_block.clone(),
            }))?;
        }

        self.set_current_block(function_name.clone(), else_name)?;
        self.scopes = saved_scopes;
        let else_value = match else_expr {
            Some(else_expr) => self.lower_expr(else_expr)?,
            None => self.emit_constant_boolean(false, IRValueKind::Temporary)?,
        };
        if !self.current_block_terminated()? {
            self.emit(IRInstruction::new(IRInstructionKind::Jump {
                target: merge_block.clone(),
            }))?;
        }

        self.set_current_block(function_name, merge_name)?;
        let phi_target = self.fresh_value();
        self.emit(IRInstruction::new(IRInstructionKind::Phi {
            target: phi_target.clone(),
            incoming: vec![(then_value, then_block), (else_value, else_block)],
            original: None,
        }))?;

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
            self.emit(IRInstruction::new(IRInstructionKind::Jump {
                target: cond_block,
            }))?;
        }
        self.pop_scope();
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
            self.emit(IRInstruction::new(IRInstructionKind::Jump {
                target: cond_block,
            }))?;
        }
        self.pop_scope();
        self.loop_stack.pop();

        self.set_current_block(function_name, exit_name)?;
        let result = self.emit_constant_boolean(false, IRValueKind::Temporary)?;
        Ok(result)
    }

    fn lower_let(
        &mut self,
        name: &str,
        annotation: &Option<TypeReference>,
        value: Option<&Expr>,
        _expr: &Expr,
    ) -> Result<IRValueId, IRLoweringError> {
        let value_id = match value {
            Some(inner) => self.lower_expr(inner)?,
            None => self.emit_constant_boolean(false, IRValueKind::Temporary)?,
        };

        let target = self.fresh_value();
        let mut target_value = IRValue::new(target.0.clone(), IRValueKind::Temporary);
        target_value.ty = annotation.as_ref().map(|type_ref| type_ref.display_name());
        self.emit(IRInstruction::new(IRInstructionKind::Assign {
            target: target.clone(),
            value: IROperand::Value(value_id),
            original: None,
        }))?;
        self.define_variable(name, target.clone());

        Ok(target)
    }

    fn lower_return(
        &mut self,
        value: Option<&Expr>,
        _expr: &Expr,
    ) -> Result<IRValueId, IRLoweringError> {
        let operand = value
            .as_ref()
            .map(|inner| self.lower_expr(inner))
            .transpose()?;

        self.emit(IRInstruction::new(IRInstructionKind::Return(
            operand.clone().map(IROperand::Value),
        )))?;
        Ok(operand.unwrap_or_else(|| self.fresh_value()))
    }

    fn lower_break(&mut self, expr: &Expr) -> Result<IRValueId, IRLoweringError> {
        let loop_context =
            self.loop_stack.last().cloned().ok_or_else(|| {
                Self::error_at(expr.span.clone(), "'break' used outside of a loop")
            })?;

        self.emit(IRInstruction::new(IRInstructionKind::Jump {
            target: loop_context.break_block,
        }))?;
        self.emit_constant_boolean(false, IRValueKind::Temporary)
    }

    fn lower_continue(&mut self, expr: &Expr) -> Result<IRValueId, IRLoweringError> {
        let loop_context = self.loop_stack.last().cloned().ok_or_else(|| {
            Self::error_at(expr.span.clone(), "'continue' used outside of a loop")
        })?;

        self.emit(IRInstruction::new(IRInstructionKind::Jump {
            target: loop_context.continue_block,
        }))?;
        self.emit_constant_boolean(false, IRValueKind::Temporary)
    }

    fn lower_member_access(
        &mut self,
        object: &Expr,
        member: &str,
        _expr: &Expr,
    ) -> Result<IRValueId, IRLoweringError> {
        let object_value = self.lower_expr(object)?;
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
        self.emit_call_with_values(&format!("new {}", type_ref.display_name()), argument_values)
    }

    fn lower_self(&mut self, expr: &Expr) -> Result<IRValueId, IRLoweringError> {
        self.lookup_variable("self").ok_or_else(|| {
            Self::error_at(expr.span.clone(), "'self' is not available in this scope")
        })
    }

    fn lower_base(
        &mut self,
        member: Option<&str>,
        _expr: &Expr,
    ) -> Result<IRValueId, IRLoweringError> {
        let base_name = member
            .map(|name| format!("base.{}", name))
            .unwrap_or_else(|| "base".to_string());
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
            ExprKind::MemberAccess { object, member } => {
                let object_value = self.lower_expr(object)?;
                Ok(format!("{}.{}", object_value.0, member))
            }
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
            target: Some(target.clone()),
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
