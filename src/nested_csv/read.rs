use {
    crate::Flattened,
    csv::StringRecord,
    serde::de::DeserializeOwned,
    serde_json::Value,
    std::{fmt::Debug, io::Read, marker::PhantomData},
    tap::Tap,
};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Headers-parsing reader is required")]
    NoHeaders,
    #[error("Reading headers")]
    ReadingHeaders(#[source] csv::Error),
    #[error("Reading a single record")]
    ReadingRecord(#[source] csv::Error),
    #[error("Deserializing a single value")]
    Deserializing(#[source] csv::Error),
    #[error("Deserializing a single flattened value")]
    DeserializingFlattened(#[source] serde_json::Error),
    #[error("Using serde_json parser to guess the type")]
    GuessingType(#[source] serde_json::Error),
    #[error("Missing field '{field}' (idx: {idx}) for record number {record}")]
    MissingField { idx: usize, field: String, record: usize },
}

type Result<T> = std::result::Result<T, self::Error>;

pub struct NestedCsvReader<R, T> {
    headers: StringRecord,
    reader: csv::Reader<R>,
    count: usize,
    _marker: PhantomData<T>,
    rec: StringRecord,
}

#[extension_traits::extension(pub trait CsvReaderEnableNestedExt)]
impl<R: Read> csv::Reader<R> {
    fn enable_nested<T: DeserializeOwned + Debug>(self) -> Result<NestedCsvReader<R, T>> {
        NestedCsvReader::new(self)
    }
}

impl<R: Read, T: DeserializeOwned + Debug> NestedCsvReader<R, T> {
    pub fn into_inner(self) -> R {
        self.reader.into_inner()
    }

    pub fn deserialize(&mut self) -> impl Iterator<Item = self::Result<T>> + '_ {
        std::iter::from_fn(|| {
            self.reader
                .read_record(&mut self.rec)
                .map_err(self::Error::ReadingRecord)
                .and_then(|r| {
                    r.then(|| {
                        self.headers
                            .iter()
                            .enumerate()
                            .map(|(idx, header)| {
                                self.rec
                                    .get(idx)
                                    .ok_or_else(|| self::Error::MissingField {
                                        idx,
                                        field: header.to_string(),
                                        record: self.count,
                                    })
                                    .and_then(|value| {
                                        serde_json::from_str::<serde_json::Value>(value)
                                            .map_err(self::Error::DeserializingFlattened)
                                            .map(|value| (header.to_string(), value))
                                    })
                            })
                            .collect::<Result<serde_json::Map<_, _>>>()
                            .map(Value::Object)
                            .and_then(|v| serde_json::from_value::<Flattened<T>>(v).map_err(self::Error::DeserializingFlattened))
                            .map(|Flattened(v)| v)
                    })
                    .transpose()
                })
                .transpose()
                .tap(|v| {
                    if matches!(v, Some(Ok(_))) {
                        self.count += 1
                    }
                })
        })
    }

    pub fn new(reader: csv::Reader<R>) -> Result<Self> {
        (match reader.has_headers() {
            true => Ok(reader),
            false => Err(self::Error::NoHeaders),
        })
        .and_then(|mut reader| {
            reader
                .headers()
                .map_err(self::Error::ReadingHeaders)
                .cloned()
                .map(|headers| (reader, headers))
        })
        .map(|(reader, headers)| Self {
            headers,
            reader,
            rec: Default::default(),
            _marker: PhantomData,
            count: 0,
        })
    }
}
