use std::ops::Range;

use super::{HealTick, Hit};

pub type HitsManager = ValuesManager<Hit>;
pub type Hits = Values<Hit>;

pub type HealTicksManager = ValuesManager<HealTick>;
pub type HealTicks = Values<HealTick>;

#[derive(Debug, Clone, Default)]
pub struct ValuesManager<T> {
    values: Vec<T>,
}

#[derive(Debug, Clone)]
pub enum Values<T> {
    Leaf(Vec<T>),
    Group(Range<usize>),
}

impl<T: Clone> ValuesManager<T> {
    pub fn track_group(&mut self, tracked: impl FnOnce(&mut Self)) -> Values<T> {
        let start = self.values.len();
        tracked(self);

        Values::Group(start..self.values.len())
    }

    #[inline]
    pub fn get<'a>(&'a self, hits: &'a Values<T>) -> &'a [T] {
        match hits {
            Values::Leaf(values) => &values,
            Values::Group(range) => &self.values[range.clone()],
        }
    }

    pub fn add_group(&mut self, values: &[T]) {
        self.values.extend_from_slice(values);
    }

    pub fn clear(&mut self) {
        self.values.clear();
    }
}

impl<T: Clone> Values<T> {
    pub const EMPTY_GROUP: Self = Self::Group(0..0);

    #[inline]
    pub const fn empty_leaf() -> Self {
        Self::Leaf(Vec::new())
    }

    #[inline]
    pub fn get<'a>(&'a self, manager: &'a ValuesManager<T>) -> &[T] {
        manager.get(self)
    }
}
