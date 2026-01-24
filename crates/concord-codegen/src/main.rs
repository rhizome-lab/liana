use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

mod generator;
mod parser;

#[derive(Parser)]
#[command(name = "liana-codegen")]
#[command(about = "Generate type-safe API bindings from schemas")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate bindings from an `OpenAPI` schema.
    Openapi {
        /// Path to `OpenAPI` schema (JSON or YAML).
        #[arg(short, long)]
        schema: PathBuf,

        /// Output directory for generated code
        #[arg(short, long)]
        output: PathBuf,

        /// Target language
        #[arg(short, long, default_value = "rust")]
        target: String,
    },

    /// Dump IR for debugging.
    DumpIr {
        /// Path to `OpenAPI` schema (JSON or YAML).
        #[arg(short, long)]
        schema: PathBuf,

        /// Output format (json or yaml)
        #[arg(short, long, default_value = "json")]
        format: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Openapi {
            schema,
            output,
            target,
        } => {
            let ir = parser::openapi::parse(&schema)?;

            match target.as_str() {
                "rust" => generator::rust::generate(&ir, &output)?,
                other => anyhow::bail!("Unknown target: {other}"),
            }

            println!("Generated {} bindings in {}", target, output.display());
        }

        Command::DumpIr { schema, format } => {
            let ir = parser::openapi::parse(&schema)?;

            let output = match format.as_str() {
                "json" => serde_json::to_string_pretty(&ir)?,
                "yaml" => serde_yaml::to_string(&ir)?,
                other => anyhow::bail!("Unknown format: {other}"),
            };

            println!("{output}");
        }
    }

    Ok(())
}
