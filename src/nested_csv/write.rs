use {
    crate::flatten_json_value::flatten::flattened,
    serde::Serialize,
    serde_json::Map,
    std::{fmt::Debug, io::Write, marker::PhantomData},
    tap::Pipe,
};

pub struct NestedCsvWriter<W: Write, T: Serialize + Debug> {
    writer: csv::Writer<W>,
    headers: Option<Vec<String>>,
    count: usize,
    _marker: PhantomData<T>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Could not convert into inner error:\n{0}")]
    IntoInner(Box<str>),
    #[error("Could not serialize the struct to value")]
    SerializingToValue(#[source] serde_json::Error),
    #[error("Could not write headers")]
    WritingHeaders(#[source] csv::Error),
    #[error("Writing record #{idx}")]
    WritingRecord {
        idx: usize,
        #[source]
        source: csv::Error,
    },
    #[error("Extra headers compared to headers line:\n{extra_values:#?}")]
    ExtraValuesComparedToHeaders {
        extra_values: Map<String, serde_json::Value>,
    },
}

type Result<T> = std::result::Result<T, self::Error>;

#[extension_traits::extension(pub trait CsvWriterEnableNestedExt)]
impl<W: Write> csv::Writer<W> {
    fn enable_nested<T: Serialize + Debug>(self) -> NestedCsvWriter<W, T> {
        NestedCsvWriter::new(self)
    }
}

impl<W, T> NestedCsvWriter<W, T>
where
    W: Write,
    T: Serialize + Debug,
{
    pub fn into_inner(self) -> Result<W> {
        self.writer
            .into_inner()
            .map_err(|e| self::Error::IntoInner(format!("{e:#?}").pipe(Box::from)))
    }

    pub fn new(writer: csv::Writer<W>) -> Self {
        Self {
            writer,
            count: 0usize,
            headers: None,
            _marker: PhantomData,
        }
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }

    pub fn serialize(&mut self, item: &T) -> Result<()> {
        serde_json::to_value(item)
            .map_err(self::Error::SerializingToValue)
            .map(flattened)
            .and_then(|mut item| -> Result<_> {
                if self.headers.is_none() {
                    let headers = item.keys().cloned().collect::<Vec<_>>();
                    self.writer
                        .write_record(&headers)
                        .map_err(self::Error::WritingHeaders)?;
                    self.headers = Some(headers);
                }
                self.count += 1;
                self.headers
                    .as_ref()
                    .expect("headers to be set above")
                    .iter()
                    .map(|h| item.remove(h.as_str()).unwrap_or(serde_json::Value::Null))
                    .map(|f| match &f {
                        serde_json::Value::Null => "".to_string(),
                        serde_json::Value::Bool(bool) => bool.to_string(),
                        serde_json::Value::Number(number) => number.to_string(),
                        serde_json::Value::String(v) => v.to_string(),
                        other => panic!("bad flattening: {other:#?}"),
                    })
                    .collect::<Vec<_>>()
                    .pipe(|values| {
                        item.is_empty().then_some(values).ok_or_else(|| {
                            self::Error::ExtraValuesComparedToHeaders { extra_values: item }
                        })
                    })
                    .and_then(|row| {
                        self.writer.write_record(&row).map_err(|source| {
                            self::Error::WritingRecord {
                                idx: self.count,
                                source,
                            }
                        })
                    })
            })
    }
}

/// allows bypassing the limitation of csv crate (it disallows writing nested objects)
pub fn write_nested_csv<'a, W, T>(
    writer: &mut W,
    items: impl IntoIterator<Item = &'a T>,
) -> Result<usize>
where
    W: Write,
    T: Serialize + Debug + 'a,
{
    NestedCsvWriter::<_, T>::new(csv::WriterBuilder::new().from_writer(writer)).pipe_ref_mut(|w| {
        items
            .into_iter()
            .try_for_each(|i| w.serialize(i))
            .map(|_| w.count)
    })
}
