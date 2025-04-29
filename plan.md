This service is wraps the [Earth Search](https://element84.com/earth-search/) [STAC](https://stacspec.org/en) API to query the European Space Agency [Sentinel-2](https://en.wikipedia.org/wiki/Sentinel-2) [L2A](https://documentation.dataspace.copernicus.eu/APIs/SentinelHub/Data/S2L2A.html) satellite data collection.

It takes queries in the form of [STAC](https://stacspec.org/en), see the [Earth Search API docs](https://earth-search.aws.element84.com/v1/api.html) for more details.

```
curl -X POST "https://earth-search.aws.element84.com/v1/search" \
  -H "Content-Type: application/json" \
  -d '{
    "collections": ["sentinel-2-l2a"],
    "bbox": [-122.52, 37.70, -122.35, 37.83],
    "datetime": "2024-06-01T00:00:00Z/2024-06-30T23:59:59Z",
    "limit": 2,
    "query": {
      "eo:cloud_cover": {
        "lt": 10
      }
    }
  }' 
```




