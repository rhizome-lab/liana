//! Rust code generator.

use std::fmt::Write;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use liana_core::{Field, Function, Item, Module, Type, TypeKind};

/// Generate Rust code from IR.
pub fn generate(module: &Module, output: &Path) -> Result<()> {
    fs::create_dir_all(output)?;

    let mut code = String::new();

    // Module doc comment
    if let Some(docs) = &module.metadata.docs {
        writeln!(code, "//! {docs}")?;
        writeln!(code)?;
    }

    // Preamble
    writeln!(code, "use serde::{{Deserialize, Serialize}};")?;
    writeln!(code)?;
    writeln!(code, "/// API error type.")?;
    writeln!(
        code,
        "#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]"
    )?;
    writeln!(code, "pub struct ApiError {{")?;
    writeln!(code, "    pub message: String,")?;
    writeln!(code, "    pub code: Option<String>,")?;
    writeln!(code, "}}")?;
    writeln!(code)?;

    // Generate items
    for item in &module.items {
        match item {
            Item::Type(typ) => generate_type(&mut code, typ)?,
            Item::Function(func) => generate_function(&mut code, func)?,
            Item::Const { name, typ, value } => {
                let typ_str = type_to_rust(typ);
                let val_str = format!("{value:?}");
                writeln!(code, "pub const {name}: {typ_str} = {val_str};")?;
                writeln!(code)?;
            }
        }
    }

    let output_file = output.join("mod.rs");
    fs::write(&output_file, &code)
        .with_context(|| format!("Failed to write {}", output_file.display()))?;

    Ok(())
}

fn generate_type(out: &mut String, typ: &Type) -> Result<()> {
    let Some(name) = &typ.name else {
        return Ok(());
    };

    // Doc comment
    if let Some(docs) = &typ.metadata.docs {
        for line in docs.lines() {
            writeln!(out, "/// {line}")?;
        }
    }

    match &typ.kind {
        TypeKind::Struct { fields } => {
            generate_struct(out, name, &typ.params, fields)?;
        }
        TypeKind::Enum { variants } => {
            generate_enum(out, name, &typ.params, variants)?;
        }
        TypeKind::Union { members } => {
            // Generate as enum with variants for each member
            writeln!(
                out,
                "#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]"
            )?;
            writeln!(out, "#[serde(untagged)]")?;
            writeln!(out, "pub enum {name} {{")?;
            for (i, member) in members.iter().enumerate() {
                let variant_name = member.name.clone().unwrap_or_else(|| format!("Variant{i}"));
                let typ_str = type_to_rust(member);
                writeln!(out, "    {variant_name}({typ_str}),")?;
            }
            writeln!(out, "}}")?;
            writeln!(out)?;
        }
        TypeKind::Ref { name: target } => {
            // Type alias
            let args_str = if typ.args.is_empty() {
                String::new()
            } else {
                let args: Vec<_> = typ.args.iter().map(type_to_rust).collect();
                format!("<{}>", args.join(", "))
            };
            writeln!(out, "pub type {name} = {target}{args_str};")?;
            writeln!(out)?;
        }
        TypeKind::Function { params, ret } => {
            // Function type alias
            let params_str: Vec<_> = params.iter().map(|p| type_to_rust(&p.typ)).collect();
            let ret_str = type_to_rust(ret);
            writeln!(
                out,
                "pub type {name} = fn({}) -> {ret_str};",
                params_str.join(", ")
            )?;
            writeln!(out)?;
        }
        TypeKind::Intersection { .. } => {
            // Intersection types don't map cleanly to Rust
            // Generate as a struct combining all members
            writeln!(out, "// TODO: Intersection type {name}")?;
            writeln!(out)?;
        }
    }

    Ok(())
}

fn generate_struct(
    out: &mut String,
    name: &str,
    params: &[liana_core::TypeParam],
    fields: &[Field],
) -> Result<()> {
    writeln!(
        out,
        "#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]"
    )?;

    let generics = if params.is_empty() {
        String::new()
    } else {
        let names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();
        format!("<{}>", names.join(", "))
    };

    if fields.is_empty() {
        writeln!(out, "pub struct {name}{generics};")?;
    } else if fields.iter().all(|f| f.name.is_none()) {
        // Tuple struct
        writeln!(out, "pub struct {name}{generics}(")?;
        for field in fields {
            let typ_str = type_to_rust(&field.typ);
            writeln!(out, "    pub {typ_str},")?;
        }
        writeln!(out, ");")?;
    } else {
        // Named struct
        writeln!(out, "pub struct {name}{generics} {{")?;
        for field in fields {
            let field_name = field.name.as_deref().unwrap_or("_");
            let field_name = to_snake_case(field_name);
            let typ_str = type_to_rust(&field.typ);

            // Rename annotation for serde if name changed
            let original = field.name.as_deref().unwrap_or("_");
            if field_name != original {
                writeln!(out, "    #[serde(rename = \"{original}\")]")?;
            }

            writeln!(out, "    pub {field_name}: {typ_str},")?;
        }
        writeln!(out, "}}")?;
    }
    writeln!(out)?;

    Ok(())
}

fn generate_enum(
    out: &mut String,
    name: &str,
    params: &[liana_core::TypeParam],
    variants: &[liana_core::Variant],
) -> Result<()> {
    writeln!(
        out,
        "#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]"
    )?;

    let generics = if params.is_empty() {
        String::new()
    } else {
        let names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();
        format!("<{}>", names.join(", "))
    };

    writeln!(out, "pub enum {name}{generics} {{")?;
    for variant in variants {
        let variant_name = to_pascal_case(&variant.name);

        // Check for serde_rename annotation
        let rename = variant.annotations.iter().find_map(|a| {
            if a.kind == "serde_rename" {
                match &a.value {
                    Some(liana_core::AnnotationValue::String(s)) => Some(s.as_str()),
                    _ => None,
                }
            } else {
                None
            }
        });

        if variant.fields.is_empty() {
            if let Some(original) = rename
                && original != variant_name
            {
                writeln!(out, "    #[serde(rename = \"{original}\")]")?;
            }
            writeln!(out, "    {variant_name},")?;
        } else if variant.fields.iter().all(|f| f.name.is_none()) {
            // Tuple variant
            let types: Vec<_> = variant
                .fields
                .iter()
                .map(|f| type_to_rust(&f.typ))
                .collect();
            writeln!(out, "    {variant_name}({}),", types.join(", "))?;
        } else {
            // Struct variant
            writeln!(out, "    {variant_name} {{")?;
            for field in &variant.fields {
                let field_name = field.name.as_deref().unwrap_or("_");
                let field_name = to_snake_case(field_name);
                let typ_str = type_to_rust(&field.typ);
                writeln!(out, "        {field_name}: {typ_str},")?;
            }
            writeln!(out, "    }},")?;
        }
    }
    writeln!(out, "}}")?;
    writeln!(out)?;

    Ok(())
}

fn generate_function(out: &mut String, func: &Function) -> Result<()> {
    // Doc comment
    if let Some(docs) = &func.metadata.docs {
        for line in docs.lines() {
            writeln!(out, "/// {line}")?;
        }
    }

    // Extract HTTP annotations
    let method = func
        .annotations
        .iter()
        .find(|a| a.kind == "http_method")
        .and_then(|a| match &a.value {
            Some(liana_core::AnnotationValue::String(s)) => Some(s.as_str()),
            _ => None,
        });
    let path = func
        .annotations
        .iter()
        .find(|a| a.kind == "http_path")
        .and_then(|a| match &a.value {
            Some(liana_core::AnnotationValue::String(s)) => Some(s.as_str()),
            _ => None,
        });

    if let (Some(method), Some(path)) = (method, path) {
        writeln!(out, "/// HTTP: {method} {path}")?;
    }

    let func_name = to_snake_case(&func.name);

    let generics = if func.params.is_empty() {
        String::new()
    } else {
        let names: Vec<_> = func.params.iter().map(|p| p.name.as_str()).collect();
        format!("<{}>", names.join(", "))
    };

    let args = func
        .args
        .iter()
        .map(|p| {
            let name = p.name.as_deref().unwrap_or("_");
            let name = to_snake_case(name);
            let typ = type_to_rust(&p.typ);
            format!("{name}: {typ}")
        })
        .collect::<Vec<_>>()
        .join(", ");

    let ret = type_to_rust(&func.ret);

    writeln!(
        out,
        "pub async fn {func_name}{generics}({args}) -> {ret} {{"
    )?;
    writeln!(out, "    todo!()")?;
    writeln!(out, "}}")?;
    writeln!(out)?;

    Ok(())
}

fn type_to_rust(typ: &Type) -> String {
    match &typ.kind {
        TypeKind::Ref { name } => {
            let base = match name.as_str() {
                "String" => "String",
                "i64" => "i64",
                "f64" => "f64",
                "bool" => "bool",
                "Any" => "serde_json::Value",
                "Unit" => "()",
                "Never" => "!",
                other => other,
            };

            if typ.args.is_empty() {
                base.to_string()
            } else {
                let args: Vec<_> = typ.args.iter().map(type_to_rust).collect();
                format!("{base}<{}>", args.join(", "))
            }
        }
        TypeKind::Struct { .. } => {
            // Anonymous struct - use a generated name or inline
            typ.name
                .clone()
                .unwrap_or_else(|| "AnonymousStruct".to_string())
        }
        TypeKind::Enum { .. } => {
            // Named enums use the type name, anonymous enums fall back to String
            typ.name.clone().unwrap_or_else(|| "String".to_string())
        }
        TypeKind::Function { params, ret } => {
            let params_str: Vec<_> = params.iter().map(|p| type_to_rust(&p.typ)).collect();
            let ret_str = type_to_rust(ret);
            format!("fn({}) -> {ret_str}", params_str.join(", "))
        }
        TypeKind::Union { members } => {
            // For simple unions, could generate an enum
            // For now, use the first member or Any
            members
                .first()
                .map_or_else(|| "serde_json::Value".to_string(), type_to_rust)
        }
        TypeKind::Intersection { members } => {
            // Intersection is tricky - for now use first member
            members
                .first()
                .map_or_else(|| "serde_json::Value".to_string(), type_to_rust)
        }
    }
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_lower = false;

    for c in s.chars() {
        if c.is_uppercase() {
            if prev_lower {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
            prev_lower = false;
        } else if c == '-' || c == ' ' {
            result.push('_');
            prev_lower = false;
        } else {
            result.push(c);
            prev_lower = c.is_lowercase();
        }
    }

    // Handle Rust keywords
    match result.as_str() {
        "type" => "r#type".to_string(),
        "self" => "r#self".to_string(),
        "super" => "r#super".to_string(),
        "crate" => "r#crate".to_string(),
        "mod" => "r#mod".to_string(),
        "fn" => "r#fn".to_string(),
        "let" => "r#let".to_string(),
        "if" => "r#if".to_string(),
        "else" => "r#else".to_string(),
        "match" => "r#match".to_string(),
        "loop" => "r#loop".to_string(),
        "while" => "r#while".to_string(),
        "for" => "r#for".to_string(),
        "in" => "r#in".to_string(),
        "ref" => "r#ref".to_string(),
        "mut" => "r#mut".to_string(),
        "const" => "r#const".to_string(),
        "static" => "r#static".to_string(),
        "async" => "r#async".to_string(),
        "await" => "r#await".to_string(),
        "move" => "r#move".to_string(),
        "return" => "r#return".to_string(),
        "break" => "r#break".to_string(),
        "continue" => "r#continue".to_string(),
        _ => result,
    }
}

fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for c in s.chars() {
        if c == '_' || c == '-' || c == ' ' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_uppercase().next().unwrap());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    result
}
