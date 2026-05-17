// =============================================================================
// Expression Type Checking
// =============================================================================
//
// Responsabilidad de Persona 3:
// - Type checking de literales (Number, String, Boolean)
// - Type checking de operaciones binarias (arithmetic, comparison, logical)
// - Type checking de llamadas a funciones
// - Type checking de constructores (new)
// - Type checking de if/while/for/let/blocks
// - Type checking de operaciones de tipo (is, as)
// - Deducción de tipos donde sea posible
//
// Esta es la interfaz compartida. Los métodos son stubs que Persona 3 implementará.
//
// =============================================================================

use crate::parser::ast::{BinaryOperator, Expr, ExprKind, Literal};
use crate::semantic::type_system::NormalizedType;
use crate::utils::errors::semantic::SemanticError;
use crate::utils::errors::span::Span;

type SemanticResult<T> = Result<T, SemanticError>;

/// Resultado del type checking de una expresión
#[derive(Debug, Clone)]
pub struct ExpressionType {
    /// El tipo resultante
    pub type_: NormalizedType,
    /// Si la expresión es un lvalue (puede asignársele)
    pub is_lvalue: bool,
}

impl ExpressionType {
    pub fn new(type_: NormalizedType, is_lvalue: bool) -> Self {
        Self { type_, is_lvalue }
    }

    pub fn value(type_: NormalizedType) -> Self {
        Self::new(type_, false)
    }

    pub fn lvalue(type_: NormalizedType) -> Self {
        Self::new(type_, true)
    }
}

/// Checker de tipos de expresiones
pub struct ExpressionChecker;

impl ExpressionChecker {
    /// Crea un nuevo checker
    pub fn new() -> Self {
        Self
    }

    /// Determina el tipo de una expresión literal
    ///
    /// - Número: Number
    /// - String: String
    /// - true/false: Boolean
    pub fn check_literal(&self, expr: &Expr) -> SemanticResult<ExpressionType> {
        match &expr.kind {
            ExprKind::Literal(Literal::Number(_)) | ExprKind::Literal(Literal::Pi) | ExprKind::Literal(Literal::E) => {
                Ok(ExpressionType::value(NormalizedType::Number))
            }
            ExprKind::Literal(Literal::String(_)) => Ok(ExpressionType::value(NormalizedType::String)),
            ExprKind::Literal(Literal::Boolean(_)) => Ok(ExpressionType::value(NormalizedType::Boolean)),
            _ => Err(SemanticError::UnsupportedFeature {
                feature: "Expected literal expression".to_string(),
            }),
        }
    }

    /// Verifica tipos en una operación binaria
    ///
    /// - Aritmética (+, -, *, /, %, ^): Number OP Number -> Number
    /// - Concatenación (@, @@): String OP String -> String
    /// - Comparación (==, !=, <, <=, >, >=): T OP T -> Boolean
    /// - Lógica (&&, ||): Boolean OP Boolean -> Boolean
    pub fn check_binary_op(
        &self,
        left_type: &NormalizedType,
        op: &BinaryOperator,
        right_type: &NormalizedType,
        _span: &Span,
    ) -> SemanticResult<ExpressionType> {
        // TODO: Implementar validación de tipos para cada operador
        // - Validar que los operandos son compatibles
        // - Retornar el tipo resultante
        // - Reportar errors específicos (ej: "Cannot add String to Number")

        let result_type = match op {
            BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide
            | BinaryOperator::Modulo
            | BinaryOperator::Power => {
                // Aritmética: requiere Number
                if left_type == &NormalizedType::Number && right_type == &NormalizedType::Number {
                    NormalizedType::Number
                } else {
                    return Err(SemanticError::InvalidBinaryOperator {
                        operator: format!("{:?}", op),
                        left_type: left_type.to_string(),
                        right_type: right_type.to_string(),
                    });
                }
            }
            BinaryOperator::Concat | BinaryOperator::Concatenate => {
                // Concatenación: requiere String
                if left_type == &NormalizedType::String && right_type == &NormalizedType::String {
                    NormalizedType::String
                } else {
                    return Err(SemanticError::InvalidBinaryOperator {
                        operator: format!("{:?}", op),
                        left_type: left_type.to_string(),
                        right_type: right_type.to_string(),
                    });
                }
            }
            BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::Less
            | BinaryOperator::LessEqual
            | BinaryOperator::Greater
            | BinaryOperator::GreaterEqual => {
                // Comparación: requiere tipos iguales
                if left_type == right_type {
                    NormalizedType::Boolean
                } else {
                    return Err(SemanticError::IncomparableTypes {
                        left_type: left_type.to_string(),
                        right_type: right_type.to_string(),
                    });
                }
            }
            BinaryOperator::And | BinaryOperator::Or => {
                // Lógica: requiere Boolean
                if left_type == &NormalizedType::Boolean && right_type == &NormalizedType::Boolean {
                    NormalizedType::Boolean
                } else {
                    return Err(SemanticError::InvalidBinaryOperator {
                        operator: format!("{:?}", op),
                        left_type: left_type.to_string(),
                        right_type: right_type.to_string(),
                    });
                }
            }
        };

        Ok(ExpressionType::value(result_type))
    }

    /// Verifica tipo de operación unaria
    ///
    /// - Negación (-): Number -> Number
    /// - NOT (!): Boolean -> Boolean
    pub fn check_unary_op(
        &self,
        operand_type: &NormalizedType,
        is_negation: bool,
        _span: &Span,
    ) -> SemanticResult<ExpressionType> {
        // TODO: Implementar validación de operadores unarios
        if is_negation {
            if operand_type == &NormalizedType::Number {
                Ok(ExpressionType::value(NormalizedType::Number))
            } else {
                Err(SemanticError::InvalidUnaryOperator {
                    operator: "-".to_string(),
                    operand_type: operand_type.to_string(),
                })
            }
        } else {
            // NOT operator
            if operand_type == &NormalizedType::Boolean {
                Ok(ExpressionType::value(NormalizedType::Boolean))
            } else {
                Err(SemanticError::InvalidUnaryOperator {
                    operator: "!".to_string(),
                    operand_type: operand_type.to_string(),
                })
            }
        }
    }

    /// Verifica tipo de una expresión agrupada (parentesis)
    /// 
    /// Simplemente retorna el mismo tipo de la expresión interna
    pub fn check_grouping(&self, inner_type: &NormalizedType) -> SemanticResult<ExpressionType> {
        Ok(ExpressionType::value(inner_type.clone()))
    }

    /// Verifica tipo de una llamada a función
    ///
    /// - Valida número de argumentos
    /// - Valida tipos de argumentos contra parámetros
    /// - Retorna tipo de retorno de la función
    pub fn check_function_call(
        &self,
        function_name: &str,
        argument_types: &[NormalizedType],
        expected_param_count: usize,
        _span: &Span,
    ) -> SemanticResult<ExpressionType> {
        // TODO: Implementar type checking de llamadas a función
        // - Validar número de argumentos
        // - Validar tipos de argumentos
        // - Retornar tipo de retorno (deducir del contexto si es necesario)

        if argument_types.len() != expected_param_count {
            return Err(SemanticError::WrongArgumentCount {
                function: function_name.to_string(),
                expected: expected_param_count,
                found: argument_types.len(),
            });
        }

        // TODO: Validar tipos de argumentos contra parámetros declarados
        // Por ahora retorna Unknown
        Ok(ExpressionType::value(NormalizedType::Unknown))
    }

    /// Verifica tipo de una expresión condicional (if)
    ///
    /// - Condición debe ser Boolean (aplicable también a los elif)
    /// - Rama then, elifs y else deben tener el mismo tipo (o compatible)
    pub fn check_if_expression(
        &self,
        condition_type: &NormalizedType,
        then_type: &NormalizedType,
        elif_branches: &[(NormalizedType, NormalizedType)],
        else_type: Option<&NormalizedType>,
        _span: &Span,
    ) -> SemanticResult<ExpressionType> {
        // Validar condition es Boolean
        if condition_type != &NormalizedType::Boolean {
            return Err(SemanticError::NonBooleanCondition {
                found_type: condition_type.to_string(),
                context: "if".to_string(),
            });
        }

        // Validar elif branches
        for (elif_cond_type, elif_body_type) in elif_branches {
            if elif_cond_type != &NormalizedType::Boolean {
                return Err(SemanticError::NonBooleanCondition {
                    found_type: elif_cond_type.to_string(),
                    context: "elif".to_string(),
                });
            }

            if then_type != elif_body_type {
                return Err(SemanticError::IncompatibleBranchTypes {
                    then_type: then_type.to_string(),
                    else_type: elif_body_type.to_string(),
                });
            }
        }

        // Validar else branch
        if let Some(else_type) = else_type {
            if then_type != else_type {
                return Err(SemanticError::IncompatibleBranchTypes {
                    then_type: then_type.to_string(),
                    else_type: else_type.to_string(),
                });
            }
        }

        Ok(ExpressionType::value(then_type.clone()))
    }

    /// Verifica tipo de un bloque de expresiones
    ///
    /// El tipo es el tipo de la última expresión
    pub fn check_block(&self, expr_types: &[NormalizedType]) -> SemanticResult<ExpressionType> {
        if let Some(last_expr_type) = expr_types.last() {
            Ok(ExpressionType::value(last_expr_type.clone()))
        } else {
            // Si el bloque está vacío, asume Unknown o void equivalente.
            Ok(ExpressionType::value(NormalizedType::Unknown))
        }
    }

    /// Verifica tipo de una expresión while
    ///
    /// - Condición debe ser Boolean
    pub fn check_while_expression(
        &self,
        condition_type: &NormalizedType,
        body_type: &NormalizedType,
        _span: &Span,
    ) -> SemanticResult<ExpressionType> {
        if condition_type != &NormalizedType::Boolean {
            return Err(SemanticError::NonBooleanCondition {
                found_type: condition_type.to_string(),
                context: "while".to_string(),
            });
        }
        
        Ok(ExpressionType::value(body_type.clone()))
    }

    /// Verifica tipo de una expresión for
    ///
    /// - Evalúa la parte del iterable 
    pub fn check_for_expression(
        &self,
        _iterable_type: &NormalizedType,
        body_type: &NormalizedType,
        _span: &Span,
    ) -> SemanticResult<ExpressionType> {
        // TODO: Validar si _iterable_type implementa protocolo Iterable
        
        Ok(ExpressionType::value(body_type.clone()))
    }

    /// Verifica tipo de constructor (new)
    ///
    /// - El tipo debe estar definido
    /// - Validar argumentos contra constructor si existe
    pub fn check_new(
        &self,
        type_name: &str,
        _argument_types: &[NormalizedType],
        _span: &Span,
    ) -> SemanticResult<ExpressionType> {
        // TODO: Implementar type checking de new
        // - Validar que el tipo existe
        // - Validar argumentos al constructor
        Ok(ExpressionType::value(NormalizedType::Named(
            type_name.to_string(),
        )))
    }

    /// Verifica tipo de operación is (type check)
    ///
    /// - Retorna Boolean
    pub fn check_is(
        &self,
        _expr_type: &NormalizedType,
        _target_type: &NormalizedType,
        _span: &Span,
    ) -> SemanticResult<ExpressionType> {
        // TODO: Validar que expr_type e target_type son válidos para is
        Ok(ExpressionType::value(NormalizedType::Boolean))
    }

    /// Verifica tipo de operación as (type cast)
    ///
    /// - Retorna el tipo destino
    pub fn check_as(
        &self,
        _expr_type: &NormalizedType,
        target_type: &NormalizedType,
        _span: &Span,
    ) -> SemanticResult<ExpressionType> {
        // TODO: Validar que el cast es válido (upcast o downcast permitido)
        Ok(ExpressionType::value(target_type.clone()))
    }
}

impl Default for ExpressionChecker {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tests (Persona 3)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expression_type_value() {
        let et = ExpressionType::value(NormalizedType::Number);
        assert_eq!(et.type_, NormalizedType::Number);
        assert!(!et.is_lvalue);
    }

    #[test]
    fn test_expression_type_lvalue() {
        let et = ExpressionType::lvalue(NormalizedType::String);
        assert_eq!(et.type_, NormalizedType::String);
        assert!(et.is_lvalue);
    }

    #[test]
    fn test_checker_new() {
        let _checker = ExpressionChecker::new();
    }

    #[test]
    fn test_if_expression_valid() {
        let checker = ExpressionChecker::new();
        // if (Boolean) { Number } elif (Boolean) { Number } else { Number }
        let elif_branches = vec![
            (NormalizedType::Boolean, NormalizedType::Number),
            (NormalizedType::Boolean, NormalizedType::Number)
        ];
        let result = checker.check_if_expression(
            &NormalizedType::Boolean,
            &NormalizedType::Number,
            &elif_branches,
            Some(&NormalizedType::Number),
            &Span::default()
        ).unwrap();
        
        assert_eq!(result.type_, NormalizedType::Number);
    }

    #[test]
    fn test_if_expression_invalid_condition() {
        let checker = ExpressionChecker::new();
        // if (Number) { String }
        let elif_branches = vec![];
        let result = checker.check_if_expression(
            &NormalizedType::Number,
            &NormalizedType::String,
            &elif_branches,
            None,
            &Span::default()
        );
        
        assert!(matches!(result, Err(SemanticError::NonBooleanCondition { .. })));
    }

    #[test]
    fn test_if_expression_invalid_elif_condition() {
        let checker = ExpressionChecker::new();
        // if (Boolean) { String } elif (Number) { String }
        let elif_branches = vec![
            (NormalizedType::Number, NormalizedType::String)
        ];
        let result = checker.check_if_expression(
            &NormalizedType::Boolean,
            &NormalizedType::String,
            &elif_branches,
            None,
            &Span::default()
        );
        
        assert!(matches!(result, Err(SemanticError::NonBooleanCondition { .. })));
    }

    #[test]
    fn test_if_expression_incompatible_elif_branch() {
        let checker = ExpressionChecker::new();
        // if (Boolean) { String } elif (Boolean) { Number }
        let elif_branches = vec![
            (NormalizedType::Boolean, NormalizedType::Number)
        ];
        let result = checker.check_if_expression(
            &NormalizedType::Boolean,
            &NormalizedType::String,
            &elif_branches,
            None,
            &Span::default()
        );
        
        assert!(matches!(result, Err(SemanticError::IncompatibleBranchTypes { .. })));
    }

    #[test]
    fn test_if_expression_incompatible_else_branch() {
        let checker = ExpressionChecker::new();
        // if (Boolean) { String } else { Number }
        let elif_branches = vec![];
        let result = checker.check_if_expression(
            &NormalizedType::Boolean,
            &NormalizedType::String,
            &elif_branches,
            Some(&NormalizedType::Number),
            &Span::default()
        );
        
        assert!(matches!(result, Err(SemanticError::IncompatibleBranchTypes { .. })));
    }

    #[test]
    fn test_while_expression_valid() {
        let checker = ExpressionChecker::new();
        let result = checker.check_while_expression(
            &NormalizedType::Boolean,
            &NormalizedType::Number,
            &Span::default()
        ).unwrap();
        assert_eq!(result.type_, NormalizedType::Number);
    }

    #[test]
    fn test_while_expression_invalid_condition() {
        let checker = ExpressionChecker::new();
        let result = checker.check_while_expression(
            &NormalizedType::Number,
            &NormalizedType::String,
            &Span::default()
        );
        assert!(matches!(result, Err(SemanticError::NonBooleanCondition { .. })));
    }

    #[test]
    fn test_for_expression_valid() {
        let checker = ExpressionChecker::new();
        let result = checker.check_for_expression(
            &NormalizedType::Unknown,
            &NormalizedType::Number,
            &Span::default()
        ).unwrap();
        assert_eq!(result.type_, NormalizedType::Number);
    }

    #[test]
    fn test_block_valid_elements() {
        let checker = ExpressionChecker::new();
        let elements = vec![
            NormalizedType::Number,
            NormalizedType::String,
            NormalizedType::Boolean
        ];
        let result = checker.check_block(&elements).unwrap();
        // The block should return the type of the last element
        assert_eq!(result.type_, NormalizedType::Boolean);
    }

    #[test]
    fn test_block_empty() {
        let checker = ExpressionChecker::new();
        let elements = vec![];
        let result = checker.check_block(&elements).unwrap();
        // Un bloque vacío asume tipo Unknown
        assert_eq!(result.type_, NormalizedType::Unknown);
    }
}
