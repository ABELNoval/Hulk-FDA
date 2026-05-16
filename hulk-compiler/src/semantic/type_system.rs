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

use crate::parser::ast::{FunctionDeclaration, ProtocolMethodSignature, TypeReference, TypeReferenceKind};
use crate::utils::errors::span::Span;
use std::collections::HashMap;

/// Información sobre un tipo declarado
#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub name: String,
    pub parent: Option<Box<TypeInfo>>,
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

        // TODO: Validar que el parent existe (si hay)
        // TODO: Validar que no hay herencia circular
        // TODO: Validar que no hereda de builtin

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
        // TODO: Considerar builtins también
        self.user_types.get(name)
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
        // TODO: Implementar comparación semántica de tipos
        // Considera: builtins, herencia, protocolos
        type1 == type2
    }

    /// Verifica si type_a es compatible con type_b (puede asignarse type_a a type_b)
    ///
    /// Compatible si:
    /// - Son iguales
    /// - type_a hereda de type_b
    /// - type_a conforma a protocolo type_b
    pub fn is_compatible(&self, type_a: &NormalizedType, type_b: &NormalizedType) -> bool {
        // TODO: Implementar con consideración de herencia y protocolos
        self.types_equal(type_a, type_b)
    }

    /// Verifica si un tipo conforma a un protocolo
    pub fn type_conforms_to_protocol(
        &self,
        _type_name: &str,
        _protocol_name: &str,
    ) -> bool {
        // TODO: Implementar conformancia
        // - El tipo debe implementar todos los métodos del protocolo
        // - Con las firmas correctas
        false
    }

    /// Obtiene el tipo padre (si existe) para herencia
    pub fn get_parent_type(&self, type_name: &str) -> Option<String> {
        self.user_types
            .get(type_name)
            .and_then(|t| t.parent.as_ref().map(|p| p.name.clone()))
    }

    /// Verifica si type_a es subtype de type_b (a hereda de b)
    pub fn is_subtype(&self, type_a: &str, type_b: &str) -> bool {
        // TODO: Implementar búsqueda en cadena de herencia
        if type_a == type_b {
            return true;
        }

        if let Some(parent) = self.get_parent_type(type_a) {
            self.is_subtype(&parent, type_b)
        } else {
            false
        }
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
}
