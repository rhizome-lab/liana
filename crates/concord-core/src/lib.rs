//! Core IR types for liana API bindings.
//!
//! This module defines the intermediate representation used by parsers and generators.
//! The IR is designed to be:
//! - Extensible: no hardcoded primitives, bounds, or modifiers
//! - Unified: HTTP and FFI semantics expressed as types, not special cases
//! - Preserving: raw info kept, generators decide how to map

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// A type in the IR.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Type {
    pub kind: TypeKind,
    /// Name if this is a named/declared type, None if anonymous.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Type parameters for generic declarations: `<T, U>`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<TypeParam>,
    /// Type arguments for generic instantiation: `<i32, String>`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<Type>,
    /// Bounds, constraints, modifiers - all unified.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<Annotation>,
    /// Documentation, source location, confidence, etc.
    #[serde(default, skip_serializing_if = "Metadata::is_empty")]
    pub metadata: Metadata,
}

/// The structural kind of a type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TypeKind {
    /// Reference to a named type.
    Ref { name: String },
    /// Product type with fields.
    Struct { fields: Vec<Field> },
    /// Sum type with variants.
    Enum { variants: Vec<Variant> },
    /// Function/callback type.
    Function { params: Vec<Param>, ret: Box<Type> },
    /// Union type: A | B.
    Union { members: Vec<Type> },
    /// Intersection type: A & B.
    Intersection { members: Vec<Type> },
}

/// A type parameter with optional bounds and default.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeParam {
    pub name: String,
    /// Bounds: `T: Foo + Bar`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bounds: Vec<Annotation>,
    /// Default value: `T = String`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Box<Type>>,
}

/// An annotation (bound, constraint, or modifier).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Annotation {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<AnnotationValue>,
}

/// The value of an annotation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AnnotationValue {
    Type(Box<Type>),
    String(String),
    Number(f64),
    Bool(bool),
    List(Vec<AnnotationValue>),
}

/// A field in a struct or variant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Field {
    /// None = positional (tuple struct/variant).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub typ: Type,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<Annotation>,
}

/// A variant in an enum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Variant {
    pub name: String,
    /// Empty = unit variant, fields with names = struct variant, without = tuple variant.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<Field>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<Annotation>,
}

/// A function parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Param {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub typ: Type,
    /// Default value for optional parameters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<Annotation>,
}

/// A runtime value (for defaults, constants).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    List(Vec<Value>),
    Object(IndexMap<String, Value>),
}

/// A function declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<TypeParam>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<Param>,
    #[serde(rename = "return")]
    pub ret: Type,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<Annotation>,
    #[serde(default, skip_serializing_if = "Metadata::is_empty")]
    pub metadata: Metadata,
}

/// A module containing types, functions, and submodules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Module {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<Item>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub submodules: Vec<Module>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<Annotation>,
    #[serde(default, skip_serializing_if = "Metadata::is_empty")]
    pub metadata: Metadata,
}

/// An item in a module.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "item", rename_all = "snake_case")]
pub enum Item {
    Type(Type),
    Function(Function),
    Const {
        name: String,
        #[serde(rename = "type")]
        typ: Type,
        value: Value,
    },
}

/// Metadata attached to types, functions, modules.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Metadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docs: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceLocation>,
    /// Confidence score for assisted generation (0.0 - 1.0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    /// Escape hatch for extra data.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub extra: IndexMap<String, Value>,
}

impl Metadata {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.docs.is_none()
            && self.source.is_none()
            && self.confidence.is_none()
            && self.extra.is_empty()
    }
}

/// Source location for error reporting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
}

// Convenience constructors

impl Type {
    /// Create a reference to a named type.
    pub fn reference(name: impl Into<String>) -> Self {
        Self {
            kind: TypeKind::Ref { name: name.into() },
            name: None,
            params: Vec::new(),
            args: Vec::new(),
            annotations: Vec::new(),
            metadata: Metadata::default(),
        }
    }

    /// Create a generic instantiation: `base<args>`.
    pub fn generic(base: impl Into<String>, args: Vec<Type>) -> Self {
        Self {
            kind: TypeKind::Ref { name: base.into() },
            name: None,
            params: Vec::new(),
            args,
            annotations: Vec::new(),
            metadata: Metadata::default(),
        }
    }
}

impl Annotation {
    /// Create an annotation with no value.
    pub fn flag(kind: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            value: None,
        }
    }

    /// Create an annotation with a type value.
    pub fn with_type(kind: impl Into<String>, typ: Type) -> Self {
        Self {
            kind: kind.into(),
            value: Some(AnnotationValue::Type(Box::new(typ))),
        }
    }

    /// Create an annotation with a string value.
    pub fn with_string(kind: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            value: Some(AnnotationValue::String(value.into())),
        }
    }

    /// Create an annotation with a number value.
    pub fn with_number(kind: impl Into<String>, value: f64) -> Self {
        Self {
            kind: kind.into(),
            value: Some(AnnotationValue::Number(value)),
        }
    }
}
