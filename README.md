# serde-flattened

[![Crates.io](https://img.shields.io/crates/v/serde-flattened)](https://crates.io/crates/serde-flattened)

A `csv` and `serde_json` extension for flattening nested structures into flat representations. This enables for example serialization/deserialization of nested data to/from CSV.

## Features
- **Nested CSV Support**: Serialize/deserialize nested Rust structs to/from flat CSV files where nested fields are encoded using `__`-separated paths.
- **Flatten/Unflatten JSON Values**: Convert nested `serde_json::Value` to/from flat maps with `__`-separated paths.

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
serde-flattened = { version = "0.1.0" }
csv = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### Flattening JSON Values

```rust
use serde_flattened::flatten_json_value::{flattened, unflattened};
use serde_json::{json, Value};

let nested = json!({
    "user": {
        "name": "John",
        "address": {
            "city": "NYC",
            "zip": "10001"
        }
    }
});

let flat_map = flattened(nested);
assert_eq!(flat_map.get("user__name").unwrap(), "John");
assert_eq!(flat_map.get("user__address__city").unwrap(), "NYC");

let restored = unflattened(flat_map).unwrap();
assert_eq!(restored, nested);
```

### Nested CSV Serialization/Deserialization

Define your nested struct:

```rust
#[derive(serde::Serialize, serde::Deserialize, Debug, Default)]
struct Child {
    field_1: bool,
    field_2: i32,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Default)]
struct Parent {
    child_1: Child,
    child_2: Child,
}
```

**Writing to CSV:**

```rust
use serde_flattened::nested_csv::CsvWriterEnableNestedExt;
use std::io::Cursor;


let mut wtr = csv::WriterBuilder::new()
    .from_writer(Cursor::new(Vec::new()))
    .enable_nested();

for parent in [Parent::default()] {
    wtr.serialize(&parent)?;
}
let csv_bytes = wtr.into_inner()?.into_inner();
```

**Reading from CSV:**

```rust
use serde_flattened::{Flattened, nested_csv::CsvReaderEnableNestedExt};
use std::io::Cursor;

let mut rdr = csv::ReaderBuilder::new()
    .has_headers(true)
    .from_reader(csv_bytes)
    .enable_nested::<Parent>()?;
for result in rdr.deserialize() {
    let parent: Parent = result?;
    println!(""{:?}"", parent);
}
```

## How it Works

- **Serialization**: Nested structs are flattened into dot-separated field paths (e.g., `user.address.city`), with primitive/nested values JSON-serialized into CSV cells.
- **Deserialization**: CSV cells are parsed as JSON `Value`s, assembled into a flat `Object`, unflattened using path keys, then deserialized into the target struct.


## License

MIT
