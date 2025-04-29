#!/bin/bash

# This script tests the Earth Search STAC API oracle component
# by sending a sample STAC query to it

# Set up environment variables
COMPILED_DIR="./compiled"
COMPONENT_PATH="${COMPILED_DIR}/wavs_regen_oracle.wasm"

# Default STAC query based on example
STAC_QUERY='{
  "collections": ["sentinel-2-l2a"],
  "bbox": [-122.52, 37.70, -122.35, 37.83],
  "datetime": "2024-06-01T00:00:00Z/2024-06-30T23:59:59Z",
  "limit": 1,
  "query": {
    "eo:cloud_cover": {
      "lt": 10
    }
  }
}'

# If a command-line argument is provided, use it as the STAC query JSON
if [ $# -eq 1 ]; then
  STAC_QUERY="$1"
fi

# Create a temporary file for the query
QUERY_FILE=$(mktemp)
echo "$STAC_QUERY" > "$QUERY_FILE"

echo "Testing STAC API oracle with query:"
echo "$STAC_QUERY"
echo "------------------------------------"

# Run the component with the query
cargo component run \
  --no-default-features \
  --target wasm32-wasip2 \
  --manifest-path ./components/regen-oracle/Cargo.toml \
  --release \
  "$COMPONENT_PATH" \
  --data @"$QUERY_FILE"

# Clean up
rm "$QUERY_FILE"