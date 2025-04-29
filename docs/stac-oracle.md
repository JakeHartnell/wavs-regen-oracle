# Earth Search STAC API Oracle

This document provides an overview of the Earth Search STAC API Oracle, a service that allows querying for Sentinel-2 satellite data through the [Earth Search STAC API](https://earth-search.aws.element84.com/v1/api.html) and processing the results to calculate NDVI (Normalized Difference Vegetation Index).

## Overview

The Earth Search STAC API Oracle:

1. Accepts a STAC query submitted to the trigger contract
2. Queries the Earth Search API to find relevant satellite imagery
3. Downloads the red and NIR bands from the first matching result
4. Calculates NDVI (vegetation health index) from these bands
5. Uploads the NDVI image and metadata to IPFS
6. Returns the IPFS URI containing the result metadata

## STAC Query Format

Queries should follow the STAC API search format. Here's an example:

```json
{
  "collections": ["sentinel-2-l2a"],
  "bbox": [-122.52, 37.70, -122.35, 37.83],
  "datetime": "2024-06-01T00:00:00Z/2024-06-30T23:59:59Z",
  "limit": 1,
  "query": {
    "eo:cloud_cover": {
      "lt": 10
    }
  }
}
```

Key parameters:
- `collections`: Set to "sentinel-2-l2a" for Sentinel-2 Level 2A data
- `bbox`: Bounding box in format [min_lon, min_lat, max_lon, max_lat]
- `datetime`: ISO-8601 time range for the search
- `limit`: Maximum number of results to return
- `query`: Additional filters like maximum cloud cover percentage

## Response Format

The oracle returns an IPFS URI pointing to JSON metadata with the following structure:

```json
{
  "metadata": {
    "id": "S2B_10SEG_20240627_0_L2A",
    "datetime": "2024-06-27T19:04:13.374000Z",
    "bbox": [-123.00021625365562, 36.951488600423914, -121.75065723902861, 37.947580558603775],
    "cloud_cover": 3.221058,
    "vegetation_percentage": 25.044519,
    "ndvi_stats": {
      "min": 0.0,
      "max": 1.0,
      "mean": 0.5
    },
    "source_red_band": "https://sentinel-cogs.s3.us-west-2.amazonaws.com/sentinel-s2-l2a-cogs/10/S/EG/2024/6/S2B_10SEG_20240627_0_L2A/B04.tif",
    "source_nir_band": "https://sentinel-cogs.s3.us-west-2.amazonaws.com/sentinel-s2-l2a-cogs/10/S/EG/2024/6/S2B_10SEG_20240627_0_L2A/B08.tif"
  },
  "ndvi_image_uri": "ipfs://Qm..."
}
```

The `ndvi_image_uri` points to a PNG visualization of the NDVI data, where:
- High NDVI values (healthy vegetation) appear green
- Low NDVI values (no vegetation) appear red

## Usage

### Testing Locally

To test the oracle locally without submitting to the blockchain:

```bash
# Build the WASM component
make wasi-build

# Test with the default query
make test-stac

# Test with a custom query
STAC_QUERY='{"collections":["sentinel-2-l2a"],"bbox":[-122.52,37.70,-122.35,37.83],"datetime":"2024-06-01T00:00:00Z/2024-06-30T23:59:59Z","limit":1,"query":{"eo:cloud_cover":{"lt":10}}}' make test-stac
```

### Deploying the Service

Follow the standard deployment steps in the project README:

1. Build the components:
   ```bash
   COMPONENT_FILENAME=wavs_regen_oracle.wasm make wasi-build
   ```

2. Deploy the service:
   ```bash
   COMPONENT_FILENAME=wavs_regen_oracle.wasm AGGREGATOR_URL=http://127.0.0.1:8001 sh ./script/build_service.sh
   SERVICE_CONFIG_FILE=.docker/service.json make deploy-service
   ```

### Triggering a Query

To trigger a STAC query on-chain:

```bash
# Use the default query
make trigger-stac

# Specify a custom query
STAC_QUERY='{"collections":["sentinel-2-l2a"],"bbox":[-122.52,37.70,-122.35,37.83],"datetime":"2024-06-01T00:00:00Z/2024-06-30T23:59:59Z","limit":1,"query":{"eo:cloud_cover":{"lt":10}}}' make trigger-stac
```

### Viewing Results

After a query has been processed:

1. Get the latest trigger ID:
   ```bash
   make get-trigger
   ```

2. View the result with the trigger ID:
   ```bash
   TRIGGER_ID=1 make show-result
   ```

## Configuration

The oracle supports the following environment variables:

- `WAVS_ENV_EARTH_SEARCH_API`: Earth Search API endpoint (default: https://earth-search.aws.element84.com/v1/search)
- `WAVS_ENV_IPFS_ENDPOINT`: IPFS upload endpoint (default: https://node.lighthouse.storage/api/v0/add)
- `WAVS_ENV_LIGHTHOUSE_API_KEY`: API key for Lighthouse storage (required for IPFS uploads)

## Windowed Reading

To efficiently handle the large Sentinel-2 GeoTIFF files, this oracle uses windowed reading:

1. The oracle extracts the bounding box (bbox) from the STAC query
2. It calculates the pixel coordinates for this geographic bbox using the raster's transform parameters
3. It downloads only a portion of the data that covers the requested bbox
4. This significantly reduces memory usage and processing time

The implementation:
- Uses the image's projection transform to map between geographic and pixel coordinates
- Limits download size to avoid memory issues
- Provides a fallback mechanism to use simulated data if windowed reading fails

## NDVI Calculation

NDVI (Normalized Difference Vegetation Index) is calculated using the formula:

```
NDVI = (NIR - RED) / (NIR + RED)
```

Where:
- NIR is the near-infrared band (Band 8 in Sentinel-2)
- RED is the red band (Band 4 in Sentinel-2)

NDVI values range from -1 to 1, where:
- Values > 0.5 indicate dense vegetation
- Values 0.2-0.5 indicate sparse vegetation
- Values < 0.2 indicate no vegetation (bare soil, water, etc.)