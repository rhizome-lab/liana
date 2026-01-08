#!/usr/bin/env bash
# Fetch an OpenAPI schema and save it to schemas/
#
# Usage: ./scripts/fetch-schema.sh <name> <url>
# Example: ./scripts/fetch-schema.sh github https://raw.githubusercontent.com/github/rest-api-description/main/descriptions/api.github.com/api.github.com.json

set -euo pipefail

NAME="$1"
URL="$2"

mkdir -p "schemas/$NAME"

echo "Fetching $NAME schema from $URL..."
curl -sSL "$URL" -o "schemas/$NAME/openapi.json"

echo "Schema saved to schemas/$NAME/openapi.json"
