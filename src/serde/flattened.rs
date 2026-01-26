use {
    crate::Flattened,
    serde::{Deserialize, Serialize, de::DeserializeOwned},
    tracing::instrument,
};

impl<T> Serialize for Flattened<T>
where
    T: Serialize,
{
    #[instrument(skip_all)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.as_ref().serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for Flattened<T>
where
    T: DeserializeOwned,
{
    #[instrument(skip(deserializer))]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        serde_json::Value::deserialize(deserializer)
            .and_then(|value| crate::flatten_json_value::unflatten::unflattened(value).map_err(serde::de::Error::custom))
            .and_then(|value| serde_json::from_value::<T>(value).map_err(serde::de::Error::custom))
            .map(Self)
    }
}
