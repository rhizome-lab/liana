#!/usr/bin/env bash
# Generate Rust code from an OpenAPI schema
#
# Usage: ./scripts/generate.sh <name>
# Example: ./scripts/generate.sh github

set -euo pipefail

NAME="$1"
SCHEMA_PATH="schemas/$NAME/openapi.json"
OUTPUT_DIR="crates/openapi-$NAME/src/generated"

if [ ! -f "$SCHEMA_PATH" ]; then
    echo "Error: Schema not found at $SCHEMA_PATH"
    echo "Run ./scripts/fetch-schema.sh $NAME <url> first"
    exit 1
fi

mkdir -p "$OUTPUT_DIR"

echo "Generating Rust code for $NAME..."
cargo run -p liana-codegen -- \
    --schema "$SCHEMA_PATH" \
    --output "$OUTPUT_DIR"

echo "Generated code at $OUTPUT_DIR"
