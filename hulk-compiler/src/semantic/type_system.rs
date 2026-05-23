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
use crate::utils::errors::span::Span;
use std::collections::HashMap;

/// Información sobre un tipo declarado
#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub name: String,
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
    pub fn register_type(&mut self, type_info: TypeInfo) -> Result<(), String> {
        if self.user_types.contains_key(&type_info.name) {
            return Err(format!("Type '{}' already defined", type_info.name));
        }
        // Validaciones iniciales:
        // - Si tiene parent, el parent debe existir y no ser builtin
        // - No permitir herencia circular simple
        if let Some(parent_name) = &type_info.parent {
            if Self::is_builtin_name(parent_name) {
                return Err(format!(
                    "Type '{}' cannot inherit from builtin '{}'",
                    type_info.name, parent_name
                ));
            }

            if parent_name == &type_info.name {
                return Err(format!(
                    "Type '{}' cannot inherit from itself",
                    type_info.name
                ));
            }

            if !self.has_type(parent_name) {
                return Err(format!(
                    "Parent type '{}' for '{}' not found",
                    parent_name, type_info.name
                ));
            }

            // Detect simple cycles: walk up the parent chain and ensure we don't encounter the new type name
            let mut cur = parent_name.clone();
            while let Some(next_parent) = self.get_parent_type(&cur) {
                if next_parent == type_info.name {
                    return Err(format!(
                        "Inheritance cycle detected involving '{}' and '{}'",
                        type_info.name, cur
                    ));
                }
                cur = next_parent;
            }
        }

        self.user_types.insert(type_info.name.clone(), type_info);
        Ok(())
    }

    /// Registra un nuevo protocolo en el entorno
    ///
    /// Retorna error si el protocolo ya existe.
    pub fn register_protocol(&mut self, protocol_info: ProtocolInfo) -> Result<(), String> {
        if self.protocols.contains_key(&protocol_info.name) {
            return Err(format!("Protocol '{}' already defined", protocol_info.name));
        }

        self.protocols
            .insert(protocol_info.name.clone(), protocol_info);
        Ok(())
    }

    /// Busca un tipo en el entorno
    pub fn get_type(&self, name: &str) -> Option<&TypeInfo> {
        // Builtins are not stored in `user_types` but we consider their names valid.
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
            // Nominal subtyping: Named(a) es compatible con Named(b) si a <: b
            (NormalizedType::Named(a), NormalizedType::Named(b)) => self.is_subtype(a, b),
            // Contenedores: T* con U* si T compatible con U
            (NormalizedType::Iterable(a), NormalizedType::Iterable(b)) => self.is_compatible(a, b),
            // Vectores: T[] con U[] si T compatible con U
            (NormalizedType::Vector(a), NormalizedType::Vector(b)) => self.is_compatible(a, b),
            _ => false,
        }
    }

    /// Verifica si un tipo conforma a un protocolo
    pub fn type_conforms_to_protocol(&self, _type_name: &str, _protocol_name: &str) -> bool {
        // Implementación básica:
        // - Buscar el protocolo
        // - Para cada método requerido por el protocolo, buscar un método con el mismo nombre
        //   en el tipo o en sus ancestros
        // - Comparar número de parámetros y tipos (si la firma del protocolo tiene anotaciones)
        // - Comparar tipo de retorno

        let protocol = match self.get_protocol(_protocol_name) {
            Some(p) => p,
            None => return false,
        };

        let cur_type_name = _type_name.to_string();

        if !self.has_type(&cur_type_name) {
            return false;
        }

        for proto_sig in &protocol.members {
            // Buscar método con el mismo nombre en la jerarquía
            let method_opt = self.get_method_from_hierarchy(&cur_type_name, &proto_sig.name);
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
            parent: None,
            methods: vec![],
            properties: vec![],
            implemented_protocols: vec![],
            span: Span::default(),
        };

        let type_b = TypeInfo {
            name: "B".into(),
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
            span: Span::default(),
        };

        env.register_protocol(proto).unwrap();

        let type_b = TypeInfo {
            name: "B".into(),
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
            parent: None,
            methods: vec![parent_func],
            properties: vec![],
            implemented_protocols: vec![],
            span: Span::default(),
        };

        // B inherits A
        let type_b = TypeInfo {
            name: "B".into(),
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
}
