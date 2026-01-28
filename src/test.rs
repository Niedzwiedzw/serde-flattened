use {
    crate::nested_csv::{read::CsvReaderEnableNestedExt, write::CsvWriterEnableNestedExt},
    anyhow::{Context, Result},
    serde::{Deserialize, Serialize, de::DeserializeOwned},
    std::io::{Read, Seek},
    tap::Pipe,
    tracing::info,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Child {
    field_1: bool,
    field_2: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Parent {
    child_1: Child,
    child_2: Child,
}

fn back_and_forth<'a, T>(mut data: impl Iterator<Item = &'a T> + 'a) -> Result<()>
where
    T: Serialize + DeserializeOwned + Send + std::fmt::Debug + 'a,
{
    csv::WriterBuilder::new()
        .from_writer(std::io::Cursor::new(Vec::new()))
        .pipe(|mut w| {
            data.try_for_each(|p| {
                info!(?p);
                w.serialize(p)
            })
            .context("serializing")
            .and_then(|()| w.into_inner().context("dropping writer"))
        })
        .and_then(|mut buffer| {
            buffer
                .rewind()
                .context("rewinding")
                .map(|()| buffer)
                .and_then(|mut buffer| {
                    csv::ReaderBuilder::new()
                        .has_headers(true)
                        .from_reader(&mut buffer)
                        .pipe(Ok)
                        .and_then(|mut reader| {
                            reader
                                .deserialize::<T>()
                                .map(|r| r.context("deserializing"))
                                .collect::<Result<Vec<_>, _>>()
                                .map(|values| info!("OK!\n{values:#?}"))
                                .with_context(|| {
                                    format!(
                                        "deserializing contents of buffer:\n{}",
                                        reader
                                            .into_inner()
                                            .pipe(|buffer| {
                                                buffer.rewind().context("rewinding").and_then(
                                                    |_| {
                                                        String::new().pipe(|mut v| {
                                                            buffer
                                                                .read_to_string(&mut v)
                                                                .context("reading to string")
                                                                .map(|_| v)
                                                        })
                                                    },
                                                )
                                            })
                                            .unwrap_or_else(|e| format!("{e:?}"))
                                    )
                                })
                        })
                })
        })
}

fn back_and_forth_nesting_enabled<'a, T>(mut data: impl Iterator<Item = &'a T> + 'a) -> Result<()>
where
    T: Serialize + DeserializeOwned + Send + std::fmt::Debug + 'a,
{
    csv::WriterBuilder::new()
        .from_writer(std::io::Cursor::new(Vec::new()))
        .enable_nested()
        .pipe(|mut w| {
            data.try_for_each(|p| {
                info!(?p);
                w.serialize(p)
            })
            .context("serializing")
            .and_then(|()| w.into_inner().context("dropping writer"))
        })
        .and_then(|mut buffer| {
            buffer
                .rewind()
                .context("rewinding")
                .map(|()| buffer)
                .and_then(|mut buffer| {
                    csv::ReaderBuilder::new()
                        .has_headers(true)
                        .from_reader(&mut buffer)
                        .enable_nested::<T>()
                        .context("enabling nesting")
                        .and_then(|mut reader| {
                            reader
                                .deserialize()
                                .map(|r| r.context("deserializing"))
                                .collect::<Result<Vec<_>, _>>()
                                .map(|values| info!("OK!\n{values:#?}"))
                                .with_context(|| {
                                    format!(
                                        "deserializing contents of buffer:\n{}",
                                        reader
                                            .into_inner()
                                            .pipe(|buffer| {
                                                buffer.rewind().context("rewinding").and_then(
                                                    |_| {
                                                        String::new().pipe(|mut v| {
                                                            buffer
                                                                .read_to_string(&mut v)
                                                                .context("reading to string")
                                                                .map(|_| v)
                                                        })
                                                    },
                                                )
                                            })
                                            .unwrap_or_else(|e| format!("{e:?}"))
                                    )
                                })
                        })
                })
        })
}

const PARENT: Parent = Parent {
    child_1: Child {
        field_1: true,
        field_2: 0,
    },
    child_2: Child {
        field_1: false,
        field_2: 1,
    },
};

const DATA: &[Parent] = &[PARENT, PARENT, PARENT];

#[test]
#[should_panic(
    expected = "cannot serialize Child container inside struct when writing headers from structs"
)]
fn test_normal_data_fails() {
    back_and_forth(DATA.iter()).unwrap()
}

#[test_log::test]
fn test_flattening_fixes_the_problem() {
    back_and_forth_nesting_enabled(DATA.iter()).expect("going back and forth with nesting enabled")
}

/// Regression test for the issue where String fields containing numeric values
/// would fail to deserialize because the intermediate JSON representation
/// would parse "123" as a number instead of a string.
#[test_log::test]
fn test_string_field_with_numeric_value() {
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    struct Inner {
        // This is a String, but the value looks like a number
        id: String,
        name: String,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    struct Outer {
        inner: Inner,
        count: i32,
    }

    let data = [
        Outer {
            inner: Inner {
                id: "12345".to_string(), // Looks like a number!
                name: "test".to_string(),
            },
            count: 42,
        },
        Outer {
            inner: Inner {
                id: "00123".to_string(), // Leading zeros - definitely should be string
                name: "another".to_string(),
            },
            count: 7,
        },
    ];

    back_and_forth_nesting_enabled(data.iter())
        .expect("String fields with numeric values should round-trip correctly");
}
