# Introduction

liana provides type-safe bindings to various APIs in Rust:

- **Web APIs** - Browser and web platform APIs
- **OpenAPI** - Generated clients from OpenAPI/Swagger schemas
- **FFI** - Foreign function interface bindings

## Design Goals

- **Type safety** - Strongly typed APIs with newtypes and enums
- **Backend agnostic** - Works with tokio, async-std, or any runtime
- **Edition 2024** - Uses latest Rust features
- **Library agnostic** - No forced dependencies on specific HTTP clients

## Repository Structure

**Branches:**
- `master` - core infrastructure only
- `bindings` - core + API bindings

**master branch:**
```
crates/                # Core infrastructure
├── liana-core/        # Shared utilities
├── liana-http/        # HTTP client trait
└── liana-codegen/     # OpenAPI code generator

schemas/               # OpenAPI specs (source of truth)
```

**bindings branch adds:**
```
bindings/              # API bindings
├── web-fetch/         # Fetch API bindings
├── openapi-github/    # GitHub API client
└── ffi-sqlite/        # SQLite FFI bindings
```
