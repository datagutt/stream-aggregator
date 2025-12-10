# Stream Aggregator API

This crate provides the REST API layer for StreamAggregator.

## Query String Parsing

### Generic QsQuery Extractor

The API uses a **generic query string extractor** (`QsQuery<T>`) that supports advanced query string parsing including:

- **Bracket notation for nested structures**: `?labels[key]=value`
- **Arrays**: `?tags[]=value1&tags[]=value2`
- **Complex nested objects**
- **All standard query parameters**

This is implemented using the `serde_qs` library, which provides full query string deserialization support.

### Usage

The extractor works with any `Deserialize` type:

```rust
use axum::extract::State;
use crate::handlers::QsQuery;

#[derive(Debug, Deserialize)]
pub struct MyQuery {
    pub simple: Option<String>,
    #[serde(default)]
    pub nested: HashMap<String, String>,
}

pub async fn my_handler(
    State(state): State<AppState>,
    QsQuery(query): QsQuery<MyQuery>,
) -> Result<Json<Response>, Error> {
    // query.nested will contain properly parsed HashMap
    // from ?nested[key1]=value1&nested[key2]=value2
}
```

### Examples

**Simple parameters:**
```
GET /api/v1/streams?platform=twitch&live=true
```

**Nested objects (labels):**
```
GET /api/v1/streams?labels[country]=no&labels[team]=vikings
```

**Mixed parameters:**
```
GET /api/v1/streams?platform=twitch&labels[tier]=pro&min_viewers=1000&sort=viewers&order=desc
```

**Complex nested structures:**
```
GET /api/v1/endpoint?filter[status]=active&filter[tags][0]=gaming&filter[tags][1]=esports
```

### Implementation Details

The `QsQuery<T>` extractor:

1. Implements `FromRequestParts` trait from axum
2. Extracts the raw query string from the URI
3. Uses `serde_qs::from_str()` to deserialize into the target type
4. Returns a `BAD_REQUEST` error if parsing fails
5. Works with any type that implements `DeserializeOwned`

This approach provides:
- ✅ **Type safety**: Full compile-time validation
- ✅ **Flexibility**: Works with any Serde-compatible type
- ✅ **Consistency**: Same parsing logic across all endpoints
- ✅ **Error handling**: Clear error messages for invalid queries
- ✅ **No boilerplate**: Just add `#[serde(default)]` for optional nested fields

### Migration from Standard Query Extractor

To migrate an endpoint from axum's standard `Query<T>` to `QsQuery<T>`:

```diff
- use axum::extract::Query;
+ use crate::handlers::QsQuery;

  pub async fn my_handler(
      State(state): State<AppState>,
-     Query(params): Query<MyParams>,
+     QsQuery(params): QsQuery<MyParams>,
  ) -> Result<Json<Response>, Error> {
      // No other changes needed!
  }
```

### Supported Formats

The extractor supports all `serde_qs` formats:

| Format | Example | Parsed As |
|--------|---------|-----------|
| Simple | `?key=value` | `{ "key": "value" }` |
| Array (brackets) | `?items[]=a&items[]=b` | `{ "items": ["a", "b"] }` |
| Array (indexed) | `?items[0]=a&items[1]=b` | `{ "items": ["a", "b"] }` |
| Nested object | `?user[name]=john&user[age]=30` | `{ "user": { "name": "john", "age": 30 } }` |
| HashMap | `?labels[k1]=v1&labels[k2]=v2` | `{ "labels": { "k1": "v1", "k2": "v2" } }` |
| Boolean | `?active=true` | `{ "active": true }` |
| Numbers | `?count=42` | `{ "count": 42 }` |

## API Endpoints

See [API.md](../../docs/API.md) for full API documentation.
