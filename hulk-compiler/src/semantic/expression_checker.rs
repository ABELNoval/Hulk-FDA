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
        expected_params: Option<&[(String, NormalizedType)]>,
        expected_return_type: Option<&NormalizedType>,
        _span: &Span,
    ) -> SemanticResult<ExpressionType> {
        // Built-in functions
        match function_name {
            "print" => {
                if argument_types.len() != 1 {
                    return Err(SemanticError::WrongArgumentCount {
                        function: function_name.to_string(),
                        expected: 1,
                        found: argument_types.len(),
                    });
                }
                // El print asume devolver el mismo tipo del argumento o Unknown
                return Ok(ExpressionType::value(argument_types[0].clone()));
            }
            "sqrt" | "sin" | "cos" | "exp" => {
                if argument_types.len() != 1 {
                    return Err(SemanticError::WrongArgumentCount {
                        function: function_name.to_string(),
                        expected: 1,
                        found: argument_types.len(),
                    });
                }
                if argument_types[0] != NormalizedType::Number && argument_types[0] != NormalizedType::Unknown {
                    return Err(SemanticError::ArgumentTypeMismatch {
                        function: function_name.to_string(),
                        parameter_name: "value".to_string(),
                        parameter_position: 0,
                        expected: NormalizedType::Number.to_string(),
                        found: argument_types[0].to_string(),
                    });
                }
                return Ok(ExpressionType::value(NormalizedType::Number));
            }
            "log" => {
                if argument_types.len() != 2 {
                    return Err(SemanticError::WrongArgumentCount {
                        function: function_name.to_string(),
                        expected: 2,
                        found: argument_types.len(),
                    });
                }
                if argument_types[0] != NormalizedType::Number && argument_types[0] != NormalizedType::Unknown {
                    return Err(SemanticError::ArgumentTypeMismatch {
                        function: function_name.to_string(),
                        parameter_name: "base".to_string(),
                        parameter_position: 0,
                        expected: NormalizedType::Number.to_string(),
                        found: argument_types[0].to_string(),
                    });
                }
                if argument_types[1] != NormalizedType::Number && argument_types[1] != NormalizedType::Unknown {
                    return Err(SemanticError::ArgumentTypeMismatch {
                        function: function_name.to_string(),
                        parameter_name: "value".to_string(),
                        parameter_position: 1,
                        expected: NormalizedType::Number.to_string(),
                        found: argument_types[1].to_string(),
                    });
                }
                return Ok(ExpressionType::value(NormalizedType::Number));
            }
            _ => {}
        }

        // Custom functions
        if let Some(params) = expected_params {
            if argument_types.len() != params.len() {
                return Err(SemanticError::WrongArgumentCount {
                    function: function_name.to_string(),
                    expected: params.len(),
                    found: argument_types.len(),
                });
            }

            for (i, (arg_type, (param_name, param_type))) in argument_types.iter().zip(params.iter()).enumerate() {
                if arg_type != param_type && arg_type != &NormalizedType::Unknown && param_type != &NormalizedType::Unknown {
                    return Err(SemanticError::ArgumentTypeMismatch {
                        function: function_name.to_string(),
                        parameter_name: param_name.to_string(),
                        parameter_position: i,
                        expected: param_type.to_string(),
                        found: arg_type.to_string(),
                    });
                }
            }
        }

        let ret_type = expected_return_type.cloned().unwrap_or(NormalizedType::Unknown);
        Ok(ExpressionType::value(ret_type))
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

    /// Verifica tipo de una expresión let (declaración de variable)
    ///
    /// - Evalúa la relación entre la anotación explícita y el tipo del valor inferido
    pub fn check_let_expression(
        &self,
        annotation_type: Option<&NormalizedType>,
        value_type: Option<&NormalizedType>,
        _span: &Span,
    ) -> SemanticResult<ExpressionType> {
        if let (Some(expected), Some(found)) = (annotation_type, value_type) {
            if expected != found {
                return Err(SemanticError::TypeMismatch {
                    expected: expected.to_string(),
                    found: found.to_string(),
                    context: "asignación en let".to_string(),
                });
            }
        }

        // El tipo de retorno de let como expresión per se puede ser el tipo declarado o inferido
        let resulting_type = annotation_type
            .or(value_type)
            .cloned()
            .unwrap_or(NormalizedType::Unknown);
            
        Ok(ExpressionType::value(resulting_type))
    }

    /// Verifica tipo de una expresión de asignación
    ///
    /// - Verifica que el target sea un lvalue válido (asignable)
    /// - Evalúa la compatibilidad de tipo entre el target y el value
    pub fn check_assignment_expression(
        &self,
        target_type: &ExpressionType,
        value_type: &NormalizedType,
        _span: &Span,
    ) -> SemanticResult<ExpressionType> {
        if !target_type.is_lvalue {
            return Err(SemanticError::AssignmentToImmutable {
                name: "expresión no asignable (no es un lvalue)".to_string(),
            });
        }

        if target_type.type_ != *value_type && target_type.type_ != NormalizedType::Unknown && *value_type != NormalizedType::Unknown {
            return Err(SemanticError::TypeMismatch {
                expected: target_type.type_.to_string(),
                found: value_type.to_string(),
                context: "asignación".to_string(),
            });
        }
        
        Ok(ExpressionType::value(value_type.clone()))
    }

    /// Verifica tipo de constructor (new)
    ///
    /// - El tipo debe estar definido
    /// - Validar argumentos contra constructor si existe
    pub fn check_new(
        &self,
        type_name: &str,
        argument_types: &[NormalizedType],
        expected_params: Option<&[(String, NormalizedType)]>,
        _span: &Span,
    ) -> SemanticResult<ExpressionType> {
        if let Some(params) = expected_params {
            if argument_types.len() != params.len() {
                return Err(SemanticError::InvalidConstructor {
                    type_name: type_name.to_string(),
                    reason: format!("esperaba {} argumentos, recibió {}", params.len(), argument_types.len()),
                });
            }

            for (i, (arg_type, (param_name, param_type))) in argument_types.iter().zip(params.iter()).enumerate() {
                if arg_type != param_type && arg_type != &NormalizedType::Unknown && param_type != &NormalizedType::Unknown {
                    return Err(SemanticError::ArgumentTypeMismatch {
                        function: type_name.to_string(),
                        parameter_name: param_name.to_string(),
                        parameter_position: i,
                        expected: param_type.to_string(),
                        found: arg_type.to_string(),
                    });
                }
            }
        }
        Ok(ExpressionType::value(NormalizedType::Named(
            type_name.to_string(),
        )))
    }

    /// Verifica tipo de un acceso a miembro (obj.member)
    pub fn check_member_access(
        &self,
        object_type: &NormalizedType,
        member_name: &str,
        member_type: Option<&NormalizedType>,
        _span: &Span,
    ) -> SemanticResult<ExpressionType> {
        if let Some(m_type) = member_type {
            // Propiedades suelen ser lvalues, por convención asignamos true. Dependerá de las reglas si son mutables.
            Ok(ExpressionType::lvalue(m_type.clone()))
        } else {
            Err(SemanticError::MemberNotFound {
                type_name: object_type.to_string(),
                member_name: member_name.to_string(),
            })
        }
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

    /// Verifica acceso a índice de vector
    pub fn check_index_access(
        &self,
        object_type: &NormalizedType,
        index_type: &NormalizedType,
        _span: &Span,
    ) -> SemanticResult<ExpressionType> {
        if *index_type != NormalizedType::Number && *index_type != NormalizedType::Unknown {
            return Err(SemanticError::InvalidIndexType {
                expected: "Number".to_string(),
                found: index_type.to_string(),
            });
        }

        match object_type {
            NormalizedType::Vector(inner) => {
                // Return an LValue so that you can do x[0] = 5
                Ok(ExpressionType::lvalue((**inner).clone()))
            }
            NormalizedType::Unknown => Ok(ExpressionType::lvalue(NormalizedType::Unknown)),
            _ => Err(SemanticError::NotIndexable {
                type_name: object_type.to_string(),
            }),
        }
    }

    /// Verifica vector literal (e.g. [1, 2, 3])
    pub fn check_vector_literal(
        &self,
        element_types: &[NormalizedType],
        _span: &Span,
    ) -> SemanticResult<ExpressionType> {
        if element_types.is_empty() {
            return Ok(ExpressionType::value(NormalizedType::Vector(Box::new(
                NormalizedType::Unknown,
            ))));
        }

        let first_type = &element_types[0];
        for (i, element_type) in element_types.iter().enumerate().skip(1) {
            if element_type != first_type && *element_type != NormalizedType::Unknown && *first_type != NormalizedType::Unknown {
                return Err(SemanticError::InconsistentArrayTypes {
                    expected: first_type.to_string(),
                    found: element_type.to_string(),
                    position: i,
                });
            }
        }

        Ok(ExpressionType::value(NormalizedType::Vector(Box::new(
            first_type.clone(),
        ))))
    }

    /// Verifica uso de enumeraciones (comprensión o iterador explícito)
    pub fn check_iterable_usage(
        &self,
        element_expr_type: &NormalizedType,
        _iterable_type: &NormalizedType,
        _span: &Span,
    ) -> SemanticResult<ExpressionType> {
        Ok(ExpressionType::value(NormalizedType::Iterable(Box::new(
            element_expr_type.clone(),
        ))))
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

    #[test]
    fn test_let_expression_valid() {
        let checker = ExpressionChecker::new();
        // let x: Number = 5;
        let result = checker.check_let_expression(
            Some(&NormalizedType::Number),
            Some(&NormalizedType::Number),
            &Span::default()
        ).unwrap();
        assert_eq!(result.type_, NormalizedType::Number);
        
        // let inferred = 5;
        let result = checker.check_let_expression(
            None,
            Some(&NormalizedType::Number),
            &Span::default()
        ).unwrap();
        assert_eq!(result.type_, NormalizedType::Number);
    }

    #[test]
    fn test_let_expression_type_mismatch() {
        let checker = ExpressionChecker::new();
        // let x: Number = "hello";
        let result = checker.check_let_expression(
            Some(&NormalizedType::Number),
            Some(&NormalizedType::String),
            &Span::default()
        );
        assert!(matches!(result, Err(SemanticError::TypeMismatch { .. })));
    }

    #[test]
    fn test_assignment_valid() {
        let checker = ExpressionChecker::new();
        // x = 5; (where x is a number lvalue)
        let target = ExpressionType::lvalue(NormalizedType::Number);
        let result = checker.check_assignment_expression(
            &target,
            &NormalizedType::Number,
            &Span::default()
        ).unwrap();
        assert_eq!(result.type_, NormalizedType::Number);
        assert!(!result.is_lvalue); // The result of the assignment expression is not an lvalue
    }

    #[test]
    fn test_assignment_invalid_lvalue() {
        let checker = ExpressionChecker::new();
        // 5 = 10; (where 5 is a value, not an lvalue)
        let target = ExpressionType::value(NormalizedType::Number);
        let result = checker.check_assignment_expression(
            &target,
            &NormalizedType::Number,
            &Span::default()
        );
        assert!(matches!(result, Err(SemanticError::AssignmentToImmutable { .. })));
    }

    #[test]
    fn test_assignment_type_mismatch() {
        let checker = ExpressionChecker::new();
        // x = "hello"; (where x is a number lvalue)
        let target = ExpressionType::lvalue(NormalizedType::Number);
        let result = checker.check_assignment_expression(
            &target,
            &NormalizedType::String,
            &Span::default()
        );
        assert!(matches!(result, Err(SemanticError::TypeMismatch { .. })));
    }

    #[test]
    fn test_function_call_builtin_print() {
        let checker = ExpressionChecker::new();
        let result = checker.check_function_call(
            "print",
            &[NormalizedType::String],
            None,
            None,
            &Span::default()
        ).unwrap();
        assert_eq!(result.type_, NormalizedType::String);
    }

    #[test]
    fn test_function_call_builtin_math() {
        let checker = ExpressionChecker::new();
        
        // sin(Number) -> Number
        let result = checker.check_function_call(
            "sin",
            &[NormalizedType::Number],
            None,
            None,
            &Span::default()
        ).unwrap();
        assert_eq!(result.type_, NormalizedType::Number);
        
        // log(Number, Number) -> Number
        let result = checker.check_function_call(
            "log",
            &[NormalizedType::Number, NormalizedType::Number],
            None,
            None,
            &Span::default()
        ).unwrap();
        assert_eq!(result.type_, NormalizedType::Number);
    }

    #[test]
    fn test_function_call_builtin_invalid() {
        let checker = ExpressionChecker::new();
        // sin(String)
        let result = checker.check_function_call(
            "sin",
            &[NormalizedType::String],
            None,
            None,
            &Span::default()
        );
        assert!(matches!(result, Err(SemanticError::ArgumentTypeMismatch { .. })));
    }

    #[test]
    fn test_new_expression_valid() {
        let checker = ExpressionChecker::new();
        let expected_params = vec![
            ("age".to_string(), NormalizedType::Number)
        ];
        let result = checker.check_new(
            "Person",
            &[NormalizedType::Number],
            Some(&expected_params),
            &Span::default()
        ).unwrap();
        assert_eq!(result.type_, NormalizedType::Named("Person".to_string()));
    }

    #[test]
    fn test_new_expression_invalid_args() {
        let checker = ExpressionChecker::new();
        let expected_params = vec![
            ("age".to_string(), NormalizedType::Number)
        ];
        let result = checker.check_new(
            "Person",
            &[NormalizedType::String],
            Some(&expected_params),
            &Span::default()
        );
        assert!(matches!(result, Err(SemanticError::ArgumentTypeMismatch { .. })));

        let result_count = checker.check_new(
            "Person",
            &[],
            Some(&expected_params),
            &Span::default()
        );
        assert!(matches!(result_count, Err(SemanticError::InvalidConstructor { .. })));
    }

    #[test]
    fn test_member_access_valid() {
        let checker = ExpressionChecker::new();
        let result = checker.check_member_access(
            &NormalizedType::Named("Person".to_string()),
            "age",
            Some(&NormalizedType::Number),
            &Span::default()
        ).unwrap();
        assert_eq!(result.type_, NormalizedType::Number);
    }

    #[test]
    fn test_member_access_invalid() {
        let checker = ExpressionChecker::new();
        let result = checker.check_member_access(
            &NormalizedType::Named("Person".to_string()),
            "unknown_field",
            None,
            &Span::default()
        );
        assert!(matches!(result, Err(SemanticError::MemberNotFound { .. })));
    }

    #[test]
    fn test_is_operator() {
        let checker = ExpressionChecker::new();
        let result = checker.check_is(
            &NormalizedType::Number,
            &NormalizedType::Named("Object".to_string()),
            &Span::default()
        ).unwrap();
        assert_eq!(result.type_, NormalizedType::Boolean);
    }

    #[test]
    fn test_as_operator() {
        let checker = ExpressionChecker::new();
        let result = checker.check_as(
            &NormalizedType::Number,
            &NormalizedType::Named("Object".to_string()),
            &Span::default()
        ).unwrap();
        assert_eq!(result.type_, NormalizedType::Named("Object".to_string()));
    }

    #[test]
    fn test_index_access_valid() {
        let checker = ExpressionChecker::new();
        let vec_type = NormalizedType::Vector(Box::new(NormalizedType::Number));
        let result = checker.check_index_access(&vec_type, &NormalizedType::Number, &Span::default()).unwrap();
        assert_eq!(result.type_, NormalizedType::Number);
        assert!(result.is_lvalue);
    }

    #[test]
    fn test_index_access_invalid_target() {
        let checker = ExpressionChecker::new();
        let result = checker.check_index_access(&NormalizedType::Number, &NormalizedType::Number, &Span::default());
        assert!(matches!(result.unwrap_err(), SemanticError::NotIndexable { .. }));
    }

    #[test]
    fn test_index_access_invalid_index() {
        let checker = ExpressionChecker::new();
        let vec_type = NormalizedType::Vector(Box::new(NormalizedType::Number));
        let result = checker.check_index_access(&vec_type, &NormalizedType::String, &Span::default());
        assert!(matches!(result.unwrap_err(), SemanticError::InvalidIndexType { .. }));
    }

    #[test]
    fn test_vector_literal_valid() {
        let checker = ExpressionChecker::new();
        let types = vec![NormalizedType::Number, NormalizedType::Number];
        let result = checker.check_vector_literal(&types, &Span::default()).unwrap();
        assert_eq!(result.type_, NormalizedType::Vector(Box::new(NormalizedType::Number)));
    }

    #[test]
    fn test_vector_literal_inconsistent() {
        let checker = ExpressionChecker::new();
        let types = vec![NormalizedType::Number, NormalizedType::String];
        let result = checker.check_vector_literal(&types, &Span::default());
        assert!(matches!(result.unwrap_err(), SemanticError::InconsistentArrayTypes { .. }));
    }

    #[test]
    fn test_iterable_usage() {
        let checker = ExpressionChecker::new();
        let result = checker.check_iterable_usage(&NormalizedType::Number, &NormalizedType::Iterable(Box::new(NormalizedType::Number)), &Span::default()).unwrap();
        assert_eq!(result.type_, NormalizedType::Iterable(Box::new(NormalizedType::Number)));
    }
}
