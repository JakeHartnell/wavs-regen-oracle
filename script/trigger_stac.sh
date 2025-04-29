#!/bin/bash

# This script triggers a STAC API query by calling the deployed trigger contract

# Set up environment variables
TRIGGER_CONTRACT=${SERVICE_TRIGGER_ADDR:-$(jq -r .deployedTo .docker/trigger.json)}
RPC_URL=${RPC_URL:-"http://localhost:8545"}

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

echo "Triggering STAC API oracle with query:"
echo "$STAC_QUERY"
echo "------------------------------------"
echo "Trigger Contract: $TRIGGER_CONTRACT"
echo "RPC URL: $RPC_URL"
echo "------------------------------------"

# Trigger the contract with the STAC query
forge script ./script/StacTrigger.s.sol "$TRIGGER_CONTRACT" "$STAC_QUERY" \
  --sig 'run(string,string)' \
  --rpc-url "$RPC_URL" \
  --broadcast

# Clean up
rm "$QUERY_FILE"

echo "------------------------------------"
echo "To check results, use 'make get-trigger' and then 'TRIGGER_ID=X make show-result'"