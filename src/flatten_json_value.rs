#![allow(clippy::unit_arg)]
use {
    std::borrow::Cow,
    tap::{Pipe, Tap},
};

const ARR_PFX: &str = "idx-";
const JOIN_TAG: &str = "__";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Segment<'a> {
    Idx(usize),
    Field(Cow<'a, str>),
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for Segment<'_> {
    fn to_string(&self) -> String {
        match self {
            Segment::Idx(idx) => format!("{ARR_PFX}{idx}"),
            Segment::Field(cow) => cow.as_ref().to_string(),
        }
    }
}
impl<'a> Segment<'a> {
    #[expect(clippy::should_implement_trait, reason = "this can never fail")]
    pub fn from_str(idx: &'a str) -> Segment<'a> {
        idx.strip_prefix(ARR_PFX)
            .and_then(|idx| idx.parse::<usize>().ok())
            .map(Segment::Idx)
            .unwrap_or_else(|| idx.pipe(Cow::Borrowed).pipe(Segment::Field))
    }
    pub fn to_owned(&self) -> Segment<'static> {
        match self {
            Segment::Idx(idx) => Segment::Idx(*idx),
            Segment::Field(cow) => cow.to_string().pipe(Cow::<str>::Owned).pipe(Segment::Field),
        }
    }

    pub fn as_ref<'b>(&'b self) -> Segment<'b> {
        match self {
            Segment::Idx(idx) => Segment::Idx(*idx),
            Segment::Field(cow) => cow.as_ref().pipe(Cow::Borrowed).pipe(Segment::Field),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct FieldPath<'a>(Vec<Segment<'a>>);

impl<'a> FieldPath<'a> {
    pub fn pop_start(mut self) -> Option<(Segment<'a>, Self)> {
        match self.0.len() {
            0 => None,
            _ => Some((self.0.remove(0), self)),
        }
    }
    pub fn to_owned(&self) -> FieldPath<'static> {
        self.0
            .iter()
            .map(Segment::to_owned)
            .collect::<Vec<_>>()
            .pipe(FieldPath)
    }
    pub fn join(&self, segment: Segment<'a>) -> Self {
        self.clone().tap_mut(|p| p.0.push(segment))
    }
    pub fn as_ref<'b>(&'b self) -> FieldPath<'b> {
        FieldPath(self.0.iter().map(|b| b.as_ref()).collect())
    }
}

pub fn boxed_iter<'a, T, I>(iter: I) -> Box<dyn Iterator<Item = T> + 'a>
where
    T: 'a,
    I: Iterator<Item = T> + 'a,
{
    Box::new(iter)
}

pub mod flatten;
pub mod unflatten;
