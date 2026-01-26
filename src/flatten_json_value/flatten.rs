use {
    super::{FieldPath, JOIN_TAG, Segment, boxed_iter},
    itertools::Itertools,
    serde_json::Value,
    std::{borrow::Cow, iter::once},
    tap::Pipe,
};

pub fn flattened_iter<'prefix>(prefix: FieldPath<'prefix>, value: Value) -> impl Iterator<Item = (FieldPath<'static>, Value)> {
    match value {
        Value::Array(arr) => arr
            .into_iter()
            .enumerate()
            .flat_map({
                let prefix = prefix.clone();
                move |(idx, value)| flattened_iter(prefix.clone().join(Segment::Idx(idx)), value)
            })
            .pipe(boxed_iter),
        Value::Object(map) => map
            .into_iter()
            .flat_map({
                let prefix = prefix.clone();
                move |(key, value)| flattened_iter(prefix.clone().join(Segment::Field(Cow::Owned(key))), value)
            })
            .pipe(boxed_iter),
        other => once((prefix.to_owned(), other)).pipe(boxed_iter),
    }
    .pipe(boxed_iter)
}

pub fn flattened(value: serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
    flattened_iter(Default::default(), value)
        .map(|(k, v)| (k.0.iter().map(|k| k.to_string()).join(JOIN_TAG), v))
        .collect()
}

pub fn assert_flattened(value: serde_json::Value) -> Result<serde_json::Map<String, serde_json::Value>, serde_json::Value> {
    match value {
        Value::Object(map) => Ok(map),
        other => Err(other),
    }
}

#[cfg(test)]
mod tests {
    use {super::*, serde_json::json, tap::Tap};

    #[test]
    fn test_flatten_simple() {
        let input = json!({
            "name": "John",
            "age": 30
        });

        let result = flattened(input);
        assert_eq!(result.get("name").unwrap(), &json!("John"));
        assert_eq!(result.get("age").unwrap(), &json!(30));
    }

    #[test]
    fn test_flatten_nested() {
        let input = json!({
            "user": {
                "name": "John",
                "address": {
                    "city": "NYC",
                    "zip": "10001"
                }
            },
            "active": true
        });

        let result = flattened(input);
        assert_eq!(
            (&result)
                .tap(|r| println!("{r:#?}"))
                .get(&format!("user{JOIN_TAG}name"))
                .unwrap(),
            &json!("John")
        );
        assert_eq!(
            (&result)
                .tap(|r| println!("{r:#?}"))
                .get(&format!("user{JOIN_TAG}address{JOIN_TAG}city"))
                .unwrap(),
            &json!("NYC")
        );
        assert_eq!(
            (&result)
                .tap(|r| println!("{r:#?}"))
                .get(&format!("user{JOIN_TAG}address{JOIN_TAG}zip"))
                .unwrap(),
            &json!("10001")
        );
        assert_eq!(result.get("active").unwrap(), &json!(true));
    }
}
