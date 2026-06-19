use crate::lexer::TokenType;
use crate::utils::errors::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub declarations: Vec<Declaration>,
    pub entry_expression: Option<Expr>,
    pub span: Span,
}

impl Program {
    pub fn new(declarations: Vec<Declaration>, entry_expression: Option<Expr>, span: Span) -> Self {
        Self {
            declarations,
            entry_expression,
            span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    pub kind: DeclarationKind,
    pub span: Span,
}

impl Declaration {
    pub fn new(kind: DeclarationKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeclarationKind {
    Function(FunctionDeclaration),
    Type(TypeDeclaration),
    Protocol(ProtocolDeclaration),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDeclaration {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<TypeReference>,
    pub body: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariableDeclaration {
    pub name: String,
    pub annotation: Option<TypeReference>,
    pub value: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeDeclaration {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub inherits: Option<TypeReference>,
    pub parent_arguments: Vec<Expr>,
    pub members: Vec<TypeMember>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProtocolDeclaration {
    pub name: String,
    pub extends: Vec<TypeReference>,
    pub members: Vec<ProtocolMethodSignature>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeMember {
    Attribute(AttributeDeclaration),
    Method(FunctionDeclaration),
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttributeDeclaration {
    pub name: String,
    pub annotation: Option<TypeReference>,
    pub initializer: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProtocolMethodSignature {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: TypeReference,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub annotation: Option<TypeReference>,
    pub span: Span,
}

impl Parameter {
    pub fn new(name: String, annotation: Option<TypeReference>, span: Span) -> Self {
        Self {
            name,
            annotation,
            span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeReference {
    pub kind: TypeReferenceKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeReferenceKind {
    Named(String),
    Iterable(Box<TypeReference>),
    Vector(Box<TypeReference>),
}

impl TypeReference {
    pub fn new(name: String, span: Span) -> Self {
        Self {
            kind: TypeReferenceKind::Named(name),
            span,
        }
    }

    pub fn iterable_of(element_type: TypeReference, span: Span) -> Self {
        Self {
            kind: TypeReferenceKind::Iterable(Box::new(element_type)),
            span,
        }
    }

    pub fn vector_of(element_type: TypeReference, span: Span) -> Self {
        Self {
            kind: TypeReferenceKind::Vector(Box::new(element_type)),
            span,
        }
    }

    pub fn display_name(&self) -> String {
        match &self.kind {
            TypeReferenceKind::Named(name) => name.clone(),
            TypeReferenceKind::Iterable(inner) => format!("{}*", inner.display_name()),
            TypeReferenceKind::Vector(inner) => format!("{}[]", inner.display_name()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn literal(literal: Literal, span: Span) -> Self {
        Self::new(ExprKind::Literal(literal), span)
    }

    pub fn identifier(name: String, span: Span) -> Self {
        Self::new(ExprKind::Identifier(name), span)
    }

    pub fn unary(operator: UnaryOperator, operand: Expr, span: Span) -> Self {
        Self::new(
            ExprKind::Unary {
                operator,
                operand: Box::new(operand),
            },
            span,
        )
    }

    pub fn binary(left: Expr, operator: BinaryOperator, right: Expr, span: Span) -> Self {
        Self::new(
            ExprKind::Binary {
                left: Box::new(left),
                operator,
                right: Box::new(right),
            },
            span,
        )
    }

    pub fn call(callee: Expr, arguments: Vec<Expr>, span: Span) -> Self {
        Self::new(
            ExprKind::Call {
                callee: Box::new(callee),
                arguments,
            },
            span,
        )
    }

    pub fn assignment(target: Expr, value: Expr, span: Span) -> Self {
        Self::new(
            ExprKind::Assignment {
                target: Box::new(target),
                value: Box::new(value),
            },
            span,
        )
    }

    pub fn block(expressions: Vec<Expr>, span: Span) -> Self {
        Self::new(ExprKind::Block(expressions), span)
    }

    pub fn if_expr(
        condition: Expr,
        then_expr: Expr,
        elif_parts: Vec<(Expr, Expr)>,
        else_expr: Option<Expr>,
        span: Span,
    ) -> Self {
        Self::new(
            ExprKind::If {
                condition: Box::new(condition),
                then_expr: Box::new(then_expr),
                elif_parts,
                else_expr: else_expr.map(Box::new),
            },
            span,
        )
    }

    pub fn while_expr(condition: Expr, body: Expr, span: Span) -> Self {
        Self::new(
            ExprKind::While {
                condition: Box::new(condition),
                body: Box::new(body),
            },
            span,
        )
    }

    pub fn for_expr(variable: String, iterable: Expr, body: Expr, span: Span) -> Self {
        Self::new(
            ExprKind::For {
                variable,
                iterable: Box::new(iterable),
                body: Box::new(body),
            },
            span,
        )
    }

    pub fn let_expr(
        name: String,
        annotation: Option<TypeReference>,
        value: Expr,
        body: Expr,
        span: Span,
    ) -> Self {
        Self::new(
            ExprKind::Let {
                name,
                annotation,
                value: Box::new(value),
                body: Box::new(body),
            },
            span,
        )
    }

    pub fn member_access(object: Expr, member: String, span: Span) -> Self {
        Self::new(
            ExprKind::MemberAccess {
                object: Box::new(object),
                member,
            },
            span,
        )
    }

    pub fn index_access(object: Expr, index: Expr, span: Span) -> Self {
        Self::new(
            ExprKind::IndexAccess {
                object: Box::new(object),
                index: Box::new(index),
            },
            span,
        )
    }

    pub fn type_check(expr: Expr, type_ref: TypeReference, span: Span) -> Self {
        Self::new(
            ExprKind::TypeCheck {
                expr: Box::new(expr),
                type_ref,
            },
            span,
        )
    }

    pub fn type_cast(expr: Expr, type_ref: TypeReference, span: Span) -> Self {
        Self::new(
            ExprKind::TypeCast {
                expr: Box::new(expr),
                type_ref,
            },
            span,
        )
    }

    pub fn new_expr(type_ref: TypeReference, arguments: Vec<Expr>, span: Span) -> Self {
        Self::new(
            ExprKind::New {
                type_ref,
                arguments,
            },
            span,
        )
    }

    pub fn self_expr(span: Span) -> Self {
        Self::new(ExprKind::Self_, span)
    }

    pub fn base_expr(span: Span) -> Self {
        Self::new(ExprKind::Base, span)
    }

    pub fn vector_literal(elements: Vec<Expr>, span: Span) -> Self {
        Self::new(ExprKind::VectorLiteral(elements), span)
    }

    pub fn vector_comprehension(
        element_expr: Expr,
        binding: String,
        iterable: Expr,
        span: Span,
    ) -> Self {
        Self::new(
            ExprKind::VectorComprehension {
                element_expr: Box::new(element_expr),
                binding,
                iterable: Box::new(iterable),
            },
            span,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    // Literales y acceso
    Literal(Literal),
    Identifier(String),

    // Operadores
    Unary {
        operator: UnaryOperator,
        operand: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        operator: BinaryOperator,
        right: Box<Expr>,
    },

    // Agrupamiento
    Block(Vec<Expr>),

    // Funciones
    Call {
        callee: Box<Expr>,
        arguments: Vec<Expr>,
    },
    Assignment {
        target: Box<Expr>,
        value: Box<Expr>,
    },

    // Control de flujo
    If {
        condition: Box<Expr>,
        then_expr: Box<Expr>,
        elif_parts: Vec<(Expr, Expr)>,
        else_expr: Option<Box<Expr>>,
    },
    While {
        condition: Box<Expr>,
        body: Box<Expr>,
    },
    For {
        variable: String,
        iterable: Box<Expr>,
        body: Box<Expr>,
    },

    // Declaraciones (como expresiones)
    Let {
        name: String,
        annotation: Option<TypeReference>,
        value: Box<Expr>,
        body: Box<Expr>,
    },

    // Acceso a miembros
    MemberAccess {
        object: Box<Expr>,
        member: String,
    },
    IndexAccess {
        object: Box<Expr>,
        index: Box<Expr>,
    },

    // Operaciones de tipo
    TypeCheck {
        expr: Box<Expr>,
        type_ref: TypeReference,
    },
    TypeCast {
        expr: Box<Expr>,
        type_ref: TypeReference,
    },

    // Instanciación y referencias
    New {
        type_ref: TypeReference,
        arguments: Vec<Expr>,
    },
    Self_,
    Base,
    // Vectores
    VectorLiteral(Vec<Expr>),
    VectorComprehension {
        element_expr: Box<Expr>,
        binding: String,
        iterable: Box<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Number(f64),
    String(String),
    Boolean(bool),
    Pi,
    E,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Plus,
    Minus,
    Not,
}

impl UnaryOperator {
    pub fn from_token_type(token_type: &TokenType) -> Option<Self> {
        match token_type {
            TokenType::Plus => Some(Self::Plus),
            TokenType::Minus => Some(Self::Minus),
            TokenType::Bang => Some(Self::Not),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Power,
    Modulo,
    Concat,
    Concatenate,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
}

impl BinaryOperator {
    pub fn from_token_type(token_type: &TokenType) -> Option<Self> {
        match token_type {
            TokenType::Plus => Some(Self::Add),
            TokenType::Minus => Some(Self::Subtract),
            TokenType::Star => Some(Self::Multiply),
            TokenType::Slash => Some(Self::Divide),
            TokenType::Caret => Some(Self::Power),
            TokenType::Percent => Some(Self::Modulo),
            TokenType::At => Some(Self::Concat),
            TokenType::AtAt => Some(Self::Concatenate),
            TokenType::EqualEqual => Some(Self::Equal),
            TokenType::BangEqual => Some(Self::NotEqual),
            TokenType::Less => Some(Self::Less),
            TokenType::LessEqual => Some(Self::LessEqual),
            TokenType::Greater => Some(Self::Greater),
            TokenType::GreaterEqual => Some(Self::GreaterEqual),
            TokenType::Ampersand => Some(Self::And),
            TokenType::Pipe => Some(Self::Or),
            _ => None,
        }
    }
}
