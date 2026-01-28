use std::{error::Error as StdError, fmt, iter, num, str};

use serde::de::{
    self, Deserialize, DeserializeSeed, Deserializer, EnumAccess, Error as SerdeError,
    IntoDeserializer, MapAccess, SeqAccess, Unexpected, VariantAccess, Visitor,
};

use serde_json::{Map, Value};

#[derive(Clone, Debug)]
pub struct DeserializeError {
    field: Option<String>,
    kind: DeserializeErrorKind,
}

#[derive(Clone, Debug)]
pub enum DeserializeErrorKind {
    Message(String),
    Unsupported(String),
    UnexpectedEndOfMap,
    InvalidUtf8(str::Utf8Error),
    ParseBool(str::ParseBoolError),
    ParseInt(num::ParseIntError),
    ParseFloat(num::ParseFloatError),
    UnsortedKeys,
    InvalidArrayIndex,
}

impl SerdeError for DeserializeError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self {
            field: None,
            kind: DeserializeErrorKind::Message(msg.to_string()),
        }
    }
}

impl StdError for DeserializeError {}

impl fmt::Display for DeserializeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(field) = &self.field {
            write!(f, "field {}: {}", field, self.kind)
        } else {
            write!(f, "{}", self.kind)
        }
    }
}

impl fmt::Display for DeserializeErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Message(msg) => write!(f, "{}", msg),
            Self::Unsupported(which) => write!(f, "unsupported deserializer method: {}", which),
            Self::UnexpectedEndOfMap => write!(f, "expected field, but got end of map"),
            Self::InvalidUtf8(err) => err.fmt(f),
            Self::ParseBool(err) => err.fmt(f),
            Self::ParseInt(err) => err.fmt(f),
            Self::ParseFloat(err) => err.fmt(f),
            Self::UnsortedKeys => write!(f, "keys are not sorted"),
            Self::InvalidArrayIndex => write!(f, "invalid array index"),
        }
    }
}

#[derive(Clone)]
pub struct FlatMapDeserializer<'de> {
    map: &'de Map<String, Value>,
    prefix: String,
}

impl<'de> FlatMapDeserializer<'de> {
    pub fn new(map: &'de Map<String, Value>, prefix: String) -> Self {
        Self { map, prefix }
    }

    fn full_keys(&self) -> Vec<String> {
        self.map
            .keys()
            .filter(|k| {
                k.starts_with(&self.prefix)
                    && (self.prefix.is_empty() || k.as_bytes()[self.prefix.len()] == b'.')
            })
            .cloned()
            .collect()
    }

    fn check_sorted(&self) -> Result<(), DeserializeError> {
        let keys = self.full_keys();
        let mut sorted = keys.clone();
        sorted.sort();
        if keys != sorted {
            Err(DeserializeError {
                field: None,
                kind: DeserializeErrorKind::UnsortedKeys,
            })
        } else {
            Ok(())
        }
    }

    fn sub_keys(&self) -> Result<Vec<String>, DeserializeError> {
        let prefix_len = self.prefix.len() + if self.prefix.is_empty() { 0 } else { 1 };
        let mut unique = BTreeMap::new();
        for k in self.full_keys() {
            let seg = &k[prefix_len..];
            let first_seg = seg.split('.').next().unwrap_or("");
            unique.insert(first_seg.to_string(), ());
        }
        Ok(unique.into_keys().collect())
    }

    fn max_array_index(&self) -> Result<usize, DeserializeError> {
        let prefix_len = self.prefix.len() + if self.prefix.is_empty() { 0 } else { 1 };
        let mut max = 0;
        let mut indices = Vec::new();
        for k in self.full_keys() {
            let seg = &k[prefix_len..];
            let first_seg = seg.split('.').next().unwrap_or("");
            if let Ok(i) = first_seg.parse::<usize>() {
                indices.push(i);
                if i > max {
                    max = i;
                }
            } else {
                return Err(DeserializeError {
                    field: Some(k),
                    kind: DeserializeErrorKind::InvalidArrayIndex,
                });
            }
        }
        let mut sorted_indices = indices.clone();
        sorted_indices.sort();
        if indices != sorted_indices {
            return Err(DeserializeError {
                field: None,
                kind: DeserializeErrorKind::UnsortedKeys,
            });
        }
        Ok(max)
    }

    fn get_value(&self) -> Option<&'de Value> {
        self.map.get(&self.prefix)
    }

    fn infer_deserialize<V: Visitor<'de>>(&self, visitor: V) -> Result<V::Value, DeserializeError> {
        if let Some(value) = self.get_value() {
            match value {
                Value::String(s) => {
                    if let Ok(b) = s.parse::<bool>() {
                        return visitor.visit_bool(b);
                    }
                    if let Ok(n) = s.parse::<i8>() {
                        return visitor.visit_i8(n);
                    }
                    if let Ok(n) = s.parse::<i16>() {
                        return visitor.visit_i16(n);
                    }
                    if let Ok(n) = s.parse::<i32>() {
                        return visitor.visit_i32(n);
                    }
                    if let Ok(n) = s.parse::<i64>() {
                        return visitor.visit_i64(n);
                    }
                    if let Ok(n) = s.parse::<i128>() {
                        return visitor.visit_i128(n);
                    }
                    if let Ok(n) = s.parse::<u8>() {
                        return visitor.visit_u8(n);
                    }
                    if let Ok(n) = s.parse::<u16>() {
                        return visitor.visit_u16(n);
                    }
                    if let Ok(n) = s.parse::<u32>() {
                        return visitor.visit_u32(n);
                    }
                    if let Ok(n) = s.parse::<u64>() {
                        return visitor.visit_u64(n);
                    }
                    if let Ok(n) = s.parse::<u128>() {
                        return visitor.visit_u128(n);
                    }
                    if let Ok(n) = s.parse::<f32>() {
                        return visitor.visit_f32(n);
                    }
                    if let Ok(n) = s.parse::<f64>() {
                        return visitor.visit_f64(n);
                    }
                    if s.len() == 1 {
                        return visitor.visit_char(s.chars().next().unwrap());
                    }
                    return visitor.visit_borrowed_str(s);
                }
                Value::Bool(b) => visitor.visit_bool(*b),
                Value::Number(n) => {
                    if let Some(n) = n.as_i64() {
                        visitor.visit_i64(n)
                    } else if let Some(n) = n.as_u64() {
                        visitor.visit_u64(n)
                    } else if let Some(n) = n.as_f64() {
                        visitor.visit_f64(n)
                    } else {
                        Err(DeserializeError::custom("invalid number"))
                    }
                }
                Value::Null => visitor.visit_unit(),
                _ => Err(DeserializeError::custom("unsupported value type")),
            }
        } else {
            visitor.visit_unit()
        }
    }
}

impl<'de, 'a> Deserializer<'de> for &'a mut FlatMapDeserializer<'de> {
    type Error = DeserializeError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.infer_deserialize(visitor)
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.infer_deserialize(visitor)
    }

    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.infer_deserialize(visitor)
    }

    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.infer_deserialize(visitor)
    }

    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.infer_deserialize(visitor)
    }

    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.infer_deserialize(visitor)
    }

    fn deserialize_i128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.infer_deserialize(visitor)
    }

    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.infer_deserialize(visitor)
    }

    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.infer_deserialize(visitor)
    }

    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.infer_deserialize(visitor)
    }

    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.infer_deserialize(visitor)
    }

    fn deserialize_u128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.infer_deserialize(visitor)
    }

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.infer_deserialize(visitor)
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.infer_deserialize(visitor)
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.infer_deserialize(visitor)
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.infer_deserialize(visitor)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.infer_deserialize(visitor)
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.infer_deserialize(visitor)
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.infer_deserialize(visitor)
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        if self.get_value().is_some() {
            visitor.visit_some(self)
        } else {
            visitor.visit_none()
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.check_sorted()?;
        visitor.visit_seq(&mut FlatSeqAccess {
            de: self.clone(),
            current_index: 0,
            max_index: 0,
        })
    }

    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.check_sorted()?;
        visitor.visit_map(&mut FlatMapAccess {
            de: self.clone(),
            keys: iter::Peekable::new(self.sub_keys()?.into_iter()),
        })
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_enum(self.clone())
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_unit()
    }
}

struct FlatMapAccess<'de> {
    de: FlatMapDeserializer<'de>,
    keys: iter::Peekable<std::vec::IntoIter<String>>,
}

impl<'de> MapAccess<'de> for &mut FlatMapAccess<'de> {
    type Error = DeserializeError;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        if let Some(key) = self.keys.peek() {
            seed.deserialize(key.as_str().into_deserializer()).map(Some)
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(
        &mut self,
        seed: V,
    ) -> Result<V::Value, Self::Error> {
        let key = self.keys.next().expect("peeked");
        let sub_prefix = if self.de.prefix.is_empty() {
            key
        } else {
            format!("{}.{}", self.de.prefix, key)
        };
        seed.deserialize(&mut FlatMapDeserializer::new(self.de.map, sub_prefix))
    }
}

struct FlatSeqAccess<'de> {
    de: FlatMapDeserializer<'de>,
    current_index: usize,
    max_index: usize,
}

impl<'de> SeqAccess<'de> for &mut FlatSeqAccess<'de> {
    type Error = DeserializeError;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Self::Error> {
        if self.max_index == 0 {
            self.max_index = self.de.max_array_index()?;
        }
        if self.current_index > self.max_index {
            Ok(None)
        } else {
            let sub_prefix = if self.de.prefix.is_empty() {
                self.current_index.to_string()
            } else {
                format!("{}.{}", self.de.prefix, self.current_index)
            };
            let mut sub_de = FlatMapDeserializer::new(self.de.map, sub_prefix);
            let result =
                if self.de.get_value(&sub_prefix).is_some() || !sub_de.full_keys().is_empty() {
                    seed.deserialize(&mut sub_de)
                } else {
                    seed.deserialize(&mut sub_de)
                };
            self.current_index += 1;
            result.map(Some)
        }
    }
}

impl<'de> EnumAccess<'de> for FlatMapDeserializer<'de> {
    type Error = DeserializeError;
    type Variant = Self;

    fn variant_seed<V: DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self::Variant), Self::Error> {
        let variant =
            self.deserialize_any(de::value::StrDeserializer::<Self::Error>::new("variant"))?;
        seed.deserialize(variant.into_deserializer())
            .map(|v| (v, self))
    }
}

impl<'de> VariantAccess<'de> for FlatMapDeserializer<'de> {
    type Error = DeserializeError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(
        self,
        seed: T,
    ) -> Result<T::Value, Self::Error> {
        seed.deserialize(self)
    }

    fn tuple_variant<V: Visitor<'de>>(
        self,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_tuple(len, visitor)
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_struct("", fields, visitor)
    }
}
