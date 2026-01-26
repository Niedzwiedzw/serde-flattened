use {super::boxed_iter, serde_json::Value, std::iter::once, tap::Pipe, tracing::instrument};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(
        "unsupported top level value, it expected a Map<String, String | bool | number | null>, found {0}"
    )]
    UnsupportedTopLevelValue(&'static str),
    #[error(
        "unsupported child level value for key: '{key}', it expected a String | bool | number | null"
    )]
    UnsupportedChildValue { key: String },
    #[error("Other error: {0}")]
    Other(&'static str),
}

type Result<T> = std::result::Result<T, self::Error>;

use crate::flatten_json_value::{FieldPath, JOIN_TAG, Segment};

trait TryFlatMapExt<'a, T, E> {
    fn try_flat_map<U, F, OutIter>(
        self,
        try_flat_map: F,
    ) -> impl Iterator<Item = std::result::Result<U, E>> + 'a
    where
        U: 'a,
        F: FnMut(T) -> OutIter + 'a,
        OutIter: Iterator<Item = std::result::Result<U, E>> + 'a;
}

impl<'a, T, E, I> TryFlatMapExt<'a, T, E> for I
where
    I: Iterator<Item = std::result::Result<T, E>> + 'a,
    T: 'a,
    E: 'a,
    I: 'a,
{
    fn try_flat_map<U, F, OutIter>(
        self,
        mut try_flat_map: F,
    ) -> impl Iterator<Item = std::result::Result<U, E>> + 'a
    where
        U: 'a,
        F: FnMut(T) -> OutIter + 'a,
        OutIter: Iterator<Item = std::result::Result<U, E>> + 'a,
    {
        self.flat_map(move |e| match e {
            Ok(i) => try_flat_map(i).pipe(boxed_iter),
            Err(e) => once(Err(e)).pipe(boxed_iter),
        })
    }
}

#[instrument]
pub fn unflatten_iter(value: Value) -> impl Iterator<Item = Result<(FieldPath<'static>, Value)>> {
    match value {
        Value::Object(map) => Ok(map),
        other => {
            tracing::debug!("other=\n{other:#?}");
            Err(self::Error::UnsupportedTopLevelValue(match other {
                Value::Null => "Value::Null ",
                Value::Bool(_) => "Value::Bool",
                Value::Number(_) => "Value::Number",
                Value::String(_) => "Value::String",
                Value::Array(_) => "Value::Array",
                Value::Object(_) => "Value::Object",
            }))
        }
    }
    .pipe(once)
    .try_flat_map(|values| {
        values.into_iter().map(|(key, value)| match value {
            value @ (Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)) => key
                .split(JOIN_TAG)
                .map(Segment::from_str)
                .collect::<Vec<_>>()
                .pipe(FieldPath)
                .pipe(|key| (key.to_owned(), value))
                .pipe(Ok),
            _other => Err(self::Error::UnsupportedChildValue { key: key.clone() }),
        })
    })
}

#[extension_traits::extension(pub trait VecTryInsertExt)]
impl<T> Vec<T> {
    fn get_mut_or_insert_with(
        &mut self,
        index: usize,
        or_insert_with: impl FnOnce() -> T,
    ) -> std::result::Result<&mut T, usize> {
        match self.len() {
            len if index == len => {
                self.insert(index, or_insert_with());
                unsafe { self.get_unchecked_mut(index) }.pipe(Ok)
            }
            len if index > len => Err(index),
            len if index < len => {
                // SAFETY: all other scenarios have been checked
                unsafe { self.get_unchecked_mut(index) }.pipe(Ok)
            }
            _ => unreachable!("all patterns were checked"),
        }
    }
    fn try_insert(&mut self, index: usize, value: T) -> std::result::Result<(), (usize, T)> {
        match self.len() {
            len if index >= len => Err((index, value)),
            _ => {
                self.insert(index, value);
                Ok(())
            }
        }
    }
}

struct ValueBuilder<'a>(&'a mut serde_json::Value);

struct ObjectBuilder<'a>(&'a mut serde_json::Value);

impl ObjectBuilder<'_> {
    fn obj(&mut self) -> &mut serde_json::Map<String, Value> {
        match &mut self.0 {
            Value::Object(map) => map,
            other => panic!("checked on creation: {other:?}"),
        }
    }

    fn get_or_create(&mut self, key: &str) -> &mut serde_json::Value {
        if self.obj().get(key).is_none() {
            self.obj().insert(key.to_string(), serde_json::Value::Null);
        }
        self.obj().get_mut(key).expect("created above")
    }
}

struct ArrayBuilder<'a>(&'a mut serde_json::Value);

impl ArrayBuilder<'_> {
    fn arr(&mut self) -> &mut Vec<serde_json::Value> {
        match &mut self.0 {
            Value::Array(values) => values,
            other => panic!("validated on creation: {other:?}"),
        }
    }
    pub fn get_or_create(&mut self, index: usize) -> &mut serde_json::Value {
        if self.arr().get(index).is_none() {
            self.arr().insert(index, serde_json::Value::Null);
        }

        self.arr().get_mut(index).expect("created above")
    }
}

impl ValueBuilder<'_> {
    fn make_array(&mut self) -> std::result::Result<ArrayBuilder<'_>, &'static str> {
        match &self.0 {
            Value::Array(_) => {}
            Value::Null => *self.0 = Value::Array(Default::default()),
            Value::Bool(_) => return Err("found bool, expected array or null"),
            Value::Number(_) => return Err("found number, expected array or null"),
            Value::String(_) => return Err("found string, expected array or null"),
            Value::Object(_) => return Err("found object, expected array or null"),
        };
        Ok(ArrayBuilder(&mut *self.0))
    }
    fn make_object(&mut self) -> std::result::Result<ObjectBuilder<'_>, &'static str> {
        match &self.0 {
            Value::Object(_) => {}
            Value::Null => *self.0 = Value::Object(Default::default()),
            Value::Bool(_) => return Err("found bool, expected object or null"),
            Value::Number(_) => return Err("found number, expected object or null"),
            Value::String(_) => return Err("found string, expected object or null"),
            Value::Array(_) => return Err("found array, expected object or null"),
        };
        Ok(ObjectBuilder(&mut *self.0))
    }

    fn apply(
        &mut self,
        path: FieldPath<'_>,
        value: serde_json::Value,
    ) -> std::result::Result<(), &'static str> {
        match path.pop_start() {
            Some((current, rest)) => match current {
                Segment::Idx(idx) => self
                    .make_array()
                    .and_then(|mut arr| ValueBuilder(arr.get_or_create(idx)).apply(rest, value)),
                Segment::Field(key) => self.make_object().and_then(|mut arr| {
                    ValueBuilder(arr.get_or_create(key.as_ref())).apply(rest, value)
                }),
            },
            None => {
                *self.0 = value;
                Ok(())
            }
        }
    }
}

#[instrument]
pub fn unflattened(value: serde_json::Value) -> Result<serde_json::Value> {
    let mut out = serde_json::Value::Null;
    unflatten_iter(value)
        .try_fold(ValueBuilder(&mut out), |mut out, next| {
            next.and_then(|(key, value)| {
                out.apply(key, value)
                    .map(|_| out)
                    .map_err(self::Error::Other)
            })
        })
        .map(drop)
        .map(|_| out)
}

#[cfg(test)]
pub mod test {
    use {anyhow::Context, serde_json::json, tap::Pipe};

    #[test]
    fn test_example_1() -> anyhow::Result<()> {
        json!({
            "user": {
                "name": "John",
                "address": {
                    "city": "NYC",
                    "zip": "10001"
                }
            },
            "active": true
        })
        .pipe(|expected| {
            crate::flatten_json_value::flatten::flattened(expected.clone())
                .pipe(serde_json::Value::from)
                .pipe(super::unflattened)
                .context("unflattening")
                .and_then(|got| {
                    anyhow::ensure!(expected == got, "expected:\n{expected}\n\ngot:\n{got}");
                    Ok(())
                })
        })
    }
}
