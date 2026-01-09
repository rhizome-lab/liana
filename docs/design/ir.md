# Intermediate Representation (IR)

The IR is a unified representation for all API surfaces - HTTP APIs (OpenAPI), FFI (C headers), and future sources. Goal: one IR that's a superset, preserving all information without ambiguity.

## Design Principles

- **Preserve raw info**: Don't lose information during parsing. Let target generators decide how to map.
- **Extensible, not hardcoded**: No fixed set of primitives, bounds, constraints, or modifiers. Everything uses extensible `kind: String` patterns.
- **Unify via types**: HTTP semantics, FFI semantics → express as types, not special cases.
- **Uniform structure**: Minimize special cases. Same patterns at different levels.

## Core Structures

### Type

```rust
struct Type {
    kind: TypeKind,
    name: Option<String>,         // None = anonymous
    params: Vec<TypeParam>,       // type parameters: <T, U>
    args: Vec<Type>,              // type arguments: <i32, String>
    annotations: Vec<Annotation>, // bounds, constraints, modifiers - all unified
    metadata: Metadata,
}

enum TypeKind {
    Ref(String),                          // reference to named type
    Struct { fields: Vec<Field> },
    Enum { variants: Vec<Variant> },
    Function { params: Vec<Param>, ret: Box<Type> },
    Union { members: Vec<Type> },         // A | B
    Intersection { members: Vec<Type> },  // A & B
}
```

### Annotations

Bounds, constraints, and modifiers unified into one extensible concept:

```rust
struct Annotation {
    kind: String,
    value: Option<AnnotationValue>,
}

enum AnnotationValue {
    Type(Box<Type>),
    String(String),
    Number(f64),
    Bool(bool),
    List(Vec<AnnotationValue>),
}
```

Examples:
- `extends Foo` → `{ kind: "extends", value: Type(Foo) }`
- `implements Bar` → `{ kind: "implements", value: Type(Bar) }`
- `min: 0` → `{ kind: "min", value: Number(0) }`
- `max: 100` → `{ kind: "max", value: Number(100) }`
- `pattern: "^[a-z]+$"` → `{ kind: "pattern", value: String(...) }`
- `format: "uuid"` → `{ kind: "format", value: String("uuid") }`
- `const` → `{ kind: "const", value: None }`
- `mut` → `{ kind: "mut", value: None }`
- `ref` → `{ kind: "ref", value: None }`
- `in` → `{ kind: "in", value: None }` (variance)
- `out` → `{ kind: "out", value: None }` (variance)
- `deprecated` → `{ kind: "deprecated", value: None }`
- `calling_convention: "cdecl"` → `{ kind: "calling_convention", value: String("cdecl") }`

Generators handle known kinds, ignore/warn on unknown.

### TypeParam

```rust
struct TypeParam {
    name: String,
    bounds: Vec<Annotation>,      // T: Foo + Bar → annotations with kind "bound"
    default: Option<Type>,        // T = String
}
```

### Field

```rust
struct Field {
    name: Option<String>,         // None = positional (tuple struct)
    typ: Type,
    annotations: Vec<Annotation>,
}
```

Newtype = struct with single positional field:
```rust
Type {
    kind: Struct { fields: [Field { name: None, typ: inner }] },
    name: Some("Uuid"),
    ...
}
```

### Variant

```rust
struct Variant {
    name: String,
    fields: Vec<Field>,           // empty = unit variant, named = struct variant, positional = tuple variant
    annotations: Vec<Annotation>,
}
```

### Param

```rust
struct Param {
    name: Option<String>,
    typ: Type,
    default: Option<Value>,       // default value for optional params
    annotations: Vec<Annotation>, // ref, in, out, etc.
}
```

### Value

```rust
enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    List(Vec<Value>),
    Object(HashMap<String, Value>),
}
```

## Special Types

No special-cased type kinds. Use `Ref` to well-known names:

- `Ref("Unit")` - unit type, void
- `Ref("Never")` - bottom type, never returns
- `Ref("Any")` - top type, any value
- `Ref("i32")`, `Ref("f64")`, etc. - primitives

Generators map these to target language equivalents.

## Generic Instantiation

`args` on Type for applying type arguments:

```rust
// Vec<i32>
Type {
    kind: Ref("Vec"),
    args: [Type { kind: Ref("i32"), ... }],
    ...
}

// Result<T, Error> where T is a param
Type {
    kind: Ref("Result"),
    args: [
        Type { kind: Ref("T"), ... },
        Type { kind: Ref("Error"), ... },
    ],
    ...
}
```

Any TypeKind can have args for uniformity, though typically only Ref uses them.

## HTTP APIs (OpenAPI)

Express as types and functions, no special HTTP layer:

```rust
// GET /users/{id} -> User

// Path params as struct
Type {
    kind: Struct { fields: [Field { name: Some("id"), typ: Ref("String") }] },
    name: Some("GetUserPath"),
}

// Endpoint as function
Function {
    name: "getUser",
    params: [
        Param { name: Some("path"), typ: Ref("GetUserPath") },
        Param { name: Some("query"), typ: Ref("GetUserQuery") },
    ],
    ret: Type { kind: Ref("Result"), args: [Ref("User"), Ref("ApiError")] },
    annotations: [
        { kind: "http_method", value: String("GET") },
        { kind: "http_path", value: String("/users/{id}") },
    ],
}
```

## FFI (C Headers)

Same type system, FFI-specific annotations:

```rust
// wlr_output* wlr_output_create(wlr_backend* backend)

Function {
    name: "wlr_output_create",
    params: [
        Param {
            name: Some("backend"),
            typ: Type { kind: Ref("Ptr"), args: [Ref("WlrBackend")] },
        }
    ],
    ret: Type { kind: Ref("Ptr"), args: [Ref("WlrOutput")] },
    annotations: [
        { kind: "calling_convention", value: String("cdecl") },
    ],
}
```

Nullability via annotations or `Ref("Option")` wrapper.
Ownership via annotations: `{ kind: "ownership", value: String("owned") }`.

## Modules

```rust
struct Module {
    name: String,
    items: Vec<Item>,
    submodules: Vec<Module>,
    annotations: Vec<Annotation>,
    metadata: Metadata,
}

enum Item {
    Type(Type),
    Function(Function),
    Const { name: String, typ: Type, value: Value },
}
```

## Metadata

```rust
struct Metadata {
    docs: Option<String>,
    source_location: Option<SourceLocation>,
    confidence: Option<f32>,      // for assisted generation
    extra: HashMap<String, Value>, // escape hatch
}
```

## Decisions

- **Interning**: Yes - types should be interned for deduplication and fast comparison.

## Open Questions

- **Validation layer**: When/how to validate annotation kinds and values?
- **Versioning**: How to handle API versions in IR?
- **Streaming**: How to represent streaming responses / async generators?

## Deferred

- **Specialcases format**: Design when we hit real pain points
- **Confidence scoring**: Design when we hit ambiguous cases in parsing
- **C preprocessor strategy**: Defer until FFI parser implementation
