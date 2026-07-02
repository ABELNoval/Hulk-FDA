use crate::parser::ast::{
    DeclarationKind, Expr, ExprKind, FunctionDeclaration, TypeMember, TypeReferenceKind,
};
use crate::parser::ast::{Program, UnaryOperator};
use crate::semantic::expression_checker::ExpressionChecker;
use crate::semantic::symbol_table::{SymbolInfo, SymbolTable};
use crate::semantic::type_system::NormalizedType;
use crate::semantic::type_system::TypeEnvironment;
use crate::utils::Span;
use crate::utils::errors::semantic::SemanticError;

type SemanticResult<T> = Result<T, SemanticError>;

pub struct SemanticContext {
    /// Tabla de símbolos con gestión de scopes
    pub symbols: SymbolTable,
    /// Entorno de tipos (tipos definidos, protocolos)
    pub types: TypeEnvironment,
    /// Checker de tipos de expresiones
    pub expression_checker: ExpressionChecker,
    // Tabla paralela que mapea Expr.id
    pub expr_types: std::collections::HashMap<usize, NormalizedType>,
    /// Errores semánticos encontrados
    pub errors: Vec<SemanticError>,
}

impl SemanticContext {
    pub fn new() -> Self {
        Self {
            symbols: SymbolTable::new(),
            types: TypeEnvironment::new(),
            expression_checker: ExpressionChecker::new(),
            expr_types: std::collections::HashMap::new(),
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
    current_inferred_signatures:
        Option<std::collections::HashMap<String, Vec<Vec<NormalizedType>>>>,
    observed_call_signatures: std::collections::HashMap<String, Vec<Vec<NormalizedType>>>,
    method_signatures: std::collections::HashMap<String, (Vec<NormalizedType>, NormalizedType)>,
    type_fields:
        std::collections::HashMap<String, std::collections::HashMap<String, NormalizedType>>,
    type_parents: std::collections::HashMap<String, String>,
    current_method_context: Option<(String, String)>,
    current_error_span: Option<Span>,
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
            observed_call_signatures: std::collections::HashMap::new(),
            method_signatures: std::collections::HashMap::new(),
            type_fields: std::collections::HashMap::new(),
            type_parents: std::collections::HashMap::new(),
            current_method_context: None,
            current_error_span: None,
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
            self.current_error_span = Some(decl.span.clone());
            match &decl.kind {
                DeclarationKind::Type(td) => {
                    // 1. Extraer padre primero
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

                    // Mapa de parámetros del tipo para inferir tipos de atributos
                    let param_types: std::collections::HashMap<
                        String,
                        crate::parser::ast::TypeReference,
                    > = td
                        .parameters
                        .iter()
                        .filter_map(|p| p.annotation.as_ref().map(|a| (p.name.clone(), a.clone())))
                        .collect();

                    // Los parámetros del tipo SON atributos (accesibles via self)
                    for param in &td.parameters {
                        if let Some(ann) = &param.annotation {
                            properties.push((param.name.clone(), ann.clone()));
                        }
                    }

                    for member in &td.members {
                        match member {
                            TypeMember::Method(f) => methods.push(f.clone()),
                            TypeMember::Attribute(a) => {
                                if let Some(ann) = &a.annotation {
                                    properties.push((a.name.clone(), ann.clone()));
                                } else {
                                    // Inferir tipo del inicializador si es un identificador conocido
                                    let inferred =
                                        if let ExprKind::Identifier(name) = &a.initializer.kind {
                                            param_types.get(name).cloned()
                                        } else {
                                            None
                                        };
                                    if let Some(type_ref) = inferred {
                                        properties.push((a.name.clone(), type_ref));
                                    } else {
                                        // Fallback: registrar como Unknown para que sea visible
                                        properties.push((
                                            a.name.clone(),
                                            crate::parser::ast::TypeReference::new(
                                                "Unknown".to_string(),
                                                a.span.clone(),
                                            ),
                                        ));
                                    }
                                }
                            }
                        }
                    }

                    // 3. AHORA construir type_info (ya existe `parent`, `methods`, `properties`)
                    let type_info = crate::semantic::type_system::TypeInfo {
                        name: td.name.clone(),
                        parameters: td.parameters.clone(),
                        parent: parent.clone(),
                        methods: methods.clone(),
                        properties: properties.clone(),
                        implemented_protocols: vec![],
                        span: crate::utils::errors::span::Span::default(),
                    };

                    // 4. Verificar overrides contra el padre (usamos `parent`, no `type_info.parent`)
                    if let Some(parent_name) = &parent {
                        for method in &methods {
                            let parent_key = format!("{}_{}", parent_name, method.name);
                            if let Some((parent_params, parent_ret)) =
                                self.method_signatures.get(&parent_key).cloned()
                            {
                                let child_params: Vec<NormalizedType> = method
                                    .parameters
                                    .iter()
                                    .map(|p| {
                                        p.annotation
                                            .as_ref()
                                            .map(|a| {
                                                self.context
                                                    .types
                                                    .validate_type_reference(a)
                                                    .unwrap_or(NormalizedType::Unknown)
                                            })
                                            .unwrap_or(NormalizedType::Unknown)
                                    })
                                    .collect();
                                let child_ret = method
                                    .return_type
                                    .as_ref()
                                    .map(|r| {
                                        self.context
                                            .types
                                            .validate_type_reference(r)
                                            .unwrap_or(NormalizedType::Unknown)
                                    })
                                    .unwrap_or(NormalizedType::Unknown);

                                if parent_params.len() != child_params.len() {
                                    self.report_error(SemanticError::WrongArgumentCount {
                                        function: format!("{}_{}", td.name, method.name),
                                        expected: parent_params.len(),
                                        found: child_params.len(),
                                    });
                                } else {
                                    for (i, (p, c)) in
                                        parent_params.iter().zip(child_params.iter()).enumerate()
                                    {
                                        if p != c && !p.is_unknown() && !c.is_unknown() {
                                            self.report_error(
                                                SemanticError::ArgumentTypeMismatch {
                                                    function: format!(
                                                        "{}_{}",
                                                        td.name, method.name
                                                    ),
                                                    parameter_name: method
                                                        .parameters
                                                        .get(i)
                                                        .map(|x| x.name.clone())
                                                        .unwrap_or_default(),
                                                    parameter_position: i,
                                                    expected: p.to_string(),
                                                    found: c.to_string(),
                                                },
                                            );
                                        }
                                    }
                                }
                                if parent_ret != child_ret
                                    && !parent_ret.is_unknown()
                                    && !child_ret.is_unknown()
                                {
                                    self.report_error(SemanticError::ReturnTypeMismatch {
                                        function: format!("{}_{}", td.name, method.name),
                                        expected: parent_ret.to_string(),
                                        found: child_ret.to_string(),
                                    });
                                }
                            }
                        }
                    }

                    // 5. Registrar relación de herencia
                    if let Some(ref parent_name) = type_info.parent {
                        self.type_parents
                            .insert(type_info.name.clone(), parent_name.clone());
                    }

                    // 6. Registrar campos para lookup de MemberAccess
                    let mut fields = std::collections::HashMap::new();
                    for (name, type_ref) in &properties {
                        let normalized = self
                            .context
                            .types
                            .validate_type_reference(type_ref)
                            .unwrap_or(NormalizedType::Unknown);
                        fields.insert(name.clone(), normalized);
                    }
                    self.type_fields.insert(type_info.name.clone(), fields);

                    // 7. Registrar firmas de métodos para lookup de llamadas
                    for method in &methods {
                        let param_types: Vec<NormalizedType> = method
                            .parameters
                            .iter()
                            .map(|p| {
                                p.annotation
                                    .as_ref()
                                    .map(|a| {
                                        self.context
                                            .types
                                            .validate_type_reference(a)
                                            .unwrap_or(NormalizedType::Unknown)
                                    })
                                    .unwrap_or(NormalizedType::Unknown)
                            })
                            .collect();
                        let ret_type = method
                            .return_type
                            .as_ref()
                            .map(|r| {
                                self.context
                                    .types
                                    .validate_type_reference(r)
                                    .unwrap_or(NormalizedType::Unknown)
                            })
                            .unwrap_or(NormalizedType::Unknown);
                        let key = format!("{}_{}", type_info.name, method.name);
                        self.method_signatures.insert(key, (param_types, ret_type));
                    }

                    // Detectar protocolos implementados implícitamente
                    let type_method_names: Vec<String> =
                        methods.iter().map(|m| m.name.clone()).collect();
                    let mut implemented = Vec::new();
                    for (proto_name, proto_info) in self.context.types.protocols() {
                        let proto_methods: Vec<String> =
                            proto_info.members.iter().map(|m| m.name.clone()).collect();
                        let all_present = proto_methods
                            .iter()
                            .all(|pm| type_method_names.contains(pm));
                        if all_present {
                            implemented.push(proto_name.clone());
                        }
                    }
                    for proto in &implemented {
                        self.context
                            .types
                            .add_type_implements(td.name.clone(), proto.clone());
                    }

                    // 8. Registrar el tipo en el entorno de tipos
                    if let Err(e) = self.context.types.register_type(type_info.clone()) {
                        self.report_error(e);
                    }

                    if let Some(ti) = self.context.types.get_type_mut(&td.name) {
                        ti.implemented_protocols = implemented;
                    }

                    // 9. Declarar el tipo en la tabla de símbolos
                    let sym = SymbolInfo::Type {
                        name: td.name.clone(),
                        span: crate::utils::errors::span::Span::default(),
                    };
                    if let Err(e) = self.context.symbols.declare(sym) {
                        self.report_error(e);
                    }

                    // 10. Pre-declarar métodos en la tabla de símbolos
                    for method in &methods {
                        let normalized_return = match &method.return_type {
                            Some(type_ref) => self
                                .context
                                .types
                                .validate_type_reference(type_ref)
                                .unwrap_or(NormalizedType::Unknown),
                            None => NormalizedType::Unknown,
                        };
                        let sym = SymbolInfo::Function {
                            name: format!("{}_{}", td.name, method.name),
                            parameters: method.parameters.clone(),
                            return_type: normalized_return,
                            span: crate::utils::errors::span::Span::default(),
                        };
                        let _ = self.context.symbols.declare(sym);
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
        // Collect call signatures from top-level entry expression to help inference.
        let initial_errors = self.context.errors.len();
        let _ = self.check_entry_expression(_program);
        self.context.errors.truncate(initial_errors);

        for decl in &_program.declarations {
            if let DeclarationKind::Function(func) = &decl.kind {
                self.context.symbols.enter_scope();

                // declare parameters as Parameter symbols with their annotations
                for (i, p) in func.parameters.iter().enumerate() {
                    let normalized_type = match &p.annotation {
                        Some(annotation) => {
                            self.context.types.validate_type_reference(annotation)?
                        }
                        None => {
                            if let Some(sigs) = self.observed_call_signatures.get(&func.name) {
                                if !sigs.is_empty() && sigs[0].len() > i {
                                    sigs[0][i].clone()
                                } else {
                                    NormalizedType::Unknown
                                }
                            } else {
                                NormalizedType::Unknown
                            }
                        }
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
                } else if body_t != NormalizedType::Unknown {
                    if let Err(e) = self
                        .context
                        .symbols
                        .update_function_return_type(&func.name, body_t.clone())
                    {
                        self.report_error(e);
                    }
                }

                self.context.symbols.exit_scope();
            }
        }

        // Fourth pass: analyze type method bodies
        for decl in &_program.declarations {
            if let DeclarationKind::Type(td) = &decl.kind {
                for member in &td.members {
                    if let TypeMember::Method(method) = member {
                        self.context.symbols.enter_scope();

                        // Declare self
                        let self_sym = SymbolInfo::Parameter {
                            name: "self".to_string(),
                            type_ref: NormalizedType::Named(td.name.clone()),
                            span: crate::utils::errors::span::Span::default(),
                        };
                        if let Err(e) = self.context.symbols.declare(self_sym) {
                            self.report_error(e);
                        }

                        // Declare method parameters
                        for p in method.parameters.iter() {
                            let normalized_type = match &p.annotation {
                                Some(annotation) => self
                                    .context
                                    .types
                                    .validate_type_reference(annotation)
                                    .unwrap_or(NormalizedType::Unknown),
                                None => NormalizedType::Unknown,
                            };
                            let param_sym = SymbolInfo::Parameter {
                                name: p.name.clone(),
                                type_ref: normalized_type,
                                span: p.span.clone(),
                            };
                            if let Err(e) = self.context.symbols.declare(param_sym) {
                                self.report_error(e);
                            }
                        }

                        // Analyze body
                        self.current_method_context = Some((td.name.clone(), method.name.clone()));
                        let body_t = match self.analyze_expr(&method.body) {
                            Ok(t) => t,
                            Err(e) => {
                                self.report_error(e);
                                NormalizedType::Unknown
                            }
                        };
                        self.current_method_context = None;

                        // Inferir tipo de retorno si no está anotado
                        if method.return_type.is_none() {
                            let key = format!("{}_{}", td.name, method.name);
                            if let Some((_, ret)) = self.method_signatures.get_mut(&key) {
                                *ret = body_t;
                            }
                        }

                        self.context.symbols.exit_scope();
                    }
                }
            }
        }

        // Analyze entry expression after function body inference so inferred
        // return types are available for top-level calls.
        self.check_entry_expression(_program)?;

        Ok(())
    }
    /// Verifica la expresión de entrada del programa
    fn check_entry_expression(&mut self, program: &Program) -> SemanticResult<()> {
        if let Some(expr) = &program.entry_expression
            && let Err(e) = self.analyze_expr(expr)
        {
            self.report_error(e);
        }

        Ok(())
    }

    /// Basic recursive expression analyzer that uses `ExpressionChecker` and
    /// available symbol/type information to validate common constructs.
    fn analyze_expr(&mut self, expr: &Expr) -> Result<NormalizedType, SemanticError> {
        self.current_error_span = Some(expr.span.clone());
        let ty = match &expr.kind {
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
            ExprKind::Unary { operator, operand } => {
                let operand_t = self.analyze_expr(operand)?;

                let is_negation = matches!(operator, UnaryOperator::Minus);

                match self
                    .context
                    .expression_checker
                    .check_unary_op(&operand_t, is_negation)
                {
                    Ok(res) => Ok(res.type_),

                    Err(e) => {
                        self.report_error(e.clone());

                        Ok(NormalizedType::Unknown)
                    }
                }
            }
            ExprKind::Call { callee, arguments } => {
                // Caso 1: obj.method(args)
                if let ExprKind::MemberAccess { object, member } = &callee.kind {
                    let obj_type = self.analyze_expr(object)?;
                    let mut arg_types = Vec::new();
                    for arg in arguments {
                        arg_types.push(self.analyze_expr(arg)?);
                    }
                    return match &obj_type {
                        NormalizedType::Named(type_name) => {
                            let mangled_name = format!("{}_{}", type_name, member);
                            let method_sig = self.find_method_signature(type_name, member);
                            if let Some((expected_params, expected_return)) = method_sig {
                                if arguments.len() != expected_params.len() {
                                    self.report_error(SemanticError::WrongArgumentCount {
                                        function: mangled_name,
                                        expected: expected_params.len(),
                                        found: arguments.len(),
                                    });
                                } else {
                                    for (i, (arg_type, param_type)) in
                                        arg_types.iter().zip(expected_params.iter()).enumerate()
                                    {
                                        if !self.context.types.is_compatible(arg_type, param_type)
                                            && !arg_type.is_unknown()
                                            && !param_type.is_unknown()
                                        {
                                            self.report_error(
                                                SemanticError::ArgumentTypeMismatch {
                                                    function: mangled_name.clone(),
                                                    parameter_name: format!("arg{}", i),
                                                    parameter_position: i,
                                                    expected: param_type.to_string(),
                                                    found: arg_type.to_string(),
                                                },
                                            );
                                        }
                                    }
                                }
                                Ok(expected_return.clone())
                            } else {
                                self.report_error(SemanticError::MemberNotFound {
                                    type_name: type_name.clone(),
                                    member_name: member.clone(),
                                });
                                Ok(NormalizedType::Unknown)
                            }
                        }
                        NormalizedType::Protocol(proto_name) => {
                            if let Some(proto_info) = self.context.types.get_protocol(proto_name) {
                                if let Some(method) =
                                    proto_info.members.iter().find(|m| m.name == *member)
                                {
                                    let ret_type = self
                                        .context
                                        .types
                                        .validate_type_reference(&method.return_type)
                                        .unwrap_or(NormalizedType::Unknown);
                                    Ok(ret_type)
                                } else {
                                    self.report_error(SemanticError::MemberNotFound {
                                        type_name: proto_name.clone(),
                                        member_name: member.clone(),
                                    });
                                    Ok(NormalizedType::Unknown)
                                }
                            } else {
                                Ok(NormalizedType::Unknown)
                            }
                        }
                        _ => {
                            self.report_error(SemanticError::UnsupportedFeature {
                                feature: "method call on non-object".to_string(),
                            });
                            Ok(NormalizedType::Unknown)
                        }
                    };
                }

                // Caso 2: base(args)
                if let ExprKind::Base = &callee.kind {
                    for arg in arguments {
                        let _ = self.analyze_expr(arg)?;
                    }
                    if let Some((type_name, method_name)) = &self.current_method_context
                        && let Some(parent_name) = self.type_parents.get(type_name)
                    {
                        let parent_key = format!("{}_{}", parent_name, method_name);
                        if let Some((_, ret_type)) = self.method_signatures.get(&parent_key) {
                            return Ok(ret_type.clone());
                        }
                    }
                    return Ok(NormalizedType::Unknown);
                }

                // Caso 3: función global identificador
                if let ExprKind::Identifier(name) = &callee.kind {
                    if name == "range" {
                        let mut arg_types = Vec::new();
                        for arg in arguments {
                            arg_types.push(self.analyze_expr(arg)?);
                        }
                        let ty = self.context.expression_checker.check_function_call(
                            name,
                            &arg_types,
                            Some(&[
                                ("lo".to_string(), NormalizedType::Number),
                                ("hi".to_string(), NormalizedType::Number),
                            ]),
                            Some(&NormalizedType::Iterable(Box::new(NormalizedType::Number))),
                            &self.context.types,
                        )?;
                        self.context.expr_types.insert(expr.id, ty.type_.clone());
                        return Ok(ty.type_);
                    }
                    let mut arg_types = Vec::new();
                    for arg in arguments {
                        arg_types.push(self.analyze_expr(arg)?);
                    }

                    self.observed_call_signatures
                        .entry(name.clone())
                        .or_insert_with(Vec::new)
                        .push(arg_types.clone());

                    let lookup = self.context.symbols.lookup(name);
                    let (expected_params, expected_return) = match lookup {
                        Some(SymbolInfo::Function {
                            parameters,
                            return_type,
                            ..
                        }) => {
                            let params: Vec<(String, NormalizedType)> = parameters
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
                            (Some(params), Some(return_type))
                        }
                        _ => {
                            self.report_error(SemanticError::UndefinedFunction {
                                name: name.clone(),
                            });
                            (None, None)
                        }
                    };

                    let result: Result<NormalizedType, SemanticError> = self
                        .context
                        .expression_checker
                        .check_function_call(
                            name,
                            &arg_types,
                            expected_params.as_deref(),
                            expected_return.as_ref(),
                            &self.context.types,
                        )
                        .map(|res| res.type_)
                        .or_else(|e| {
                            self.report_error(e);
                            Ok(NormalizedType::Unknown)
                        });
                    self.context
                        .expr_types
                        .insert(expr.id, result.clone().unwrap_or(NormalizedType::Unknown));
                    return result;
                }

                // Fallback
                self.report_error(SemanticError::UnsupportedFeature {
                    feature: "complex function calls".to_string(),
                });
                Ok(NormalizedType::Unknown)
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
            ExprKind::For {
                variable,
                iterable,
                body,
            } => {
                let iterable_type = self.analyze_expr(iterable)?;

                // Inferir tipo del elemento del iterable
                let element_type = match &iterable_type {
                    NormalizedType::Iterable(inner) => (**inner).clone(),
                    NormalizedType::Vector(inner) => (**inner).clone(),
                    NormalizedType::Unknown => NormalizedType::Unknown,
                    NormalizedType::Named(type_name) => {
                        // Si implementa Iterable, inferir el tipo del current()
                        if self.context.types.type_implements(type_name, "Iterable") {
                            // Buscar el método current() en el tipo para obtener su tipo de retorno
                            if let Some(type_info) = self.context.types.get_type(type_name) {
                                if let Some(current_method) =
                                    type_info.methods.iter().find(|m| m.name == "current")
                                {
                                    if let Some(ret_type_ref) = &current_method.return_type {
                                        self.context
                                            .types
                                            .validate_type_reference(ret_type_ref)
                                            .unwrap_or(NormalizedType::Unknown)
                                    } else {
                                        NormalizedType::Unknown
                                    }
                                } else {
                                    NormalizedType::Unknown
                                }
                            } else {
                                NormalizedType::Unknown
                            }
                        } else {
                            self.report_error(SemanticError::InvalidOperandType {
                                expected: "iterable".to_string(),
                                found: iterable_type.to_string(),
                                context: "for".to_string(),
                            });
                            NormalizedType::Unknown
                        }
                    }
                    _ => {
                        self.report_error(SemanticError::InvalidOperandType {
                            expected: "iterable".to_string(),
                            found: iterable_type.to_string(),
                            context: "for".to_string(),
                        });
                        NormalizedType::Unknown
                    }
                };

                // Declarar la variable de iteración en el scope
                self.context.symbols.enter_scope();
                let var_sym = SymbolInfo::Variable {
                    name: variable.clone(),
                    type_ref: element_type,
                    span: expr.span.clone(),
                };
                if let Err(e) = self.context.symbols.declare(var_sym) {
                    self.report_error(e);
                }

                let body_type = self.analyze_expr(body)?;

                self.context.symbols.exit_scope();

                match self.context.expression_checker.check_for_expression(
                    &iterable_type,
                    &body_type,
                    &self.context.types,
                ) {
                    Ok(res) => Ok(res.type_),
                    Err(e) => {
                        self.report_error(e.clone());
                        Ok(NormalizedType::Unknown)
                    }
                }
            }
            ExprKind::MemberAccess { object, member } => {
                let obj_type = self.analyze_expr(object)?;
                if let NormalizedType::Named(type_name) = &obj_type {
                    if let Some(field_type) = self.find_field_type(type_name, member) {
                        Ok(field_type)
                    } else {
                        self.report_error(SemanticError::MemberNotFound {
                            type_name: type_name.clone(),
                            member_name: member.clone(),
                        });
                        Ok(NormalizedType::Unknown)
                    }
                } else {
                    Ok(NormalizedType::Unknown)
                }
            }
            ExprKind::IndexAccess { object, index } => {
                self.analyze_expr(object)?;
                self.analyze_expr(index)?;
                Ok(NormalizedType::Unknown)
            }
            ExprKind::Self_ => match self.context.symbols.lookup("self") {
                Some(SymbolInfo::Parameter { type_ref, .. }) => Ok(type_ref.clone()),
                Some(SymbolInfo::Variable { type_ref, .. }) => Ok(type_ref.clone()),
                Some(_) => Ok(NormalizedType::Unknown),
                None => {
                    self.report_error(SemanticError::UndeclaredVariable {
                        name: "self".to_string(),
                        span: expr.span.clone(),
                    });
                    Ok(NormalizedType::Unknown)
                }
            },
            ExprKind::Base => match self.context.symbols.lookup("base") {
                Some(_) => Ok(NormalizedType::Unknown),
                None => {
                    self.report_error(SemanticError::UndeclaredVariable {
                        name: "base".to_string(),
                        span: expr.span.clone(),
                    });
                    Ok(NormalizedType::Unknown)
                }
            },
            ExprKind::New { type_ref, .. } => {
                let type_name = type_ref.display_name();
                if !self.context.types.has_type(&type_name) {
                    self.report_error(SemanticError::UndeclaredType {
                        name: type_name.clone(),
                    });
                }
                Ok(NormalizedType::Named(type_name))
            }
            ExprKind::Assignment { target, value } => {
                let value_t = self.analyze_expr(value)?;
                let target_t = self.analyze_expr(target)?;
                // Verificar compatibilidad de tipos si es posible
                if !self.context.types.is_compatible(&value_t, &target_t)
                    && target_t != NormalizedType::Unknown
                    && value_t != NormalizedType::Unknown
                {
                    self.report_error(SemanticError::TypeMismatch {
                        expected: target_t.to_string(),
                        found: value_t.to_string(),
                        context: "asignación".to_string(),
                        span: expr.span.clone(),
                    });
                }
                Ok(value_t)
            }
            ExprKind::Lambda {
                parameters,
                return_type,
                body,
            } => {
                self.context.symbols.enter_scope();
                for p in parameters {
                    let p_ty = p
                        .annotation
                        .as_ref()
                        .map(|a| {
                            self.context
                                .types
                                .validate_type_reference(a)
                                .unwrap_or(NormalizedType::Unknown)
                        })
                        .unwrap_or(NormalizedType::Unknown);
                    let sym = SymbolInfo::Parameter {
                        name: p.name.clone(),
                        type_ref: p_ty,
                        span: p.span.clone(),
                    };
                    let _ = self.context.symbols.declare(sym);
                }
                let body_t = self.analyze_expr(body)?;
                self.context.symbols.exit_scope();

                // Verificar contra anotación de retorno si existe
                if let Some(ret_ann) = return_type {
                    let expected = self.context.types.validate_type_reference(ret_ann)?;
                    if body_t != NormalizedType::Unknown
                        && !self.context.types.is_compatible(&body_t, &expected)
                    {
                        self.report_error(SemanticError::ReturnTypeMismatch {
                            function: "<lambda>".to_string(),
                            expected: expected.to_string(),
                            found: body_t.to_string(),
                        });
                    }
                }
                Ok(NormalizedType::Unknown)
            }
            _ => Ok(NormalizedType::Unknown),
        };
        if let Ok(ref resolved_type) = ty {
            self.context
                .expr_types
                .insert(expr.id, resolved_type.clone());
        }
        ty
    }

    fn find_method_signature(
        &self,
        type_name: &str,
        method_name: &str,
    ) -> Option<(Vec<NormalizedType>, NormalizedType)> {
        let key = format!("{}_{}", type_name, method_name);
        if let Some(sig) = self.method_signatures.get(&key) {
            return Some(sig.clone());
        }
        // Buscar recursivamente en el padre
        if let Some(parent) = self.type_parents.get(type_name) {
            return self.find_method_signature(parent, method_name);
        }
        None
    }

    fn find_field_type(&self, type_name: &str, field_name: &str) -> Option<NormalizedType> {
        if let Some(fields) = self.type_fields.get(type_name)
            && let Some(ty) = fields.get(field_name)
        {
            return Some(ty.clone());
        }
        // Buscar recursivamente en el padre
        if let Some(parent) = self.type_parents.get(type_name) {
            return self.find_field_type(parent, field_name);
        }
        None
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
        if let Some(span) = &self.current_error_span {
            self.context
                .push_error(error.with_fallback_span(span.clone()));
        } else {
            self.context.push_error(error);
        }
    }

    pub fn take_expr_types(&mut self) -> std::collections::HashMap<usize, NormalizedType> {
        std::mem::take(&mut self.context.expr_types)
    }
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
// tests moved to consolidated `tests.rs`
