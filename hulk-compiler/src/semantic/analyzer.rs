// =============================================================================
// Semantic Analyzer (Analizador Semántico Principal)
// =============================================================================
//
// Responsabilidad del Líder:
// - Orquestar la ejecución de los análisis semánticos
// - Coordinar Symbol Table, Type System, Expression Checker
// - Verificar declaraciones (funciones, tipos, protocolos)
// - Verificar expresión de entrada
// - Recopilar y reportar errores
// - Mantener contexto semántico global
//
// Esta es la interfaz pública del módulo semántico.
//
// =============================================================================

use crate::parser::ast::Program;
use crate::parser::ast::{
    DeclarationKind, Expr, ExprKind, FunctionDeclaration, TypeMember, TypeReferenceKind,
};
use crate::semantic::expression_checker::ExpressionChecker;
use crate::semantic::symbol_table::{SymbolInfo, SymbolTable};
use crate::semantic::type_system::NormalizedType;
use crate::semantic::type_system::TypeEnvironment;
use crate::utils::errors::semantic::SemanticError;

type SemanticResult<T> = Result<T, SemanticError>;

/// Contexto semántico global
///
/// Contiene toda la información acumulada durante el análisis semántico:
/// - Tabla de símbolos con scopes
/// - Entorno de tipos (tipos, protocolos)
/// - Errores encontrados
pub struct SemanticContext {
    /// Tabla de símbolos con gestión de scopes
    pub symbols: SymbolTable,
    /// Entorno de tipos (tipos definidos, protocolos)
    pub types: TypeEnvironment,
    /// Checker de tipos de expresiones
    pub expression_checker: ExpressionChecker,
    /// Errores semánticos encontrados
    pub errors: Vec<SemanticError>,
}

impl SemanticContext {
    pub fn new() -> Self {
        Self {
            symbols: SymbolTable::new(),
            types: TypeEnvironment::new(),
            expression_checker: ExpressionChecker::new(),
            errors: Vec::new(),
        }
    }

    /// Registra un error sin detener el análisis
    pub fn push_error(&mut self, error: SemanticError) {
        self.errors.push(error);
    }

    /// Retorna true si hay errores
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Retorna todos los errores encontrados
    pub fn get_errors(&self) -> &[SemanticError] {
        &self.errors
    }

    /// Limpia la lista de errores
    pub fn clear_errors(&mut self) {
        self.errors.clear();
    }
}

impl Default for SemanticContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Analizador semántico principal
///
/// Orquesta toda la fase semántica:
/// 1. Verificación de declaraciones (funciones, tipos, protocolos)
/// 2. Verificación de expresión de entrada
/// 3. Recopilación de errores
pub struct SemanticAnalyzer {
    context: SemanticContext,
    /// During analysis of a function body we may collect observed call signatures
    /// for parameters/variables without annotations. This map stores for the
    /// current function (if any) a mapping from identifier -> list of observed
    /// argument type vectors (each call's argument types).
    current_inferred_signatures:
        Option<std::collections::HashMap<String, Vec<Vec<NormalizedType>>>>,
}

impl SemanticAnalyzer {
    /// Crea un nuevo analizador semántico
    pub fn new() -> Self {
        let mut context = SemanticContext::new();
        // TODO: Declarar builtins (print, sqrt, sin, cos, log, exp, rand)
        context.symbols.declare_builtins();
        Self {
            context,
            current_inferred_signatures: None,
        }
    }

    /// Retorna una referencia al contexto
    pub fn context(&self) -> &SemanticContext {
        &self.context
    }

    /// Retorna una referencia mutable al contexto
    pub fn context_mut(&mut self) -> &mut SemanticContext {
        &mut self.context
    }

    /// Analiza un programa completo
    ///
    /// Pasos:
    /// 1. Procesar todas las declaraciones (functions, types, protocols)
    /// 2. Procesar expresión de entrada (si existe)
    /// 3. Retornar lista de errores (si las hay)
    pub fn analyze(&mut self, program: &Program) -> SemanticResult<()> {
        self.check_declarations(program)?;
        self.check_entry_expression(program)?;

        if let Some(error) = self.context.errors.first() {
            return Err(error.clone());
        }

        Ok(())
    }

    /// Verifica las declaraciones del programa
    ///
    /// Procesa en orden:
    /// 1. Todas las declaraciones de tipos
    /// 2. Todas las declaraciones de protocolos
    /// 3. Todas las declaraciones de funciones
    fn check_declarations(&mut self, _program: &Program) -> SemanticResult<()> {
        // First pass: register types and protocols so functions can reference them
        for decl in &_program.declarations {
            match &decl.kind {
                DeclarationKind::Type(td) => {
                    // Build TypeInfo
                    let parent = match &td.inherits {
                        Some(tr) => match &tr.kind {
                            TypeReferenceKind::Named(name) => Some(name.clone()),
                            _ => {
                                self.report_error(SemanticError::UnsupportedFeature {
                                    feature: "parametrized inherits".into(),
                                });
                                None
                            }
                        },
                        None => None,
                    };

                    let mut methods: Vec<FunctionDeclaration> = Vec::new();
                    let mut properties: Vec<(String, crate::parser::ast::TypeReference)> =
                        Vec::new();
                    for member in &td.members {
                        match member {
                            TypeMember::Method(f) => methods.push(f.clone()),
                            TypeMember::Attribute(a) => {
                                if let Some(ann) = &a.annotation {
                                    properties.push((a.name.clone(), ann.clone()));
                                }
                            }
                        }
                    }

                    let type_info = crate::semantic::type_system::TypeInfo {
                        name: td.name.clone(),
                        parameters: td.parameters.clone(),
                        parent,
                        methods,
                        properties,
                        implemented_protocols: vec![],
                        span: crate::utils::errors::span::Span::default(),
                    };

                    if let Err(e) = self.context.types.register_type(type_info) {
                        self.report_error(e);
                    }

                    // declare type in symbol table
                    let sym = crate::semantic::symbol_table::SymbolInfo::Type {
                        name: td.name.clone(),
                        span: crate::utils::errors::span::Span::default(),
                    };
                    if let Err(e) = self.context.symbols.declare(sym) {
                        self.report_error(e);
                    }
                }
                DeclarationKind::Protocol(pd) => {
                    // Build ProtocolInfo
                    let mut extends: Vec<String> = Vec::new();
                    for ext in &pd.extends {
                        match &ext.kind {
                            TypeReferenceKind::Named(name) => extends.push(name.clone()),
                            _ => {
                                self.report_error(SemanticError::UnsupportedFeature {
                                    feature: "parametrized protocol extends".into(),
                                });
                            }
                        }
                    }

                    let proto_info = crate::semantic::type_system::ProtocolInfo {
                        name: pd.name.clone(),
                        members: pd.members.clone(),
                        extends,
                        span: crate::utils::errors::span::Span::default(),
                    };

                    if let Err(e) = self.context.types.register_protocol(proto_info) {
                        self.report_error(e);
                    }

                    // declare protocol symbol
                    let sym = crate::semantic::symbol_table::SymbolInfo::Protocol {
                        name: pd.name.clone(),
                        span: crate::utils::errors::span::Span::default(),
                    };
                    if let Err(e) = self.context.symbols.declare(sym) {
                        self.report_error(e);
                    }
                }
                _ => {}
            }
        }

        // Second pass: register functions and validate signatures
        for decl in &_program.declarations {
            if let DeclarationKind::Function(func) = &decl.kind {
                // Validate function signature types
                if let Err(e) = self.context.types.validate_function_signature(func) {
                    self.report_error(e);
                }
                let normalized_return = match &func.return_type {
                    Some(type_ref) => self.context.types.validate_type_reference(type_ref)?,
                    None => NormalizedType::Unknown,
                };

                // Declare function in symbol table
                let sym = crate::semantic::symbol_table::SymbolInfo::Function {
                    name: func.name.clone(),
                    parameters: func.parameters.clone(),
                    return_type: normalized_return,
                    span: crate::utils::errors::span::Span::default(),
                };

                if let Err(e) = self.context.symbols.declare(sym) {
                    self.report_error(e);
                }
            }
        }

        // Third pass: analyze function bodies with parameter scopes so that
        // parameters annotated with types/protocols are available during body analysis.
        for decl in &_program.declarations {
            if let DeclarationKind::Function(func) = &decl.kind {
                self.context.symbols.enter_scope();

                // declare parameters as Parameter symbols with their annotations
                for p in &func.parameters {
                    let normalized_type = match &p.annotation {
                        Some(annotation) => {
                            self.context.types.validate_type_reference(annotation)?
                        }
                        None => NormalizedType::Unknown,
                    };
                    let param_sym = crate::semantic::symbol_table::SymbolInfo::Parameter {
                        name: p.name.clone(),
                        type_ref: normalized_type,
                        span: p.span.clone(),
                    };
                    if let Err(e) = self.context.symbols.declare(param_sym) {
                        self.report_error(e);
                    }
                }

                // prepare inference map for this function body
                self.current_inferred_signatures = Some(std::collections::HashMap::new());

                // analyze function body (during this call we'll collect observed
                // call argument types for unannotated parameters)
                let body_t = match self.analyze_expr(&func.body) {
                    Ok(t) => t,
                    Err(e) => {
                        self.report_error(e.clone());
                        NormalizedType::Unknown
                    }
                };

                // clear inference state
                self.current_inferred_signatures = None;

                // if function has annotated return type, validate compatibility
                if let Some(ret_ann) = &func.return_type {
                    match self.context.types.validate_type_reference(ret_ann) {
                        Ok(expected_t) => {
                            if body_t != NormalizedType::Unknown
                                && !self.context.types.is_compatible(&body_t, &expected_t)
                            {
                                self.report_error(SemanticError::ReturnTypeMismatch {
                                    function: func.name.clone(),
                                    expected: expected_t.to_string(),
                                    found: body_t.to_string(),
                                });
                            }
                        }
                        Err(e) => self.report_error(e),
                    }
                }

                self.context.symbols.exit_scope();
            }
        }

        Ok(())
    }

    /// Verifica la expresión de entrada del programa
    fn check_entry_expression(&mut self, program: &Program) -> SemanticResult<()> {
        if let Some(expr) = &program.entry_expression {
            if let Err(e) = self.analyze_expr(expr) {
                self.report_error(e);
            }
        }

        Ok(())
    }

    /// Basic recursive expression analyzer that uses `ExpressionChecker` and
    /// available symbol/type information to validate common constructs.
    fn analyze_expr(&mut self, expr: &Expr) -> Result<NormalizedType, SemanticError> {
        match &expr.kind {
            ExprKind::Literal(literal) => {
                let et = self.context.expression_checker.check_literal(literal)?;

                Ok(et.type_)
            }
            ExprKind::Identifier(name) => match self.context.symbols.lookup(name) {
                Some(SymbolInfo::Variable { type_ref, .. })
                | Some(SymbolInfo::Parameter { type_ref, .. }) => Ok(type_ref.clone()),

                Some(SymbolInfo::Function { .. }) => Ok(NormalizedType::Unknown),

                Some(_) => Ok(NormalizedType::Unknown),

                None => {
                    self.report_error(SemanticError::UndeclaredVariable {
                        name: name.clone(),
                        span: expr.span.clone(),
                    });

                    Ok(NormalizedType::Unknown)
                }
            },
            ExprKind::Call { callee, arguments } => {
                if let ExprKind::Identifier(name) = &callee.kind {
                    // 1. Obtener tipos de los argumentos
                    let mut arg_types = Vec::new();

                    for arg in arguments {
                        arg_types.push(self.analyze_expr(arg)?);
                    }

                    // 2. Construir la firma esperada
                    let mut expected_params: Option<Vec<(String, NormalizedType)>> = None;
                    let mut expected_return: Option<NormalizedType> = None;

                    match self.context.symbols.lookup(name) {
                        // Función normal
                        Some(SymbolInfo::Function {
                            parameters,
                            return_type,
                            ..
                        }) => {
                            let params = parameters
                                .iter()
                                .map(|p| {
                                    let ty = p
                                        .annotation
                                        .as_ref()
                                        .map(|ann| {
                                            self.context
                                                .types
                                                .validate_type_reference(ann)
                                                .unwrap_or(NormalizedType::Unknown)
                                        })
                                        .unwrap_or(NormalizedType::Unknown);

                                    (p.name.clone(), ty)
                                })
                                .collect();

                            expected_params = Some(params);

                            expected_return = Some(return_type.clone());
                        }

                        // Variables o parámetros que implementan invoke
                        Some(SymbolInfo::Variable { type_ref, .. })
                        | Some(SymbolInfo::Parameter { type_ref, .. }) => {
                            if let NormalizedType::Named(type_name) = &type_ref {
                                if let Some(proto) = self.context.types.get_protocol(type_name)
                                    && let Some(invoke_sig) =
                                        proto.members.iter().find(|m| m.name == "invoke")
                                {
                                    let params = invoke_sig
                                        .parameters
                                        .iter()
                                        .map(|p| {
                                            let ty = p
                                                .annotation
                                                .as_ref()
                                                .map(|ann| {
                                                    self.context
                                                        .types
                                                        .validate_type_reference(ann)
                                                        .unwrap_or(NormalizedType::Unknown)
                                                })
                                                .unwrap_or(NormalizedType::Unknown);

                                            (p.name.clone(), ty)
                                        })
                                        .collect();

                                    expected_params = Some(params);

                                    expected_return = Some(
                                        self.context
                                            .types
                                            .validate_type_reference(&invoke_sig.return_type)
                                            .unwrap_or(NormalizedType::Unknown),
                                    );
                                }
                            }

                            // Inferencia de firmas
                            if let Some(map) = &mut self.current_inferred_signatures {
                                let entry = map.entry(name.clone()).or_insert_with(Vec::new);

                                entry.push(arg_types.clone());

                                if let Some(first) = entry.first() {
                                    let params = first
                                        .iter()
                                        .enumerate()
                                        .map(|(i, t)| (format!("arg{}", i), t.clone()))
                                        .collect();

                                    expected_params = Some(params);
                                }
                            }
                        }

                        _ => {}
                    }

                    // 3. Delegar la validación al checker
                    match self.context.expression_checker.check_function_call(
                        name,
                        &arg_types,
                        expected_params.as_deref(),
                        expected_return.as_ref(),
                        &self.context.types,
                    ) {
                        Ok(res) => Ok(res.type_),

                        Err(e) => {
                            self.report_error(e.clone());

                            Ok(NormalizedType::Unknown)
                        }
                    }
                } else {
                    self.report_error(SemanticError::UnsupportedFeature {
                        feature: "complex function calls".to_string(),
                    });

                    Ok(NormalizedType::Unknown)
                }
            }
            ExprKind::Binary {
                left,
                operator,
                right,
            } => {
                let left_type = self.analyze_expr(left)?;

                let right_type = self.analyze_expr(right)?;

                match self.context.expression_checker.check_binary_op(
                    &left_type,
                    operator,
                    &right_type,
                    &self.context.types,
                ) {
                    Ok(res) => Ok(res.type_),

                    Err(e) => {
                        self.report_error(e.clone());

                        Ok(NormalizedType::Unknown)
                    }
                }
            }
            ExprKind::Block(exprs) => {
                let mut types = Vec::new();
                for e in exprs {
                    types.push(self.analyze_expr(e)?);
                }
                let et = self.context.expression_checker.check_block(&types)?;
                Ok(et.type_)
            }
            ExprKind::Let {
                name,
                annotation,
                value,
                body,
            } => {
                self.context.symbols.enter_scope();

                let value_type = self.analyze_expr(value)?;

                let annotation_type = match annotation {
                    Some(annotation) => {
                        match self.context.types.validate_type_reference(annotation) {
                            Ok(t) => Some(t),

                            Err(e) => {
                                self.report_error(e);

                                None
                            }
                        }
                    }

                    None => None,
                };

                let final_type = match self.context.expression_checker.check_let_expression(
                    annotation_type.as_ref(),
                    Some(&value_type),
                    &self.context.types,
                    &expr.span,
                ) {
                    Ok(res) => res.type_,

                    Err(e) => {
                        self.report_error(e);

                        NormalizedType::Unknown
                    }
                };

                let symbol = SymbolInfo::Variable {
                    name: name.clone(),

                    type_ref: final_type,

                    span: expr.span.clone(),
                };

                if let Err(e) = self.context.symbols.declare(symbol) {
                    self.report_error(e);
                }

                let body_type = self.analyze_expr(body);

                self.context.symbols.exit_scope();

                body_type
            }
            ExprKind::If {
                condition,
                then_expr,
                elif_parts,
                else_expr,
            } => {
                let cond_t = self.analyze_expr(condition)?;

                let then_t = self.analyze_expr(then_expr)?;

                let mut elif_types = Vec::new();

                for (condition, body) in elif_parts {
                    let elif_cond = self.analyze_expr(condition)?;

                    let elif_body = self.analyze_expr(body)?;

                    elif_types.push((elif_cond, elif_body));
                }

                let else_t = match else_expr {
                    Some(expr) => Some(self.analyze_expr(expr)?),

                    None => None,
                };

                match self.context.expression_checker.check_if_expression(
                    &cond_t,
                    &then_t,
                    &elif_types,
                    else_t.as_ref(),
                    &self.context.types,
                ) {
                    Ok(res) => Ok(res.type_),

                    Err(e) => {
                        self.report_error(e.clone());

                        Ok(NormalizedType::Unknown)
                    }
                }
            }
            ExprKind::TypeCheck {
                expr: inner,
                type_ref,
            } => {
                let expr_t = self.analyze_expr(inner)?;

                let target_t = match self.context.types.validate_type_reference(type_ref) {
                    Ok(t) => t,

                    Err(e) => {
                        self.report_error(e);

                        return Ok(NormalizedType::Unknown);
                    }
                };

                match self.context.expression_checker.check_is(
                    &expr_t,
                    &target_t,
                    &self.context.types,
                ) {
                    Ok(res) => Ok(res.type_),

                    Err(e) => {
                        self.report_error(e);

                        Ok(NormalizedType::Unknown)
                    }
                }
            }
            ExprKind::TypeCast {
                expr: inner,
                type_ref,
            } => {
                let expr_t = self.analyze_expr(inner)?;

                let target_t = match self.context.types.validate_type_reference(type_ref) {
                    Ok(t) => t,

                    Err(e) => {
                        self.report_error(e);

                        return Ok(NormalizedType::Unknown);
                    }
                };

                match self.context.expression_checker.check_as(
                    &expr_t,
                    &target_t,
                    &self.context.types,
                ) {
                    Ok(res) => Ok(res.type_),

                    Err(e) => {
                        self.report_error(e);

                        Ok(NormalizedType::Unknown)
                    }
                }
            }
            _ => Ok(NormalizedType::Unknown),
        }
    }

    /// Retorna todos los errores encontrados
    pub fn errors(&self) -> &[SemanticError] {
        self.context.get_errors()
    }

    /// Retorna true si hay errores
    pub fn has_errors(&self) -> bool {
        self.context.has_errors()
    }

    /// Reporta un error
    pub fn report_error(&mut self, error: SemanticError) {
        self.context.push_error(error);
    }
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
// tests moved to consolidated `tests.rs`
