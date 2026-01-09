//! `OpenAPI` schema parser.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use liana_core::{
    Annotation, Field, Function, Item, Metadata, Module, Param, Type, TypeKind, Variant,
};
use openapiv3::{OpenAPI, ReferenceOr, Schema, SchemaKind, StringType, Type as OaType};

/// Parse an `OpenAPI` schema file into IR.
pub fn parse(path: &Path) -> Result<Module> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;

    let spec: OpenAPI = if path.extension().is_some_and(|e| e == "yaml" || e == "yml") {
        serde_yaml::from_str(&content)?
    } else {
        serde_json::from_str(&content)?
    };

    let converter = Converter::new(&spec);
    converter.convert()
}

struct Converter<'a> {
    spec: &'a OpenAPI,
    items: Vec<Item>,
}

impl<'a> Converter<'a> {
    fn new(spec: &'a OpenAPI) -> Self {
        Self {
            spec,
            items: Vec::new(),
        }
    }

    fn convert(mut self) -> Result<Module> {
        // Convert schemas (components/schemas)
        if let Some(components) = &self.spec.components {
            for (name, schema_ref) in &components.schemas {
                if let ReferenceOr::Item(schema) = schema_ref {
                    let typ = self.convert_schema(schema, Some(name.clone()))?;
                    self.items.push(Item::Type(typ));
                }
            }
        }

        // Convert paths to functions
        for (path, path_item_ref) in &self.spec.paths.paths {
            if let ReferenceOr::Item(path_item) = path_item_ref {
                self.convert_path_item(path, path_item)?;
            }
        }

        let title = self.spec.info.title.clone();

        Ok(Module {
            name: to_module_name(&title),
            items: self.items,
            submodules: Vec::new(),
            annotations: Vec::new(),
            metadata: Metadata {
                docs: self.spec.info.description.clone(),
                ..Default::default()
            },
        })
    }

    #[allow(clippy::unnecessary_wraps)]
    fn convert_schema(&self, schema: &Schema, name: Option<String>) -> Result<Type> {
        let (kind, args) = match &schema.schema_kind {
            SchemaKind::Type(typ) => self.convert_schema_type(typ),
            SchemaKind::OneOf { one_of } => {
                let members = one_of
                    .iter()
                    .filter_map(|r| self.resolve_schema_ref(r).ok())
                    .collect();
                (TypeKind::Union { members }, Vec::new())
            }
            SchemaKind::AllOf { all_of } => {
                let members = all_of
                    .iter()
                    .filter_map(|r| self.resolve_schema_ref(r).ok())
                    .collect();
                (TypeKind::Intersection { members }, Vec::new())
            }
            SchemaKind::AnyOf { any_of } => {
                let members = any_of
                    .iter()
                    .filter_map(|r| self.resolve_schema_ref(r).ok())
                    .collect();
                (TypeKind::Union { members }, Vec::new())
            }
            SchemaKind::Not { .. } => {
                // Not types are tricky, just use Any for now
                (
                    TypeKind::Ref {
                        name: "Any".to_string(),
                    },
                    Vec::new(),
                )
            }
            SchemaKind::Any(any) => {
                // Try to infer from properties
                if any.properties.is_empty() {
                    (
                        TypeKind::Ref {
                            name: "Any".to_string(),
                        },
                        Vec::new(),
                    )
                } else {
                    let fields = any
                        .properties
                        .iter()
                        .map(|(name, schema_ref)| {
                            let typ = self
                                .resolve_boxed_schema_ref(schema_ref)
                                .unwrap_or_else(|_| Type::reference("Any"));
                            let required = any.required.contains(name);
                            let typ = if required {
                                typ
                            } else {
                                Type::generic("Option", vec![typ])
                            };
                            Field {
                                name: Some(name.clone()),
                                typ,
                                annotations: Vec::new(),
                            }
                        })
                        .collect();
                    (TypeKind::Struct { fields }, Vec::new())
                }
            }
        };

        let mut annotations = Vec::new();

        // Add format annotation if present
        if let SchemaKind::Type(OaType::String(StringType { format, .. })) = &schema.schema_kind {
            if let openapiv3::VariantOrUnknownOrEmpty::Item(f) = format {
                annotations.push(Annotation::with_string(
                    "format",
                    format!("{f:?}").to_lowercase(),
                ));
            } else if let openapiv3::VariantOrUnknownOrEmpty::Unknown(f) = format {
                annotations.push(Annotation::with_string("format", f.clone()));
            }
        }

        Ok(Type {
            kind,
            name,
            params: Vec::new(),
            args,
            annotations,
            metadata: Metadata {
                docs: schema.schema_data.description.clone(),
                ..Default::default()
            },
        })
    }

    fn convert_schema_type(&self, typ: &OaType) -> (TypeKind, Vec<Type>) {
        match typ {
            OaType::String(s) => {
                if s.enumeration.is_empty() {
                    (
                        TypeKind::Ref {
                            name: "String".to_string(),
                        },
                        Vec::new(),
                    )
                } else {
                    // Generate enum variants from string values
                    let variants = s
                        .enumeration
                        .iter()
                        .filter_map(|v| v.as_ref())
                        .map(|v| Variant {
                            name: to_pascal_case(v),
                            fields: Vec::new(),
                            annotations: vec![Annotation::with_string("serde_rename", v.clone())],
                        })
                        .collect();
                    (TypeKind::Enum { variants }, Vec::new())
                }
            }
            OaType::Number(_) => (
                TypeKind::Ref {
                    name: "f64".to_string(),
                },
                Vec::new(),
            ),
            OaType::Integer(_) => (
                TypeKind::Ref {
                    name: "i64".to_string(),
                },
                Vec::new(),
            ),
            OaType::Boolean(_) => (
                TypeKind::Ref {
                    name: "bool".to_string(),
                },
                Vec::new(),
            ),
            OaType::Object(obj) => {
                let fields = obj
                    .properties
                    .iter()
                    .map(|(name, schema_ref)| {
                        let typ = self
                            .resolve_boxed_schema_ref(schema_ref)
                            .unwrap_or_else(|_| Type::reference("Any"));
                        let required = obj.required.contains(name);
                        let typ = if required {
                            typ
                        } else {
                            Type::generic("Option", vec![typ])
                        };
                        Field {
                            name: Some(name.clone()),
                            typ,
                            annotations: Vec::new(),
                        }
                    })
                    .collect();
                (TypeKind::Struct { fields }, Vec::new())
            }
            OaType::Array(arr) => {
                let item_type = arr
                    .items
                    .as_ref()
                    .and_then(|r| self.resolve_boxed_schema_ref(r).ok())
                    .unwrap_or_else(|| Type::reference("Any"));
                (
                    TypeKind::Ref {
                        name: "Vec".to_string(),
                    },
                    vec![item_type],
                )
            }
        }
    }

    fn resolve_schema_ref(&self, schema_ref: &ReferenceOr<Schema>) -> Result<Type> {
        match schema_ref {
            ReferenceOr::Reference { reference } => {
                // Extract name from #/components/schemas/Name
                let name = reference
                    .strip_prefix("#/components/schemas/")
                    .unwrap_or(reference);
                Ok(Type::reference(name))
            }
            ReferenceOr::Item(schema) => self.convert_schema(schema, None),
        }
    }

    fn resolve_boxed_schema_ref(&self, schema_ref: &ReferenceOr<Box<Schema>>) -> Result<Type> {
        match schema_ref {
            ReferenceOr::Reference { reference } => {
                let name = reference
                    .strip_prefix("#/components/schemas/")
                    .unwrap_or(reference);
                Ok(Type::reference(name))
            }
            ReferenceOr::Item(schema) => self.convert_schema(schema, None),
        }
    }

    fn convert_path_item(&mut self, path: &str, item: &openapiv3::PathItem) -> Result<()> {
        let operations = [
            ("get", &item.get),
            ("post", &item.post),
            ("put", &item.put),
            ("delete", &item.delete),
            ("patch", &item.patch),
            ("head", &item.head),
            ("options", &item.options),
            ("trace", &item.trace),
        ];

        for (method, op) in operations {
            if let Some(operation) = op {
                let func = self.convert_operation(path, method, operation)?;
                self.items.push(Item::Function(func));
            }
        }

        Ok(())
    }

    fn convert_operation(
        &self,
        path: &str,
        method: &str,
        op: &openapiv3::Operation,
    ) -> Result<Function> {
        let name = op.operation_id.clone().unwrap_or_else(|| {
            format!(
                "{method}_{}",
                path.replace('/', "_").replace(['{', '}'], "")
            )
        });

        let mut args = Vec::new();

        // Convert parameters
        for param_ref in &op.parameters {
            if let ReferenceOr::Item(param) = param_ref {
                let param_data = match param {
                    openapiv3::Parameter::Query { parameter_data, .. }
                    | openapiv3::Parameter::Header { parameter_data, .. }
                    | openapiv3::Parameter::Path { parameter_data, .. }
                    | openapiv3::Parameter::Cookie { parameter_data, .. } => parameter_data,
                };

                let typ = match &param_data.format {
                    openapiv3::ParameterSchemaOrContent::Schema(s) => self
                        .resolve_schema_ref(s)
                        .unwrap_or_else(|_| Type::reference("String")),
                    openapiv3::ParameterSchemaOrContent::Content(_) => Type::reference("String"),
                };

                let typ = if param_data.required {
                    typ
                } else {
                    Type::generic("Option", vec![typ])
                };

                args.push(Param {
                    name: Some(param_data.name.clone()),
                    typ,
                    default: None,
                    annotations: Vec::new(),
                });
            }
        }

        // Convert request body
        if let Some(ReferenceOr::Item(body)) = &op.request_body
            && let Some(content) = body.content.get("application/json")
            && let Some(schema_ref) = &content.schema
        {
            let typ = self.resolve_schema_ref(schema_ref)?;
            args.push(Param {
                name: Some("body".to_string()),
                typ,
                default: None,
                annotations: Vec::new(),
            });
        }

        // Convert response
        let ret = op
            .responses
            .default
            .as_ref()
            .or_else(|| {
                op.responses
                    .responses
                    .get(&openapiv3::StatusCode::Code(200))
            })
            .and_then(|r| match r {
                ReferenceOr::Item(resp) => resp.content.get("application/json"),
                ReferenceOr::Reference { .. } => None,
            })
            .and_then(|content| content.schema.as_ref())
            .and_then(|s| self.resolve_schema_ref(s).ok())
            .unwrap_or_else(|| Type::reference("Unit"));

        let ret = Type::generic("Result", vec![ret, Type::reference("ApiError")]);

        Ok(Function {
            name,
            params: Vec::new(),
            args,
            ret,
            annotations: vec![
                Annotation::with_string("http_method", method.to_uppercase()),
                Annotation::with_string("http_path", path),
            ],
            metadata: Metadata {
                docs: op.description.clone().or_else(|| op.summary.clone()),
                ..Default::default()
            },
        })
    }
}

fn to_module_name(title: &str) -> String {
    title
        .to_lowercase()
        .replace(|c: char| !c.is_alphanumeric(), "_")
        .trim_matches('_')
        .to_string()
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
