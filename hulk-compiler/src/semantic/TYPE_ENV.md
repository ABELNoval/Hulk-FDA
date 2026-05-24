TypeEnvironment — documentación y ejemplos
=========================================

Resumen
------
`TypeEnvironment` es el registro central de tipos y protocolos del compilador.
Se encarga de:

- Registrar tipos definidos por el usuario (`register_type`).
- Registrar protocolos (`register_protocol`) y resolver `extends`.
- Resolver referencias a tipos (`validate_type_reference`).
- Preguntar si dos tipos son compatibles (`is_compatible`).
- Verificar conformidad de un tipo a un protocolo (`type_conforms_to_protocol`).

Errores comunes
---------------
- `SemanticError::TypeAlreadyDeclared`: intentar registrar un tipo/protocolo ya existente.
- `SemanticError::InheritFromUndeclared`: tipo declara un `inherits` apuntando a un nombre no registrado.
- `SemanticError::CircularInheritance`: intento evidente de herencia a sí mismo.
- `SemanticError::UndeclaredType`: referencia a tipo desconocido desde una `TypeReference`.
- `SemanticError::CyclicDefinition`: cuando la cadena de `extends` en protocolos forma un ciclo.

Ejemplos rápidos
----------------
```rust
use crate::semantic::type_system::*;
use crate::parser::ast::{TypeReference, Parameter};
use crate::utils::errors::span::Span;

let mut env = TypeEnvironment::new();

// Registrar un tipo simple
let t = TypeInfo {
    name: "A".into(),
    parameters: vec![],
    parent: None,
    methods: vec![],
    properties: vec![],
    implemented_protocols: vec![],
    span: Span::default(),
};
env.register_type(t).unwrap();

// Registrar herencia B -> A
let b = TypeInfo {
    name: "B".into(),
    parameters: vec![],
    parent: Some("A".into()),
    methods: vec![],
    properties: vec![],
    implemented_protocols: vec![],
    span: Span::default(),
};
env.register_type(b).unwrap();

// Compatibilidad nominal: B es compatible con A
assert!(env.is_compatible(&NormalizedType::Named("B".into()), &NormalizedType::Named("A".into())));

// Validar TypeReference
let tr = TypeReference::new("A".into(), Span::default());
let nt = env.validate_type_reference(&tr).unwrap();
```

Notas
-----
- Los tipos built-in (`Number`, `String`, `Boolean`) están implícitos y no deben registrarse.
- `register_protocol` exige que los protocolos referenciados en `extends` ya estén registrados.
- La verificación de conformidad a protocolos revisa signatures de métodos en la jerarquía del tipo.

Si necesitas más ejemplos o documentación en formato `rustdoc` dentro del código, puedo incorporarlos directamente en `type_system.rs`.
