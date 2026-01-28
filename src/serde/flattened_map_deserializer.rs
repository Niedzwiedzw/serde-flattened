//! Custom deserializer for flattened string maps.
//!
//! This deserializer takes a `Map<String, String>` where keys are flattened paths
//! (e.g., "user__address__city") and values are raw strings. It handles nested
//! path lookups and lets the target type decide how to parse string values.

use {
    indexmap::IndexMap,
    serde::{
        Deserializer,
        de::{self, DeserializeSeed, IntoDeserializer, MapAccess, SeqAccess, Visitor},
    },
    std::borrow::Cow,
};

const JOIN_TAG: &str = "__";
const ARR_PFX: &str = "idx-";

/// Error type for deserialization
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Custom(String),
    #[error("missing field: {0}")]
    MissingField(String),
    #[error("invalid type: expected {expected}, got '{got}'")]
    InvalidType { expected: &'static str, got: String },
}

impl de::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Error::Custom(msg.to_string())
    }
}

type Result<T> = std::result::Result<T, Error>;

/// Deserializer for a flattened map of string keys to string values.
///
/// This is the main entry point - it deserializes nested structures from
/// a flat map by looking up keys with the appropriate prefix.
pub struct FlattenedMapDeserializer<'de> {
    /// The flattened map data
    data: &'de IndexMap<String, String>,
    /// Current path prefix (for nested access)
    prefix: Cow<'de, str>,
}

impl<'de> FlattenedMapDeserializer<'de> {
    pub fn new(data: &'de IndexMap<String, String>) -> Self {
        Self {
            data,
            prefix: Cow::Borrowed(""),
        }
    }

    /// Get the direct child field names at the current prefix level
    fn child_fields(&self) -> Vec<&'de str> {
        let mut fields: Vec<&str> = Vec::new();
        let prefix_len = if self.prefix.is_empty() {
            0
        } else {
            self.prefix.len() + JOIN_TAG.len()
        };

        for key in self.data.keys() {
            let relevant = if self.prefix.is_empty() {
                Some(key.as_str())
            } else if key.starts_with(self.prefix.as_ref())
                && key[self.prefix.len()..].starts_with(JOIN_TAG)
            {
                Some(&key[prefix_len..])
            } else {
                None
            };

            if let Some(rest) = relevant {
                // Get the first segment of the remaining path
                let field = rest.split(JOIN_TAG).next().unwrap_or(rest);
                if !field.is_empty() && !fields.contains(&field) {
                    fields.push(field);
                }
            }
        }
        fields
    }

    /// Check if this is a leaf value (exact key match)
    fn get_leaf_value(&self) -> Option<&'de str> {
        if self.prefix.is_empty() {
            None
        } else {
            self.data.get(self.prefix.as_ref()).map(|s| s.as_str())
        }
    }

    /// Check if this prefix represents an array (has idx-N children)
    fn is_array(&self) -> bool {
        self.child_fields().iter().any(|f| f.starts_with(ARR_PFX))
    }

    /// Get array indices at current prefix
    fn array_indices(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = self
            .child_fields()
            .iter()
            .filter_map(|f| f.strip_prefix(ARR_PFX)?.parse().ok())
            .collect();
        indices.sort();
        indices
    }

    /// Check if there are any non-empty values under the current prefix.
    /// Used to determine if an Option<Struct> should be Some or None.
    fn has_non_empty_descendants(&self) -> bool {
        for (key, value) in self.data.iter() {
            let matches = if self.prefix.is_empty() {
                true
            } else {
                key == self.prefix.as_ref()
                    || (key.starts_with(self.prefix.as_ref())
                        && key[self.prefix.len()..].starts_with(JOIN_TAG))
            };

            if matches && !value.is_empty() {
                return true;
            }
        }
        false
    }
}

impl<'de> de::Deserializer<'de> for FlattenedMapDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // Check if this is a leaf value first
        if let Some(value) = self.get_leaf_value() {
            return StrDeserializer::new(value).deserialize_any(visitor);
        }

        // Check if it's an array
        if self.is_array() {
            return self.deserialize_seq(visitor);
        }

        // Otherwise treat as a map/struct
        self.deserialize_map(visitor)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if let Some(value) = self.get_leaf_value() {
            StrDeserializer::new(value).deserialize_bool(visitor)
        } else {
            Err(Error::MissingField(self.prefix.into_owned()))
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if let Some(value) = self.get_leaf_value() {
            StrDeserializer::new(value).deserialize_i8(visitor)
        } else {
            Err(Error::MissingField(self.prefix.into_owned()))
        }
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if let Some(value) = self.get_leaf_value() {
            StrDeserializer::new(value).deserialize_i16(visitor)
        } else {
            Err(Error::MissingField(self.prefix.into_owned()))
        }
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if let Some(value) = self.get_leaf_value() {
            StrDeserializer::new(value).deserialize_i32(visitor)
        } else {
            Err(Error::MissingField(self.prefix.into_owned()))
        }
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if let Some(value) = self.get_leaf_value() {
            StrDeserializer::new(value).deserialize_i64(visitor)
        } else {
            Err(Error::MissingField(self.prefix.into_owned()))
        }
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if let Some(value) = self.get_leaf_value() {
            StrDeserializer::new(value).deserialize_u8(visitor)
        } else {
            Err(Error::MissingField(self.prefix.into_owned()))
        }
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if let Some(value) = self.get_leaf_value() {
            StrDeserializer::new(value).deserialize_u16(visitor)
        } else {
            Err(Error::MissingField(self.prefix.into_owned()))
        }
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if let Some(value) = self.get_leaf_value() {
            StrDeserializer::new(value).deserialize_u32(visitor)
        } else {
            Err(Error::MissingField(self.prefix.into_owned()))
        }
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if let Some(value) = self.get_leaf_value() {
            StrDeserializer::new(value).deserialize_u64(visitor)
        } else {
            Err(Error::MissingField(self.prefix.into_owned()))
        }
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if let Some(value) = self.get_leaf_value() {
            StrDeserializer::new(value).deserialize_f32(visitor)
        } else {
            Err(Error::MissingField(self.prefix.into_owned()))
        }
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if let Some(value) = self.get_leaf_value() {
            StrDeserializer::new(value).deserialize_f64(visitor)
        } else {
            Err(Error::MissingField(self.prefix.into_owned()))
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if let Some(value) = self.get_leaf_value() {
            StrDeserializer::new(value).deserialize_char(visitor)
        } else {
            Err(Error::MissingField(self.prefix.into_owned()))
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if let Some(value) = self.get_leaf_value() {
            visitor.visit_borrowed_str(value)
        } else {
            Err(Error::MissingField(self.prefix.into_owned()))
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if let Some(value) = self.get_leaf_value() {
            visitor.visit_borrowed_str(value)
        } else {
            Err(Error::MissingField(self.prefix.into_owned()))
        }
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if let Some(value) = self.get_leaf_value() {
            visitor.visit_borrowed_bytes(value.as_bytes())
        } else {
            Err(Error::MissingField(self.prefix.into_owned()))
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // In CSV, empty strings represent null/None values.
        // For Option<T>, we need to check if there's any actual data:
        // - For leaf values: non-empty string means Some
        // - For nested structs: at least one non-empty descendant means Some
        if self.has_non_empty_descendants() {
            visitor.visit_some(self)
        } else {
            visitor.visit_none()
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let indices = self.array_indices();
        visitor.visit_seq(SeqAccessor {
            data: self.data,
            prefix: self.prefix,
            indices: indices.into_iter(),
        })
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let fields = self.child_fields();
        visitor.visit_map(MapAccessor {
            data: self.data,
            prefix: self.prefix,
            fields: fields.into_iter(),
            current_field: None,
        })
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // For simple enums, treat the leaf value as a unit variant name
        if let Some(value) = self.get_leaf_value() {
            visitor.visit_enum(value.into_deserializer())
        } else {
            // For complex enums with data, treat child fields as variant name -> data
            let fields = self.child_fields();
            if fields.len() == 1 {
                visitor.visit_enum(EnumAccessor {
                    data: self.data,
                    prefix: self.prefix,
                    variant: fields[0],
                })
            } else {
                Err(Error::Custom(format!(
                    "expected enum at '{}', found {} fields",
                    self.prefix,
                    fields.len()
                )))
            }
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }
}

/// MapAccess implementation for iterating over struct fields
struct MapAccessor<'de, I> {
    data: &'de IndexMap<String, String>,
    prefix: Cow<'de, str>,
    fields: I,
    current_field: Option<&'de str>,
}

impl<'de, I: Iterator<Item = &'de str>> MapAccess<'de> for MapAccessor<'de, I> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        match self.fields.next() {
            Some(field) => {
                self.current_field = Some(field);
                seed.deserialize(field.into_deserializer()).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        let field = self
            .current_field
            .take()
            .ok_or_else(|| Error::Custom("next_value_seed called before next_key_seed".into()))?;

        let new_prefix = if self.prefix.is_empty() {
            Cow::Owned(field.to_string())
        } else {
            Cow::Owned(format!("{}{JOIN_TAG}{field}", self.prefix))
        };

        seed.deserialize(FlattenedMapDeserializer {
            data: self.data,
            prefix: new_prefix,
        })
    }
}

/// SeqAccess implementation for iterating over array elements
struct SeqAccessor<'de, I> {
    data: &'de IndexMap<String, String>,
    prefix: Cow<'de, str>,
    indices: I,
}

impl<'de, I: Iterator<Item = usize>> SeqAccess<'de> for SeqAccessor<'de, I> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        match self.indices.next() {
            Some(idx) => {
                let field = format!("{ARR_PFX}{idx}");
                let new_prefix = if self.prefix.is_empty() {
                    Cow::Owned(field)
                } else {
                    Cow::Owned(format!("{}{JOIN_TAG}{field}", self.prefix))
                };

                seed.deserialize(FlattenedMapDeserializer {
                    data: self.data,
                    prefix: new_prefix,
                })
                .map(Some)
            }
            None => Ok(None),
        }
    }
}

/// EnumAccess implementation for deserializing enums
struct EnumAccessor<'de> {
    data: &'de IndexMap<String, String>,
    prefix: Cow<'de, str>,
    variant: &'de str,
}

impl<'de> de::EnumAccess<'de> for EnumAccessor<'de> {
    type Error = Error;
    type Variant = VariantAccessor<'de>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        let variant_de = self.variant.into_deserializer();
        let variant = seed.deserialize(variant_de)?;

        let new_prefix = if self.prefix.is_empty() {
            Cow::Owned(self.variant.to_string())
        } else {
            Cow::Owned(format!("{}{JOIN_TAG}{}", self.prefix, self.variant))
        };

        Ok((
            variant,
            VariantAccessor {
                de: FlattenedMapDeserializer {
                    data: self.data,
                    prefix: new_prefix,
                },
            },
        ))
    }
}

struct VariantAccessor<'de> {
    de: FlattenedMapDeserializer<'de>,
}

impl<'de> de::VariantAccess<'de> for VariantAccessor<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(self.de)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de.deserialize_tuple(len, visitor)
    }

    fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de.deserialize_struct("", fields, visitor)
    }
}

/// Deserializer for leaf string values.
///
/// This handles converting raw strings to the requested type.
struct StrDeserializer<'de> {
    value: &'de str,
}

impl<'de> StrDeserializer<'de> {
    fn new(value: &'de str) -> Self {
        Self { value }
    }
}

impl<'de> de::Deserializer<'de> for StrDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // When type is unknown, return as string and let visitor decide
        visitor.visit_borrowed_str(self.value)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            "true" => visitor.visit_bool(true),
            "false" => visitor.visit_bool(false),
            _ => Err(Error::InvalidType {
                expected: "bool",
                got: self.value.to_string(),
            }),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let n: i8 = self.value.parse().map_err(|_| Error::InvalidType {
            expected: "i8",
            got: self.value.to_string(),
        })?;
        visitor.visit_i8(n)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let n: i16 = self.value.parse().map_err(|_| Error::InvalidType {
            expected: "i16",
            got: self.value.to_string(),
        })?;
        visitor.visit_i16(n)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let n: i32 = self.value.parse().map_err(|_| Error::InvalidType {
            expected: "i32",
            got: self.value.to_string(),
        })?;
        visitor.visit_i32(n)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let n: i64 = self.value.parse().map_err(|_| Error::InvalidType {
            expected: "i64",
            got: self.value.to_string(),
        })?;
        visitor.visit_i64(n)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let n: u8 = self.value.parse().map_err(|_| Error::InvalidType {
            expected: "u8",
            got: self.value.to_string(),
        })?;
        visitor.visit_u8(n)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let n: u16 = self.value.parse().map_err(|_| Error::InvalidType {
            expected: "u16",
            got: self.value.to_string(),
        })?;
        visitor.visit_u16(n)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let n: u32 = self.value.parse().map_err(|_| Error::InvalidType {
            expected: "u32",
            got: self.value.to_string(),
        })?;
        visitor.visit_u32(n)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let n: u64 = self.value.parse().map_err(|_| Error::InvalidType {
            expected: "u64",
            got: self.value.to_string(),
        })?;
        visitor.visit_u64(n)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let n: f32 = self.value.parse().map_err(|_| Error::InvalidType {
            expected: "f32",
            got: self.value.to_string(),
        })?;
        visitor.visit_f32(n)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let n: f64 = self.value.parse().map_err(|_| Error::InvalidType {
            expected: "f64",
            got: self.value.to_string(),
        })?;
        visitor.visit_f64(n)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut chars = self.value.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => visitor.visit_char(c),
            _ => Err(Error::InvalidType {
                expected: "char",
                got: self.value.to_string(),
            }),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_str(self.value)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_str(self.value)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_bytes(self.value.as_bytes())
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.value.is_empty() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::InvalidType {
            expected: "sequence",
            got: "string".to_string(),
        })
    }

    fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::InvalidType {
            expected: "tuple",
            got: "string".to_string(),
        })
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::InvalidType {
            expected: "tuple struct",
            got: "string".to_string(),
        })
    }

    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::InvalidType {
            expected: "map",
            got: "string".to_string(),
        })
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::InvalidType {
            expected: "struct",
            got: "string".to_string(),
        })
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_enum(self.value.into_deserializer())
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_str(self.value)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[test]
    fn test_simple_struct() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Simple {
            name: String,
            age: u32,
        }

        let mut data = IndexMap::new();
        data.insert("name".to_string(), "Alice".to_string());
        data.insert("age".to_string(), "30".to_string());

        let de = FlattenedMapDeserializer::new(&data);
        let result: Simple = Simple::deserialize(de).unwrap();

        assert_eq!(
            result,
            Simple {
                name: "Alice".to_string(),
                age: 30
            }
        );
    }

    #[test]
    fn test_nested_struct() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Inner {
            value: i32,
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct Outer {
            inner: Inner,
            label: String,
        }

        let mut data = IndexMap::new();
        data.insert("inner__value".to_string(), "42".to_string());
        data.insert("label".to_string(), "test".to_string());

        let de = FlattenedMapDeserializer::new(&data);
        let result: Outer = Outer::deserialize(de).unwrap();

        assert_eq!(
            result,
            Outer {
                inner: Inner { value: 42 },
                label: "test".to_string()
            }
        );
    }

    #[test]
    fn test_string_that_looks_like_number() {
        // This is the key test case - "123" should deserialize as String, not fail
        #[derive(Debug, Deserialize, PartialEq)]
        struct Data {
            id: String,
        }

        let mut data = IndexMap::new();
        data.insert("id".to_string(), "123".to_string());

        let de = FlattenedMapDeserializer::new(&data);
        let result: Data = Data::deserialize(de).unwrap();

        assert_eq!(
            result,
            Data {
                id: "123".to_string()
            }
        );
    }

    #[test]
    fn test_option_with_empty_string_is_none() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Data {
            name: String,
            age: Option<u32>,
        }

        let mut data = IndexMap::new();
        data.insert("name".to_string(), "Alice".to_string());
        data.insert("age".to_string(), "".to_string()); // empty = null in CSV

        let de = FlattenedMapDeserializer::new(&data);
        let result: Data = Data::deserialize(de).unwrap();

        assert_eq!(
            result,
            Data {
                name: "Alice".to_string(),
                age: None
            }
        );
    }

    #[test]
    fn test_option_with_value_is_some() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Data {
            name: String,
            age: Option<u32>,
        }

        let mut data = IndexMap::new();
        data.insert("name".to_string(), "Bob".to_string());
        data.insert("age".to_string(), "25".to_string());

        let de = FlattenedMapDeserializer::new(&data);
        let result: Data = Data::deserialize(de).unwrap();

        assert_eq!(
            result,
            Data {
                name: "Bob".to_string(),
                age: Some(25)
            }
        );
    }

    #[test]
    fn test_option_with_missing_key_is_none() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Data {
            name: String,
            age: Option<u32>,
        }

        let mut data = IndexMap::new();
        data.insert("name".to_string(), "Charlie".to_string());
        // age key not present at all

        let de = FlattenedMapDeserializer::new(&data);
        let result: Data = Data::deserialize(de).unwrap();

        assert_eq!(
            result,
            Data {
                name: "Charlie".to_string(),
                age: None
            }
        );
    }

    #[test]
    fn test_nested_option_with_empty_string() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Inner {
            value: i32,
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct Outer {
            label: String,
            inner: Option<Inner>,
        }

        // Case 1: nested struct has data
        let mut data = IndexMap::new();
        data.insert("label".to_string(), "test".to_string());
        data.insert("inner__value".to_string(), "42".to_string());

        let de = FlattenedMapDeserializer::new(&data);
        let result: Outer = Outer::deserialize(de).unwrap();

        assert_eq!(
            result,
            Outer {
                label: "test".to_string(),
                inner: Some(Inner { value: 42 })
            }
        );

        // Case 2: nested struct fields are missing
        let mut data = IndexMap::new();
        data.insert("label".to_string(), "test2".to_string());

        let de = FlattenedMapDeserializer::new(&data);
        let result: Outer = Outer::deserialize(de).unwrap();

        assert_eq!(
            result,
            Outer {
                label: "test2".to_string(),
                inner: None
            }
        );

        // Case 3: nested struct fields exist but all have empty values (CSV null pattern)
        #[derive(Debug, Deserialize, PartialEq)]
        struct Price {
            amount: String,
            currency: String,
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct Product {
            name: String,
            price: Option<Price>,
        }

        let mut data = IndexMap::new();
        data.insert("name".to_string(), "Widget".to_string());
        data.insert("price__amount".to_string(), "".to_string());
        data.insert("price__currency".to_string(), "".to_string());

        let de = FlattenedMapDeserializer::new(&data);
        let result: Product = Product::deserialize(de).unwrap();

        assert_eq!(
            result,
            Product {
                name: "Widget".to_string(),
                price: None // All child fields empty = None
            }
        );

        // Case 4: nested struct has some empty and some non-empty values
        let mut data = IndexMap::new();
        data.insert("name".to_string(), "Gadget".to_string());
        data.insert("price__amount".to_string(), "".to_string());
        data.insert("price__currency".to_string(), "USD".to_string());

        let de = FlattenedMapDeserializer::new(&data);
        let result: Product = Product::deserialize(de).unwrap();

        assert_eq!(
            result,
            Product {
                name: "Gadget".to_string(),
                price: Some(Price {
                    amount: "".to_string(),
                    currency: "USD".to_string()
                })
            }
        );
    }

    #[test]
    fn test_option_string_empty_is_none() {
        // Edge case: Option<String> with empty string should be None, not Some("")
        #[derive(Debug, Deserialize, PartialEq)]
        struct Data {
            nickname: Option<String>,
        }

        let mut data = IndexMap::new();
        data.insert("nickname".to_string(), "".to_string());

        let de = FlattenedMapDeserializer::new(&data);
        let result: Data = Data::deserialize(de).unwrap();

        assert_eq!(result, Data { nickname: None });
    }
}
