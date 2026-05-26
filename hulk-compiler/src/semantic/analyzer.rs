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
use crate::semantic::symbol_table::SymbolTable;
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

        if self.context.has_errors() {
            return Err(self.context.errors[0].clone());
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

                // Declare function in symbol table
                let sym = crate::semantic::symbol_table::SymbolInfo::Function {
                    name: func.name.clone(),
                    parameters: func.parameters.clone(),
                    return_type: func.return_type.clone(),
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
                    let param_sym = crate::semantic::symbol_table::SymbolInfo::Parameter {
                        name: p.name.clone(),
                        type_ref: p.annotation.clone(),
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
    fn check_entry_expression(&mut self, _program: &Program) -> SemanticResult<()> {
        if let Some(expr) = &_program.entry_expression {
            // Try to infer/check the expression and collect errors
            let _ = self.analyze_expr(expr);
        }
        Ok(())
    }

    /// Basic recursive expression analyzer that uses `ExpressionChecker` and
    /// available symbol/type information to validate common constructs.
    fn analyze_expr(&mut self, expr: &Expr) -> Result<NormalizedType, SemanticError> {
        match &expr.kind {
            ExprKind::Literal(_) => {
                let et = self.context.expression_checker.check_literal(expr)?;
                Ok(et.type_)
            }
            ExprKind::Identifier(name) => match self.context.symbols.lookup(name.as_str()) {
                Some(sym) => match sym {
                    crate::semantic::symbol_table::SymbolInfo::Variable { type_ref, .. }
                    | crate::semantic::symbol_table::SymbolInfo::Parameter { type_ref, .. } => {
                        if let Some(tr) = type_ref {
                            match self.context.types.validate_type_reference(&tr) {
                                Ok(nt) => Ok(nt),
                                Err(e) => {
                                    self.report_error(e.clone());
                                    Ok(NormalizedType::Unknown)
                                }
                            }
                        } else {
                            Ok(NormalizedType::Unknown)
                        }
                    }
                    crate::semantic::symbol_table::SymbolInfo::Function { .. } => {
                        Ok(NormalizedType::Unknown)
                    }
                    _ => Ok(NormalizedType::Unknown),
                },
                None => {
                    self.report_error(SemanticError::UndeclaredVariable { name: name.clone() });
                    Ok(NormalizedType::Unknown)
                }
            },
            ExprKind::Call { callee, arguments } => {
                // Only handle simple identifier callees
                if let ExprKind::Identifier(name) = &callee.kind {
                    let mut arg_types = Vec::new();
                    for a in arguments {
                        arg_types.push(self.analyze_expr(a)?);
                    }

                    // Try to obtain expected params/return from symbol table or from a
                    // protocol named in the variable's annotation (e.g., a functor protocol
                    // that defines `invoke(...)`). This allows calling parameters that are
                    // typed as a protocol with an `invoke` method.
                    let mut expected_params: Option<Vec<(String, NormalizedType)>> = None;
                    let mut expected_return: Option<NormalizedType> = None;

                    match self.context.symbols.lookup(name.as_str()) {
                        Some(crate::semantic::symbol_table::SymbolInfo::Function {
                            parameters,
                            return_type,
                            ..
                        }) => {
                            let mut params = Vec::new();
                            for p in parameters {
                                let t = if let Some(ann) = &p.annotation {
                                    self.context
                                        .types
                                        .validate_type_reference(ann)
                                        .unwrap_or(NormalizedType::Unknown)
                                } else {
                                    NormalizedType::Unknown
                                };
                                params.push((p.name.clone(), t));
                            }
                            expected_params = Some(params);
                            expected_return = return_type
                                .as_ref()
                                .and_then(|rt| self.context.types.validate_type_reference(rt).ok());
                        }
                        Some(crate::semantic::symbol_table::SymbolInfo::Variable {
                            type_ref,
                            ..
                        })
                        | Some(crate::semantic::symbol_table::SymbolInfo::Parameter {
                            type_ref,
                            ..
                        }) => {
                            if let Some(tr) = type_ref
                                && let crate::parser::ast::TypeReferenceKind::Named(type_name) =
                                    &tr.kind
                            {
                                // If the annotation names a protocol, try to extract an
                                // `invoke` method signature from it.
                                if let Some(proto) = self.context.types.get_protocol(type_name)
                                    && let Some(invoke_sig) =
                                        proto.members.iter().find(|m| m.name == "invoke")
                                {
                                    let mut params = Vec::new();
                                    for p in &invoke_sig.parameters {
                                        let t = if let Some(ann) = &p.annotation {
                                            self.context
                                                .types
                                                .validate_type_reference(ann)
                                                .unwrap_or(NormalizedType::Unknown)
                                        } else {
                                            NormalizedType::Unknown
                                        };
                                        params.push((p.name.clone(), t));
                                    }
                                    expected_params = Some(params);
                                    // return type for invoke is required in protocols
                                    expected_return = Some(
                                        self.context
                                            .types
                                            .validate_type_reference(&invoke_sig.return_type)
                                            .unwrap_or(NormalizedType::Unknown),
                                    );
                                }
                            } else {
                                // No annotation: try to use inferred signature (collected
                                // during this function body's analysis). If none exists yet,
                                // record the observed argument types so subsequent calls
                                // can be checked against the first observed signature.
                                if let Some(map) = &mut self.current_inferred_signatures {
                                    let entry = map.entry(name.clone()).or_insert_with(Vec::new);
                                    // push the observed arg types for later unification
                                    entry.push(arg_types.clone());

                                    // If this is the first observed call, use it to build
                                    // an expected_params vector for immediate checking.
                                    if let Some(first) = entry.first() {
                                        let mut params = Vec::new();
                                        for (i, t) in first.iter().enumerate() {
                                            params.push((format!("arg{}", i), t.clone()));
                                        }
                                        expected_params = Some(params);
                                        // return type remains unknown in this heuristic
                                        expected_return = None;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }

                    match self.context.expression_checker.check_function_call(
                        name,
                        &arg_types,
                        expected_params.as_deref(),
                        expected_return.as_ref(),
                        &expr.span,
                    ) {
                        Ok(res) => Ok(res.type_),
                        Err(e) => {
                            self.report_error(e.clone());
                            Ok(NormalizedType::Unknown)
                        }
                    }
                } else {
                    // Complex callee (member calls etc.) not yet supported
                    Ok(NormalizedType::Unknown)
                }
            }
            ExprKind::Binary {
                left,
                operator,
                right,
            } => {
                let lt = self.analyze_expr(left)?;
                let rt = self.analyze_expr(right)?;
                match self
                    .context
                    .expression_checker
                    .check_binary_op(&lt, operator, &rt, &expr.span)
                {
                    Ok(et) => Ok(et.type_),
                    Err(e) => {
                        self.report_error(e.clone());
                        Ok(NormalizedType::Unknown)
                    }
                }
            }
            ExprKind::Grouping(inner) => self.analyze_expr(inner),
            ExprKind::Block(exprs) => {
                let mut types = Vec::new();
                for e in exprs {
                    types.push(self.analyze_expr(e)?);
                }
                let et = self.context.expression_checker.check_block(&types)?;
                Ok(et.type_)
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
                for (c, e) in elif_parts {
                    let ct = self.analyze_expr(c)?;
                    let et = self.analyze_expr(e)?;
                    elif_types.push((ct, et));
                }
                let else_t = if let Some(e) = else_expr {
                    Some(self.analyze_expr(e)?)
                } else {
                    None
                };

                match self.context.expression_checker.check_if_expression(
                    &cond_t,
                    &then_t,
                    &elif_types
                        .iter()
                        .map(|(a, b)| (a.clone(), b.clone()))
                        .collect::<Vec<_>>(),
                    else_t.as_ref(),
                    &expr.span,
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
                match self.context.types.validate_type_reference(type_ref) {
                    Ok(target_t) => match self
                        .context
                        .expression_checker
                        .check_is(&expr_t, &target_t, &expr.span)
                    {
                        Ok(et) => Ok(et.type_),
                        Err(e) => {
                            self.report_error(e.clone());
                            Ok(NormalizedType::Unknown)
                        }
                    },
                    Err(e) => {
                        self.report_error(e.clone());
                        Ok(NormalizedType::Unknown)
                    }
                }
            }
            ExprKind::TypeCast {
                expr: inner,
                type_ref,
            } => {
                let expr_t = self.analyze_expr(inner)?;
                match self.context.types.validate_type_reference(type_ref) {
                    Ok(target_t) => {
                        // Allow cast if same type, unknowns, or types are compatible in either direction
                        if expr_t == target_t
                            || expr_t == NormalizedType::Unknown
                            || target_t == NormalizedType::Unknown
                            || self.context.types.is_compatible(&expr_t, &target_t)
                            || self.context.types.is_compatible(&target_t, &expr_t)
                        {
                            match self
                                .context
                                .expression_checker
                                .check_as(&expr_t, &target_t, &expr.span)
                            {
                                Ok(et) => Ok(et.type_),
                                Err(e) => {
                                    self.report_error(e.clone());
                                    Ok(NormalizedType::Unknown)
                                }
                            }
                        } else {
                            // Not compatible cast
                            self.report_error(SemanticError::TypeMismatch {
                                expected: target_t.to_string(),
                                found: expr_t.to_string(),
                                context: "cast".to_string(),
                            });
                            Ok(target_t)
                        }
                    }
                    Err(e) => {
                        self.report_error(e.clone());
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
