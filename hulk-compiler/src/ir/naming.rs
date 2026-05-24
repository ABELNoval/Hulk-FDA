// Persona 1 — SSA IR design and core representation
// Assigned: Persona1
// Responsibilities: definir convenciones de nombres para valores SSA, bloques
// y módulos. Estas reglas facilitan la lectura, debug y comparación en tests.

pub struct IRNaming;

impl IRNaming {
    pub const SSA_VALUE_PREFIX: &'static str = "%t";
    pub const BASIC_BLOCK_PREFIX: &'static str = "bb";
    pub const MODULE_PREFIX: &'static str = "module";

    pub fn module_name(name: impl AsRef<str>) -> String {
        format!("{}::{}", Self::MODULE_PREFIX, name.as_ref())
    }

    pub fn function_name(name: impl AsRef<str>) -> String {
        name.as_ref().to_string()
    }

    pub fn basic_block_name(index: usize) -> String {
        format!("{}{}", Self::BASIC_BLOCK_PREFIX, index)
    }

    pub fn temporary_name(index: usize) -> String {
        format!("{}{}", Self::SSA_VALUE_PREFIX, index)
    }

    pub fn parameter_name(index: usize) -> String {
        format!("%p{}", index)
    }
}
