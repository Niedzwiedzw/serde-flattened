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

#[extension_traits::extension(trait CustomDeErrorContextExt)]
impl<T, E: std::error::Error> std::result::Result<T, E> {
    fn serde_context<Err: serde::de::Error>(self, context: &str) -> std::result::Result<T, Err> {
        self.map_err(|e| serde::de::Error::custom(format!("{e:?}\n{context}")))
    }
    fn with_serde_context<Err: serde::de::Error>(
        self,
        with_context: impl FnOnce() -> String,
    ) -> std::result::Result<T, Err> {
        self.map_err(|e| serde::de::Error::custom(format!("{e:?}\n{}", with_context())))
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
            .serde_context("deserializing as serde_json::Value")
            .and_then(|value| {
                crate::flatten_json_value::unflatten::unflattened(value.clone())
                    .with_serde_context(|| format!("unflattening value:\n{value:#?}"))
            })
            .and_then(|value| {
                serde_json::from_value::<T>(value.clone()).with_serde_context(|| {
                    format!("converting to {}:\n{value:#?}", std::any::type_name::<T>())
                })
            })
            .map(Self)
    }
}
