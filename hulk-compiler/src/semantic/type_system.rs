// =============================================================================
// Type System & Type Environment
// =============================================================================
//
// Responsabilidad de Persona 2:
// - Mantener registro de tipos declarados (type, protocol)
// - Verificar herencia y conformancia a protocolos
// - Resolver referencias a tipos
// - Mantener jerarquía de tipos
//
// Esta es la interfaz compartida. Los métodos son stubs que Persona 2 implementará.
//
// =============================================================================

use crate::parser::ast::{
    Expr, FunctionDeclaration, Literal, ProtocolMethodSignature, TypeReference, TypeReferenceKind,
};
use crate::utils::errors::semantic::SemanticError;
use crate::utils::errors::span::Span;
use std::collections::HashMap;

/// Información sobre un tipo declarado
#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub name: String,
    pub parameters: Vec<crate::parser::ast::Parameter>,
    pub parent: Option<String>,
    pub methods: Vec<FunctionDeclaration>,
    pub properties: Vec<(String, TypeReference)>,
    pub implemented_protocols: Vec<String>,
    pub span: Span,
}

/// Información sobre un protocolo declarado
#[derive(Debug, Clone)]
pub struct ProtocolInfo {
    pub name: String,
    pub members: Vec<ProtocolMethodSignature>,
    pub extends: Vec<String>,
    pub span: Span,
}

/// Tipo normalizado que representa cómo el compilador interpreta los tipos
///
/// Los tipos del programa se normalizan a:
/// - Tipos builtin (Number, String, Boolean)
/// - Tipos definidos por el usuario (identificados por nombre)
/// - Tipos iterables (T*)
/// - Tipos vector (T[])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NormalizedType {
    /// Tipos builtin
    Number,
    String,
    Boolean,
    /// Tipo definido por el usuario (nombre)
    Named(String),
    /// Tipo iterable (por ejemplo, Number*)
    Iterable(Box<NormalizedType>),
    /// Tipo vector (por ejemplo, Number[])
    Vector(Box<NormalizedType>),
    /// Tipo desconocido (útil para error recovery)
    Unknown,
}

impl NormalizedType {
    pub fn from_type_reference(type_ref: &TypeReference) -> Self {
        Self::from_kind(&type_ref.kind)
    }

    pub fn from_kind(kind: &TypeReferenceKind) -> Self {
        match kind {
            TypeReferenceKind::Named(name) => match name.as_str() {
                "Number" => Self::Number,
                "String" => Self::String,
                "Boolean" => Self::Boolean,
                _ => Self::Named(name.clone()),
            },
            TypeReferenceKind::Iterable(type_ref) => {
                Self::Iterable(Box::new(Self::from_type_reference(type_ref)))
            }
            TypeReferenceKind::Vector(type_ref) => {
                Self::Vector(Box::new(Self::from_type_reference(type_ref)))
            }
        }
    }

    pub fn is_builtin(&self) -> bool {
        matches!(self, Self::Number | Self::String | Self::Boolean)
    }

    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }
}

impl std::fmt::Display for NormalizedType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NormalizedType::Number => write!(f, "Number"),
            NormalizedType::String => write!(f, "String"),
            NormalizedType::Boolean => write!(f, "Boolean"),
            NormalizedType::Named(name) => write!(f, "{}", name),
            NormalizedType::Iterable(inner) => write!(f, "{}*", inner),
            NormalizedType::Vector(inner) => write!(f, "{}[]", inner),
            NormalizedType::Unknown => write!(f, "?"),
        }
    }
}

/// Entorno de tipos: registro global de tipos y protocolos
pub struct TypeEnvironment {
    /// Tipos definidos por el usuario (excluye builtins)
    user_types: HashMap<String, TypeInfo>,
    /// Protocolos definidos
    protocols: HashMap<String, ProtocolInfo>,
}

impl TypeEnvironment {
    /// Crea un nuevo entorno de tipos vacío
    pub fn new() -> Self {
        Self {
            user_types: HashMap::new(),
            protocols: HashMap::new(),
        }
    }

    /// Registra un nuevo tipo en el entorno
    ///
    /// Retorna error si el tipo ya existe o si sus padres no existen.
    pub fn register_type(&mut self, type_info: TypeInfo) -> Result<(), SemanticError> {
        if self.user_types.contains_key(&type_info.name) {
            return Err(SemanticError::TypeAlreadyDeclared {
                name: type_info.name.clone(),
                first_line: 0,
                first_column: 0,
            });
        }

        if let Some(parent_name) = &type_info.parent {
            // Can't inherit from builtin or from protocol
            if Self::is_builtin_name(parent_name) {
                return Err(SemanticError::InheritFromUndeclared {
                    type_name: type_info.name.clone(),
                    parent_name: parent_name.clone(),
                });
            }

            if parent_name == &type_info.name {
                return Err(SemanticError::CircularInheritance {
                    type_name: type_info.name.clone(),
                    cycle: vec![type_info.name.clone(), parent_name.clone()],
                });
            }

            if !self.user_types.contains_key(parent_name) {
                return Err(SemanticError::InheritFromUndeclared {
                    type_name: type_info.name.clone(),
                    parent_name: parent_name.clone(),
                });
            }

            // Detect simple cycles by walking parents
            fn reaches_target(
                types: &HashMap<String, TypeInfo>,
                start: &str,
                target: &str,
                visited: &mut std::collections::HashSet<String>,
            ) -> bool {
                if start == target {
                    return true;
                }
                if visited.contains(start) {
                    return false;
                }
                visited.insert(start.to_string());
                if let Some(t) = types.get(start) {
                    if let Some(parent) = &t.parent {
                        if reaches_target(types, parent, target, visited) {
                            return true;
                        }
                    }
                }
                false
            }

            let mut visited = std::collections::HashSet::new();
            if reaches_target(&self.user_types, parent_name, &type_info.name, &mut visited) {
                return Err(SemanticError::CircularInheritance {
                    type_name: type_info.name.clone(),
                    cycle: vec![type_info.name.clone(), parent_name.clone()],
                });
            }
        }

        self.user_types.insert(type_info.name.clone(), type_info);
        Ok(())
    }

    /// Registra un protocolo en el entorno
    pub fn register_protocol(&mut self, protocol_info: ProtocolInfo) -> Result<(), SemanticError> {
        if self.protocols.contains_key(&protocol_info.name) {
            return Err(SemanticError::TypeAlreadyDeclared {
                name: protocol_info.name.clone(),
                first_line: 0,
                first_column: 0,
            });
        }

        // DFS to detect cycles in extends
        fn reaches_target(
            protocols: &HashMap<String, ProtocolInfo>,
            start: &str,
            target: &str,
            visited: &mut std::collections::HashSet<String>,
        ) -> bool {
            if start == target {
                return true;
            }
            if visited.contains(start) {
                return false;
            }
            visited.insert(start.to_string());
            if let Some(p) = protocols.get(start) {
                for e in &p.extends {
                    if reaches_target(protocols, e, target, visited) {
                        return true;
                    }
                }
            }
            false
        }

        for ext in &protocol_info.extends {
            if !self.protocols.contains_key(ext) {
                return Err(SemanticError::UndeclaredType { name: ext.clone() });
            }

            let mut visited = std::collections::HashSet::new();
            if reaches_target(&self.protocols, ext, &protocol_info.name, &mut visited) {
                return Err(SemanticError::CyclicDefinition {
                    names: vec![protocol_info.name.clone(), ext.clone()],
                });
            }
        }

        self.protocols
            .insert(protocol_info.name.clone(), protocol_info);
        Ok(())
    }

    /// Busca un tipo en el entorno (builtin o user-declared)
    pub fn get_type(&self, name: &str) -> Option<&TypeInfo> {
        self.user_types.get(name)
    }

    /// Comprueba si un nombre corresponde a un tipo builtin conocido
    pub fn is_builtin_name(name: &str) -> bool {
        matches!(name, "Number" | "String" | "Boolean")
    }

    /// Comprueba si el entorno conoce un tipo (builtin o definido por el usuario)
    pub fn has_type(&self, name: &str) -> bool {
        Self::is_builtin_name(name) || self.user_types.contains_key(name)
    }

    /// Busca un protocolo en el entorno
    pub fn get_protocol(&self, name: &str) -> Option<&ProtocolInfo> {
        self.protocols.get(name)
    }

    /// Verifica si dos tipos son iguales
    ///
    /// Considera:
    /// - Igualdad directa (mismo nombre)
    /// - Herencia (B hereda de A)
    /// - Conformancia a protocolo
    pub fn types_equal(&self, type1: &NormalizedType, type2: &NormalizedType) -> bool {
        match (type1, type2) {
            (NormalizedType::Number, NormalizedType::Number)
            | (NormalizedType::String, NormalizedType::String)
            | (NormalizedType::Boolean, NormalizedType::Boolean) => true,
            (NormalizedType::Named(a), NormalizedType::Named(b)) => a == b,
            (NormalizedType::Iterable(a), NormalizedType::Iterable(b)) => self.types_equal(a, b),
            (NormalizedType::Vector(a), NormalizedType::Vector(b)) => self.types_equal(a, b),
            (NormalizedType::Unknown, NormalizedType::Unknown) => true,
            _ => false,
        }
    }

    /// Verifica si type_a es compatible con type_b (puede asignarse type_a a type_b)
    ///
    /// Compatible si:
    /// - Son iguales
    /// - type_a hereda de type_b
    /// - type_a conforma a protocolo type_b
    pub fn is_compatible(&self, type_a: &NormalizedType, type_b: &NormalizedType) -> bool {
        // Igualdad directa y compatibilidad recursiva para contenedores
        if self.types_equal(type_a, type_b) {
            return true;
        }

        match (type_a, type_b) {
            // Vectores: T[] con U[] si T compatible con U
            (NormalizedType::Vector(a), NormalizedType::Vector(b)) => self.is_compatible(a, b),
            // Iterables: T* con U* si T compatible con U
            (NormalizedType::Iterable(a), NormalizedType::Iterable(b)) => self.is_compatible(a, b),
            // Named -> Named: check nominal subtyping
            (NormalizedType::Named(a), NormalizedType::Named(b)) => {
                self.is_subtype(a, b) || self.type_conforms_to_protocol(a, b)
            }
            _ => false,
        }
    }

    /// Verifica si un tipo conforma a un protocolo
    pub fn type_conforms_to_protocol(&self, type_name: &str, protocol_name: &str) -> bool {
        let protocol = match self.get_protocol(protocol_name) {
            Some(p) => p,
            None => return false,
        };

        if !self.has_type(type_name) {
            return false;
        }

        for proto_sig in &protocol.members {
            // Buscar método con el mismo nombre en la jerarquía
            let method_opt = self.get_method_from_hierarchy(type_name, &proto_sig.name);
            if method_opt.is_none() {
                return false;
            }

            let method = method_opt.unwrap();

            // Parámetros: el protocolo puede o no anotar los parámetros.
            if method.parameters.len() != proto_sig.parameters.len() {
                return false;
            }

            for (m_param, p_param) in method.parameters.iter().zip(proto_sig.parameters.iter()) {
                match (&m_param.annotation, &p_param.annotation) {
                    (_, None) => {
                        // protocolo no exige tipo para este parámetro -> ok
                    }
                    (Some(m_ann), Some(p_ann)) => {
                        let m_t = NormalizedType::from_type_reference(m_ann);
                        let p_t = NormalizedType::from_type_reference(p_ann);
                        if !self.is_compatible(&m_t, &p_t) {
                            return false;
                        }
                    }
                    (None, Some(_)) => {
                        // método no anotó pero protocolo sí -> no conforme
                        return false;
                    }
                }
            }

            // Return type: protocolo exige un return_type concreto
            if let Some(m_ret) = &method.return_type {
                let m_rt = NormalizedType::from_type_reference(m_ret);
                let p_rt = NormalizedType::from_type_reference(&proto_sig.return_type);
                if !self.is_compatible(&m_rt, &p_rt) {
                    return false;
                }
            } else {
                // método no anotó retorno pero protocolo sí -> no conforme
                return false;
            }
        }

        true
    }

    /// Busca un método llamado `method_name` en `type_name` o cualquiera de sus ancestros
    fn get_method_from_hierarchy(
        &self,
        type_name: &str,
        method_name: &str,
    ) -> Option<FunctionDeclaration> {
        let mut cur = type_name.to_string();
        while let Some(tinfo) = self.user_types.get(&cur) {
            for m in &tinfo.methods {
                if m.name == method_name {
                    return Some(m.clone());
                }
            }
            if let Some(parent) = &tinfo.parent {
                cur = parent.clone();
            } else {
                break;
            }
        }
        None
    }

    /// Obtiene el tipo padre (si existe) para herencia
    pub fn get_parent_type(&self, type_name: &str) -> Option<String> {
        self.user_types
            .get(type_name)
            .and_then(|t| t.parent.as_ref().map(|p| p.clone()))
    }

    /// Verifica si type_a es subtype de type_b (a hereda de b)
    pub fn is_subtype(&self, type_a: &str, type_b: &str) -> bool {
        if type_a == type_b {
            return true;
        }

        // If either side is a builtin, only exact equality counts
        if Self::is_builtin_name(type_a) || Self::is_builtin_name(type_b) {
            return type_a == type_b;
        }

        let mut cur = type_a.to_string();
        while let Some(parent) = self.get_parent_type(&cur) {
            if parent == type_b {
                return true;
            }
            cur = parent;
        }

        false
    }

    /// Devuelve la lista de ancestros (cadena de herencia) para `type_name`
    pub fn get_ancestors(&self, type_name: &str) -> Vec<String> {
        let mut res = Vec::new();
        let mut cur = type_name.to_string();
        while let Some(parent) = self.get_parent_type(&cur) {
            res.push(parent.clone());
            cur = parent;
        }
        res
    }

    /// Resolver el tipo `self` cuando estamos dentro de un método de `type_name`.
    /// Básicamente devuelve el tipo nominal correspondiente.
    pub fn resolve_self_type(&self, type_name: &str) -> NormalizedType {
        NormalizedType::Named(type_name.to_string())
    }

    /// Busca un método en los ancestros (excluyendo el propio `type_name`).
    /// Útil para `base.member` donde queremos obtener el miembro proveniente del padre.
    pub fn get_method_in_parent(
        &self,
        type_name: &str,
        method_name: &str,
    ) -> Option<FunctionDeclaration> {
        let mut cur = match self.get_parent_type(type_name) {
            Some(p) => p,
            None => return None,
        };

        while let Some(tinfo) = self.user_types.get(&cur) {
            for m in &tinfo.methods {
                if m.name == method_name {
                    return Some(m.clone());
                }
            }
            if let Some(parent) = &tinfo.parent {
                cur = parent.clone();
            } else {
                break;
            }
        }

        None
    }

    /// Lista todos los tipos registrados
    pub fn all_types(&self) -> Vec<&TypeInfo> {
        self.user_types.values().collect()
    }

    /// Lista todos los protocolos registrados
    pub fn all_protocols(&self) -> Vec<&ProtocolInfo> {
        self.protocols.values().collect()
    }

    /// Valida una `TypeReference` y devuelve su `NormalizedType` si es válida.
    ///
    /// Retorna error si la referencia nombra un tipo desconocido.
    pub fn validate_type_reference(
        &self,
        tr: &TypeReference,
    ) -> Result<NormalizedType, SemanticError> {
        match &tr.kind {
            TypeReferenceKind::Named(name) => {
                if self.has_type(name) {
                    Ok(NormalizedType::Named(name.clone()))
                } else {
                    Err(SemanticError::UndeclaredType { name: name.clone() })
                }
            }
            TypeReferenceKind::Iterable(inner) => {
                let inner_nt = self.validate_type_reference(inner)?;
                Ok(NormalizedType::Iterable(Box::new(inner_nt)))
            }
            TypeReferenceKind::Vector(inner) => {
                let inner_nt = self.validate_type_reference(inner)?;
                Ok(NormalizedType::Vector(Box::new(inner_nt)))
            }
        }
    }

    /// Valida las anotaciones de parámetros y retorno de una función.
    ///
    /// Retorna `Ok(())` si todas las anotaciones nombradas existen y son válidas.
    pub fn validate_function_signature(
        &self,
        func: &FunctionDeclaration,
    ) -> Result<(), SemanticError> {
        for p in &func.parameters {
            if let Some(ann) = &p.annotation {
                self.validate_type_reference(ann)?;
            }
        }

        if let Some(ret) = &func.return_type {
            self.validate_type_reference(ret)?;
        }

        Ok(())
    }

    /// Valida una declaración de tipo: su `inherits` y las anotaciones de atributos.
    /// Actualmente valida que el tipo padre (si existe) esté declarado y que
    /// las anotaciones de atributos referencien tipos válidos.
    pub fn validate_type_declaration(
        &self,
        td: &crate::parser::ast::TypeDeclaration,
    ) -> Result<(), SemanticError> {
        // Si hereda, validar que el tipo padre exista y no sea builtin
        if let Some(inherits) = &td.inherits {
            match &inherits.kind {
                TypeReferenceKind::Named(name) => {
                    if Self::is_builtin_name(name) {
                        return Err(SemanticError::InvalidConstructor {
                            type_name: td.name.clone(),
                            reason: format!("cannot inherit from builtin '{}'", name),
                        });
                    }
                    if !self.has_type(name) {
                        return Err(SemanticError::InheritFromUndeclared {
                            type_name: td.name.clone(),
                            parent_name: name.clone(),
                        });
                    }

                    // Validate parent_arguments arity against parent's parameters (if known)
                    if let Some(parent_info) = self.get_type(name) {
                        let expected = parent_info.parameters.len();
                        let provided = td.parent_arguments.len();
                        if expected != provided {
                            return Err(SemanticError::InvalidConstructor {
                                type_name: td.name.clone(),
                                reason: format!(
                                    "parent arguments arity mismatch: expected {}, got {}",
                                    expected, provided
                                ),
                            });
                        }
                    }
                }
                _ => {
                    // Herencia parametrizada/compuesta no soportada actualmente
                    return Err(SemanticError::UnsupportedFeature {
                        feature: "parametrized inherits".into(),
                    });
                }
            }
        }

        // Validar anotaciones en atributos
        for member in &td.members {
            if let crate::parser::ast::TypeMember::Attribute(attr) = member {
                if let Some(ann) = &attr.annotation {
                    self.validate_type_reference(ann)?;
                }
            }
        }

        Ok(())
    }
}

impl Default for TypeEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tests (Persona 2)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalized_type_builtin() {
        assert_eq!(NormalizedType::Number.to_string(), "Number");
        assert_eq!(NormalizedType::String.to_string(), "String");
        assert_eq!(NormalizedType::Boolean.to_string(), "Boolean");
    }

    #[test]
    fn test_normalized_type_iterable() {
        let iter_num = NormalizedType::Iterable(Box::new(NormalizedType::Number));
        assert_eq!(iter_num.to_string(), "Number*");
    }

    #[test]
    fn test_normalized_type_vector() {
        let vec_num = NormalizedType::Vector(Box::new(NormalizedType::Number));
        assert_eq!(vec_num.to_string(), "Number[]");
    }

    #[test]
    fn test_type_environment_new() {
        let env = TypeEnvironment::new();
        assert_eq!(env.all_types().len(), 0);
    }

    #[test]
    fn test_types_equal_builtins() {
        let env = TypeEnvironment::new();
        assert!(env.types_equal(&NormalizedType::Number, &NormalizedType::Number));
        assert!(!env.types_equal(&NormalizedType::Number, &NormalizedType::String));
    }

    #[test]
    fn test_is_compatible_subtype() {
        let mut env = TypeEnvironment::new();

        let type_a = TypeInfo {
            name: "A".into(),
            parameters: vec![],
            parent: None,
            methods: vec![],
            properties: vec![],
            implemented_protocols: vec![],
            span: Span::default(),
        };

        let type_b = TypeInfo {
            name: "B".into(),
            parameters: vec![],
            parent: Some("A".into()),
            methods: vec![],
            properties: vec![],
            implemented_protocols: vec![],
            span: Span::default(),
        };

        env.register_type(type_a).unwrap();
        env.register_type(type_b).unwrap();

        let t_b = NormalizedType::Named("B".into());
        let t_a = NormalizedType::Named("A".into());

        assert!(env.is_compatible(&t_b, &t_a));
        assert!(!env.is_compatible(&t_a, &t_b));
    }

    #[test]
    fn test_is_compatible_iterable() {
        let env = TypeEnvironment::new();
        let t_num_iter = NormalizedType::Iterable(Box::new(NormalizedType::Number));
        let t_num_iter2 = NormalizedType::Iterable(Box::new(NormalizedType::Number));
        assert!(env.is_compatible(&t_num_iter, &t_num_iter2));
    }

    #[test]
    fn test_protocol_conformance_positive() {
        let mut env = TypeEnvironment::new();

        // Protocol P { fn m() -> Number }
        let proto_sig = ProtocolMethodSignature {
            name: "m".into(),
            parameters: vec![],
            return_type: TypeReference::new("Number".into(), Span::default()),
            span: Span::default(),
        };

        let proto = ProtocolInfo {
            name: "P".into(),
            members: vec![proto_sig],
            extends: vec![],
            span: Span::default(),
        };

        env.register_protocol(proto).unwrap();

        // Type A { fn m() -> Number }
        let func = FunctionDeclaration {
            name: "m".into(),
            parameters: vec![],
            return_type: Some(TypeReference::new("Number".into(), Span::default())),
            body: Expr::literal(Literal::Number(0.0), Span::default()),
        };

        let type_a = TypeInfo {
            name: "A".into(),
            parameters: vec![],
            parent: None,
            methods: vec![func],
            properties: vec![],
            implemented_protocols: vec![],
            span: Span::default(),
        };

        env.register_type(type_a).unwrap();

        assert!(env.type_conforms_to_protocol("A", "P"));
    }

    #[test]
    fn test_protocol_conformance_negative_missing_method() {
        let mut env = TypeEnvironment::new();

        let proto_sig = ProtocolMethodSignature {
            name: "m".into(),
            parameters: vec![],
            return_type: TypeReference::new("Number".into(), Span::default()),
            span: Span::default(),
        };

        let proto = ProtocolInfo {
            name: "P2".into(),
            members: vec![proto_sig],
            extends: vec![],
            span: Span::default(),
        };

        env.register_protocol(proto).unwrap();

        let type_b = TypeInfo {
            name: "B".into(),
            parameters: vec![],
            parent: None,
            methods: vec![],
            properties: vec![],
            implemented_protocols: vec![],
            span: Span::default(),
        };

        env.register_type(type_b).unwrap();

        assert!(!env.type_conforms_to_protocol("B", "P2"));
    }

    #[test]
    fn test_self_and_base_helpers() {
        let mut env = TypeEnvironment::new();

        // A has method parent_m
        let parent_func = FunctionDeclaration {
            name: "parent_m".into(),
            parameters: vec![],
            return_type: Some(TypeReference::new("Number".into(), Span::default())),
            body: Expr::literal(Literal::Number(0.0), Span::default()),
        };

        let type_a = TypeInfo {
            name: "A".into(),
            parameters: vec![],
            parent: None,
            methods: vec![parent_func],
            properties: vec![],
            implemented_protocols: vec![],
            span: Span::default(),
        };

        // B inherits A
        let type_b = TypeInfo {
            name: "B".into(),
            parameters: vec![],
            parent: Some("A".into()),
            methods: vec![],
            properties: vec![],
            implemented_protocols: vec![],
            span: Span::default(),
        };

        env.register_type(type_a).unwrap();
        env.register_type(type_b).unwrap();

        // resolve_self_type for B
        assert_eq!(env.resolve_self_type("B").to_string(), "B");

        // get_method_from_hierarchy should find parent_m on B
        let m = env.get_method_from_hierarchy("B", "parent_m");
        assert!(m.is_some());

        // get_method_in_parent should also find it and indicate it's from parent
        let mp = env.get_method_in_parent("B", "parent_m");
        assert!(mp.is_some());
    }

    #[test]
    fn test_validate_type_reference_and_function_signature() {
        let mut env = TypeEnvironment::new();

        let type_a = TypeInfo {
            name: "A".into(),
            parameters: vec![],
            parent: None,
            methods: vec![],
            properties: vec![],
            implemented_protocols: vec![],
            span: Span::default(),
        };

        env.register_type(type_a).unwrap();

        // Valid named type
        let tr_a = TypeReference::new("A".into(), Span::default());
        assert!(env.validate_type_reference(&tr_a).is_ok());

        // Unknown type
        let tr_x = TypeReference::new("X".into(), Span::default());
        assert!(env.validate_type_reference(&tr_x).is_err());

        // Function signature valid
        let func = FunctionDeclaration {
            name: "f".into(),
            parameters: vec![crate::parser::ast::Parameter::new(
                "p".into(),
                Some(TypeReference::new("A".into(), Span::default())),
                Span::default(),
            )],
            return_type: Some(TypeReference::new("Number".into(), Span::default())),
            body: Expr::literal(Literal::Number(0.0), Span::default()),
        };

        assert!(env.validate_function_signature(&func).is_ok());

        // Function signature invalid (unknown param type)
        let func2 = FunctionDeclaration {
            name: "g".into(),
            parameters: vec![crate::parser::ast::Parameter::new(
                "p".into(),
                Some(TypeReference::new("X".into(), Span::default())),
                Span::default(),
            )],
            return_type: None,
            body: Expr::literal(Literal::Number(0.0), Span::default()),
        };

        assert!(env.validate_function_signature(&func2).is_err());
    }

    #[test]
    fn test_validate_type_declaration() {
        let mut env = TypeEnvironment::new();

        let type_a = TypeInfo {
            name: "A".into(),
            parameters: vec![],
            parent: None,
            methods: vec![],
            properties: vec![],
            implemented_protocols: vec![],
            span: Span::default(),
        };

        env.register_type(type_a).unwrap();

        // Type declaration that inherits A and has an attribute annotated A
        let td = crate::parser::ast::TypeDeclaration {
            name: "B".into(),
            parameters: vec![],
            inherits: Some(TypeReference::new("A".into(), Span::default())),
            parent_arguments: vec![],
            members: vec![crate::parser::ast::TypeMember::Attribute(
                crate::parser::ast::AttributeDeclaration {
                    name: "x".into(),
                    annotation: Some(TypeReference::new("A".into(), Span::default())),
                    initializer: Expr::literal(Literal::Number(0.0), Span::default()),
                    span: Span::default(),
                },
            )],
        };

        assert!(env.validate_type_declaration(&td).is_ok());

        // Invalid: inherits unknown type
        let td2 = crate::parser::ast::TypeDeclaration {
            name: "C".into(),
            parameters: vec![],
            inherits: Some(TypeReference::new("X".into(), Span::default())),
            parent_arguments: vec![],
            members: vec![],
        };

        assert!(env.validate_type_declaration(&td2).is_err());

        // Parent arguments arity mismatch
        let type_p = TypeInfo {
            name: "P".into(),
            parameters: vec![crate::parser::ast::Parameter::new(
                "x".into(),
                Some(TypeReference::new("Number".into(), Span::default())),
                Span::default(),
            )],
            parent: None,
            methods: vec![],
            properties: vec![],
            implemented_protocols: vec![],
            span: Span::default(),
        };

        env.register_type(type_p).unwrap();

        let td3 = crate::parser::ast::TypeDeclaration {
            name: "Q".into(),
            parameters: vec![],
            inherits: Some(TypeReference::new("P".into(), Span::default())),
            parent_arguments: vec![], // missing one arg
            members: vec![],
        };

        assert!(env.validate_type_declaration(&td3).is_err());
    }
}
