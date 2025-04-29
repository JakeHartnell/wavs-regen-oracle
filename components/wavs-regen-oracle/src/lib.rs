mod ipfs;
mod trigger;
use trigger::{decode_trigger_event, encode_trigger_output, Destination};
use wavs_wasi_utils::http::{fetch_json, http_request_get, http_request_post_json};
pub mod bindings;
use crate::bindings::{export, Guest, TriggerAction, WasmResponse};
use crate::ipfs::upload_nft_content;
use geo::Rect;
use image::GenericImage;
use image::{DynamicImage, GenericImageView};
use ndarray::{Array2, Zip};
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read};
use wstd::{http::HeaderValue, runtime::block_on};

struct Component;
export!(Component with_types_in bindings);

impl Guest for Component {
    /// Main entry point for the Earth Search STAC API oracle component.
    /// WAVS is subscribed to watch for events emitted by the blockchain.
    /// When WAVS observes an event is emitted, it will internally route the event and its data to this function (component).
    /// The processing then occurs before the output is returned back to WAVS to be submitted to the blockchain by the operator(s).
    ///
    /// This function:
    /// 1. Receives a trigger action containing encoded STAC query parameters
    /// 2. Decodes the input to get the STAC query (in JSON)
    /// 3. Calls the Earth Search API with the query parameters
    /// 4. Downloads red and NIR bands to calculate NDVI
    /// 5. Uploads data to IPFS and returns the URI
    fn run(action: TriggerAction) -> std::result::Result<Option<WasmResponse>, String> {
        let (trigger_id, req, dest) =
            decode_trigger_event(action.data).map_err(|e| e.to_string())?;

        // Convert bytes to string for STAC query
        let stac_query = std::str::from_utf8(&req).map_err(|e| e.to_string())?;
        println!("STAC query: {}", stac_query);

        let res = block_on(async move {
            // Get the API endpoint from environment variables, with fallback
            let api_endpoint = std::env::var("WAVS_ENV_EARTH_SEARCH_API")
                .unwrap_or_else(|_| "https://earth-search.aws.element84.com/v1/search".to_string());
            let ipfs_endpoint = std::env::var("WAVS_ENV_IPFS_ENDPOINT")
                .unwrap_or_else(|_| "https://node.lighthouse.storage/api/v0/add".to_string());

            // Query the Earth Search API
            let stac_response = query_earth_search(&api_endpoint, stac_query).await?;
            println!("Found {} features", stac_response.features.len());

            if stac_response.features.is_empty() {
                return Err("No features found for the given query".to_string());
            }

            // Process first feature
            let feature = &stac_response.features[0];
            println!("Processing feature with ID: {}", feature.id);

            // Extract red and NIR band URLs and metadata
            let red_asset = feature
                .assets
                .get("red")
                .ok_or_else(|| "Red band not found in feature assets".to_string())?;

            let nir_asset = feature
                .assets
                .get("nir")
                .ok_or_else(|| "NIR band not found in feature assets".to_string())?;

            let red_url = red_asset
                .get("href")
                .and_then(|href| href.as_str())
                .ok_or_else(|| "Red band URL not found".to_string())?;

            let nir_url = nir_asset
                .get("href")
                .and_then(|href| href.as_str())
                .ok_or_else(|| "NIR band URL not found".to_string())?;

            println!("Red band URL: {}", red_url);
            println!("NIR band URL: {}", nir_url);

            // Extract transform and shape from asset metadata
            let red_transform = red_asset
                .get("proj:transform")
                .and_then(|t| t.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect::<Vec<f64>>())
                .unwrap_or_else(|| vec![10.0, 0.0, 499980.0, 0.0, -10.0, 4200000.0]); // Default transform

            let red_shape = red_asset
                .get("proj:shape")
                .and_then(|s| s.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_i64()).collect::<Vec<i64>>())
                .unwrap_or_else(|| vec![10980, 10980]); // Default shape

            let nir_transform = nir_asset
                .get("proj:transform")
                .and_then(|t| t.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect::<Vec<f64>>())
                .unwrap_or_else(|| vec![10.0, 0.0, 499980.0, 0.0, -10.0, 4200000.0]); // Default transform

            let nir_shape = nir_asset
                .get("proj:shape")
                .and_then(|s| s.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_i64()).collect::<Vec<i64>>())
                .unwrap_or_else(|| vec![10980, 10980]); // Default shape

            println!("Red band transform: {:?}, shape: {:?}", red_transform, red_shape);
            println!("NIR band transform: {:?}, shape: {:?}", nir_transform, nir_shape);

            // Using bounding box for windowed reading
            let bbox = &feature.bbox;

            // Download the red band data
            println!("Downloading red band data...");
            let red_data = download_band_window(red_url, bbox, &red_transform, &red_shape).await?;

            // Download the NIR band data
            println!("Downloading NIR band data...");
            let nir_data = download_band_window(nir_url, bbox, &nir_transform, &nir_shape).await?;

            // Calculate NDVI (simplified for this example)
            let ndvi_image = calculate_ndvi(&red_data, &nir_data)?;

            // Create metadata
            let metadata = NdviMetadata {
                id: feature.id.clone(),
                datetime: feature
                    .properties
                    .get("datetime")
                    .and_then(|dt| dt.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                bbox: feature.bbox.clone(),
                cloud_cover: feature
                    .properties
                    .get("eo:cloud_cover")
                    .and_then(|cc| cc.as_f64())
                    .unwrap_or(0.0),
                vegetation_percentage: feature
                    .properties
                    .get("s2:vegetation_percentage")
                    .and_then(|vp| vp.as_f64())
                    .unwrap_or(0.0),
                ndvi_stats: NdviStats {
                    min: 0.0,  // Simplified
                    max: 1.0,  // Simplified
                    mean: 0.5, // Simplified
                },
                source_red_band: red_url.to_string(),
                source_nir_band: nir_url.to_string(),
            };

            // Upload NDVI image to IPFS
            let ndvi_uri = upload_nft_content("image/png", &ndvi_image, &ipfs_endpoint)
                .await
                .map_err(|e| e.to_string())?;
            println!("NDVI image uploaded to IPFS: {}", ndvi_uri);

            // Add NDVI URI to metadata
            let metadata_with_uri = NdviResultMetadata { metadata, ndvi_image_uri: ndvi_uri };

            // Upload metadata to IPFS
            let metadata_json = serde_json::to_string(&metadata_with_uri)
                .map_err(|e| format!("Failed to serialize metadata: {}", e))?;

            let metadata_uri =
                upload_nft_content("application/json", metadata_json.as_bytes(), &ipfs_endpoint)
                    .await
                    .map_err(|e| e.to_string())?;
            println!("Metadata uploaded to IPFS: {}", metadata_uri);

            // Return the metadata URI as the result
            let result = OracleResult { metadata_uri, feature_id: feature.id.clone() };

            serde_json::to_vec(&result).map_err(|e| e.to_string())
        })?;

        let output = match dest {
            Destination::Ethereum => Some(encode_trigger_output(trigger_id, &res)),
            Destination::CliOutput => Some(WasmResponse { payload: res.into(), ordering: None }),
        };
        Ok(output)
    }
}

/// Result returned by the oracle
#[derive(Debug, Serialize, Deserialize)]
pub struct OracleResult {
    pub metadata_uri: String,
    pub feature_id: String,
}

/// Metadata for the NDVI calculation
#[derive(Debug, Serialize, Deserialize)]
pub struct NdviMetadata {
    pub id: String,
    pub datetime: String,
    pub bbox: Vec<f64>,
    pub cloud_cover: f64,
    pub vegetation_percentage: f64,
    pub ndvi_stats: NdviStats,
    pub source_red_band: String,
    pub source_nir_band: String,
}

/// Statistics for the NDVI calculation
#[derive(Debug, Serialize, Deserialize)]
pub struct NdviStats {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
}

/// Final metadata including the IPFS URI for the NDVI image
#[derive(Debug, Serialize, Deserialize)]
pub struct NdviResultMetadata {
    pub metadata: NdviMetadata,
    pub ndvi_image_uri: String,
}

/// Fetches satellite data from Earth Search STAC API
///
/// # Arguments
/// * `api_endpoint` - The Earth Search API endpoint
/// * `query_json` - STAC query as a JSON string
///
/// # Returns
/// * The STAC API response with satellite data matching the query
async fn query_earth_search(api_endpoint: &str, query_json: &str) -> Result<StacResponse, String> {
    // Parse the query JSON to ensure it's valid
    let stac_query: serde_json::Value =
        serde_json::from_str(query_json).map_err(|e| format!("Invalid STAC query JSON: {}", e))?;

    let mut req = http_request_post_json(api_endpoint, stac_query).map_err(|e| e.to_string())?;
    // req.headers_mut().insert("Accept", HeaderValue::from_static("application/json"));
    // req.headers_mut().insert("Content-Type", HeaderValue::from_static("application/json"));
    // req.headers_mut()
    //     .insert("User-Agent", HeaderValue::from_static("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36"));

    // Set the request body to the query JSON
    // *req.body_mut() = stac_query.to_string().into_bytes().into();

    // Fetch the JSON response
    let stac_response: StacResponse = fetch_json(req).await.map_err(|e| e.to_string())?;

    Ok(stac_response)
}

/// Downloads a band image from the given URL
///
/// # Arguments
/// * `url` - URL to download the band from
///
/// # Returns
/// * The raw bytes of the band image
/// Downloads a subset of a band image from the given URL using windowed reads
///
/// # Arguments
/// * `url` - URL to download the band from
/// * `bbox` - Bounding box [min_lon, min_lat, max_lon, max_lat]
/// * `transform` - Geo transform parameters from the asset metadata
/// * `shape` - [height, width] dimensions from the asset metadata
///
/// # Returns
/// * The windowed subset of the band image
async fn download_band_window(
    url: &str,
    bbox: &[f64],
    transform: &[f64],
    shape: &[i64],
) -> Result<Vec<u8>, String> {
    println!("Downloading windowed data from URL: {}", url);

    // Extract transform parameters
    let pixel_width = transform[0];
    let x_origin = transform[2];
    let pixel_height = transform[4]; // Usually negative
    let y_origin = transform[5];

    // Extract shape
    let width = shape[1] as f64;
    let height = shape[0] as f64;

    // Calculate pixel coordinates from geo coordinates
    let min_x = ((bbox[0] - x_origin) / pixel_width).floor() as i64;
    let max_y = ((bbox[1] - y_origin) / pixel_height).floor() as i64;
    let max_x = ((bbox[2] - x_origin) / pixel_width).ceil() as i64;
    let min_y = ((bbox[3] - y_origin) / pixel_height).ceil() as i64;

    // Clamp to image bounds
    let min_x = min_x.max(0).min(width as i64);
    let min_y = min_y.max(0).min(height as i64);
    let max_x = max_x.max(0).min(width as i64);
    let max_y = max_y.max(0).min(height as i64);

    // Calculate window dimensions
    let window_width = (max_x - min_x) as u32;
    let window_height = (max_y - min_y) as u32;

    // Further constrain window size if it's too large
    // This helps prevent out-of-gas errors
    const MAX_WINDOW_DIMENSION: u32 = 500;
    let scale_factor_x = if window_width > MAX_WINDOW_DIMENSION {
        (window_width as f32) / (MAX_WINDOW_DIMENSION as f32)
    } else {
        1.0
    };

    let scale_factor_y = if window_height > MAX_WINDOW_DIMENSION {
        (window_height as f32) / (MAX_WINDOW_DIMENSION as f32)
    } else {
        1.0
    };

    let scale_factor = scale_factor_x.max(scale_factor_y);
    let effective_window_width = (window_width as f32 / scale_factor) as u32;
    let effective_window_height = (window_height as f32 / scale_factor) as u32;

    println!(
        "Original window coordinates: x={}-{}, y={}-{}, width={}, height={}",
        min_x, max_x, min_y, max_y, window_width, window_height
    );

    println!(
        "Reduced window size: width={}, height={}, scale_factor={}",
        effective_window_width, effective_window_height, scale_factor
    );

    // For COG GeoTIFFs, we would ideally use range requests to get only the needed tiles
    // For this example, we'll use a simpler approach to demonstrate the concept

    // Limit the download size to reduce memory usage
    let mut req = http_request_get(url).map_err(|e| e.to_string())?;
    req.headers_mut().insert("Accept", HeaderValue::from_static("*/*"));
    req.headers_mut()
        .insert("User-Agent", HeaderValue::from_static("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36"));

    // Set a very small byte range to minimize data transfer
    req.headers_mut().insert("Range", HeaderValue::from_static("bytes=0-100000"));

    let mut response = wstd::http::Client::new()
        .send(req)
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.status().is_success() && response.status().as_u16() != 206 {
        // 206 is Partial Content, which is success for a range request
        return Err(format!("Failed to download band. Status: {:?}", response.status()));
    }

    // Read just a small portion for demonstration
    let mut body_buf = Vec::new();
    let mut bytes_read = 0;
    let max_bytes = 100_000; // Reduce to 100KB max to save gas

    let mut buffer = [0u8; 4096]; // Smaller buffer size
    loop {
        let n = wstd::io::AsyncRead::read(response.body_mut(), &mut buffer)
            .await
            .map_err(|e| format!("Failed to read response body: {}", e))?;

        if n == 0 {
            break;
        }

        body_buf.extend_from_slice(&buffer[..n]);
        bytes_read += n;

        if bytes_read >= max_bytes {
            println!("Reached byte limit ({}KB). Truncating download.", max_bytes / 1000);
            break;
        }
    }

    println!("Downloaded {} bytes (window subsample)", body_buf.len());
    Ok(body_buf)
}

/// Simulates downloading a band image (temporary replacement while implementing windowed reads)
///
/// # Arguments
/// * `url` - URL to download the band from
///
/// # Returns
/// * A small sample of the band image data
async fn download_band(url: &str) -> Result<Vec<u8>, String> {
    println!("Downloading band data from URL: {}", url);

    // Set up HTTP request with limited size to save gas
    let mut req = http_request_get(url).map_err(|e| e.to_string())?;
    req.headers_mut().insert("Accept", HeaderValue::from_static("*/*"));
    req.headers_mut()
        .insert("User-Agent", HeaderValue::from_static("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36"));

    // Set a byte range to minimize data transfer - just download the first part of the file
    // This is enough to get header info and some data while saving gas
    req.headers_mut().insert("Range", HeaderValue::from_static("bytes=0-50000"));

    let mut response = wstd::http::Client::new()
        .send(req)
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.status().is_success() && response.status().as_u16() != 206 {
        // 206 is Partial Content response, which is success for a range request
        return Err(format!("Failed to download band. Status: {:?}", response.status()));
    }

    // Read the response data with a size limit
    let mut body_buf = Vec::new();
    let mut bytes_read = 0;
    let max_bytes = 50_000; // 50KB max to avoid excessive gas usage

    let mut buffer = [0u8; 4096]; // Use a reasonably sized buffer
    loop {
        let n = wstd::io::AsyncRead::read(response.body_mut(), &mut buffer)
            .await
            .map_err(|e| format!("Failed to read response body: {}", e))?;

        if n == 0 {
            break;
        }

        body_buf.extend_from_slice(&buffer[..n]);
        bytes_read += n;

        if bytes_read >= max_bytes {
            println!("Reached byte limit ({}KB). Truncating download.", max_bytes / 1000);
            break;
        }
    }

    println!("Downloaded {} bytes", body_buf.len());
    Ok(body_buf)
}

/// Calculates NDVI from red and NIR bands
///
/// # Arguments
/// * `red_data` - Raw data for the red band
/// * `nir_data` - Raw data for the NIR band
///
/// # Returns
/// * PNG image data of the NDVI visualization
fn calculate_ndvi(red_data: &[u8], nir_data: &[u8]) -> Result<Vec<u8>, String> {
    // NOTE: In a real implementation, we would need to properly parse the GeoTIFF files,
    // extract the actual pixel values considering scale and offset, handle nodata values,
    // coordinate systems, etc. This is a simplified example.

    println!("Calculating NDVI (simplified)");

    // For this example, we'll create a mock NDVI image
    // In a real implementation, we would:
    // 1. Parse the GeoTIFF files
    // 2. Extract the pixel values
    // 3. Apply the NDVI formula: (NIR - RED) / (NIR + RED)
    // 4. Create a visualization

    // Create a smaller image to reduce gas usage
    let width = 200; // Reduced from 500
    let height = 200; // Reduced from 500
    let mut img = DynamicImage::new_rgb8(width, height);

    // Pre-calculate the NDVI values in a smaller grid
    // This reduces the number of pixels we need to set
    let grid_size = 20; // 20x20 grid = 400 pixels instead of 40,000
    let cell_width = width / grid_size;
    let cell_height = height / grid_size;

    for grid_y in 0..grid_size {
        for grid_x in 0..grid_size {
            // Create a gradient from bottom-left to top-right
            // This is just for visualization purposes
            let ndvi_value =
                (grid_x as f32 / grid_size as f32 + grid_y as f32 / grid_size as f32) / 2.0;

            // Apply a color scale (red to green)
            let r = ((1.0 - ndvi_value) * 255.0) as u8;
            let g = (ndvi_value * 255.0) as u8;
            let b = 0;

            // Fill the cell with the same color
            let start_x = grid_x * cell_width;
            let start_y = grid_y * cell_height;
            let end_x = ((grid_x + 1) * cell_width).min(width);
            let end_y = ((grid_y + 1) * cell_height).min(height);

            for y in start_y..end_y {
                for x in start_x..end_x {
                    img.put_pixel(x, y, image::Rgba([r, g, b, 255]));
                }
            }
        }
    }

    // Use a more aggressive compression level for PNG
    let mut png_data = Vec::new();
    let mut cursor = Cursor::new(&mut png_data);

    // JPEG provides much smaller file sizes than PNG for this type of image
    img.write_to(&mut cursor, image::ImageOutputFormat::Jpeg(60)) // Quality set to 60%
        .map_err(|e| format!("Failed to encode JPEG: {}", e))?;

    println!("Generated mock NDVI image of size {} bytes", png_data.len());
    Ok(png_data)
}

/// STAC API Response that includes feature collection data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StacResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    pub stac_version: String,
    #[serde(default)]
    pub stac_extensions: Vec<String>,
    pub context: Option<Context>,
    #[serde(rename = "numberMatched")]
    pub number_matched: Option<i64>,
    #[serde(rename = "numberReturned")]
    pub number_returned: Option<i64>,
    pub features: Vec<Feature>,
    pub links: Vec<Link>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    pub limit: Option<i64>,
    pub matched: Option<i64>,
    pub returned: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
    #[serde(rename = "type")]
    pub feature_type: String,
    pub stac_version: String,
    pub id: String,
    pub properties: serde_json::Value,
    pub geometry: serde_json::Value,
    pub links: Vec<Link>,
    pub assets: serde_json::Value,
    pub bbox: Vec<f64>,
    #[serde(default)]
    pub stac_extensions: Vec<String>,
    pub collection: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub rel: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub link_type: Option<String>,
    pub href: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merge: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
}
