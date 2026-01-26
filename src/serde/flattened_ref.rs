use {
    crate::FlattenedRef,
    serde::{Serialize, ser::SerializeStruct},
    std::{cell::RefCell, collections::HashMap},
};

#[derive(Debug, Default)]
struct StaticLookup(HashMap<Box<str>, &'static str>);

impl StaticLookup {
    fn intern(&mut self, val: impl AsRef<str>) -> &'static str {
        if !self.0.contains_key(val.as_ref()) {
            self.0
                .insert(Box::from(val.as_ref()), Box::leak(Box::from(val.as_ref())));
        }
        self.0.get(val.as_ref()).expect("checked above")
    }
}

thread_local! {
    static STATIC_LOOKUP: RefCell<StaticLookup> = Default::default();
}

impl<T> Serialize for FlattenedRef<'_, T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serde_json::to_value(self.0)
            .map_err(serde::ser::Error::custom)
            .map(crate::flatten_json_value::flatten::flattened)
            .and_then({
                move |v| {
                    serializer
                        .serialize_struct(std::any::type_name::<Self>(), v.len())
                        .and_then(|mut serialize_struct| {
                            v.into_iter()
                                .try_for_each(|(k, v)| {
                                    serialize_struct.serialize_field(
                                        STATIC_LOOKUP.with_borrow_mut(|static_lookup| {
                                            static_lookup.intern(k)
                                        }),
                                        &v,
                                    )
                                })
                                .and_then(|()| serialize_struct.end())
                        })
                }
            })
    }
}
