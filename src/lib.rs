pub mod flatten_json_value;
pub mod nested_csv;

#[derive(Debug)]
pub struct Flattened<T>(T);

#[derive(Debug)]
pub struct FlattenedRef<'a, T>(&'a T);

impl<T> Flattened<T> {
    pub fn as_ref(&self) -> FlattenedRef<'_, T> {
        FlattenedRef(&self.0)
    }
}

mod serde;

#[cfg(test)]
mod test;
