use std::ops::Range;

use super::{HealTick, Hit};

pub type HitsManager = ValuesManager<Hit>;
pub type Hits = Values<Hit>;

pub type HealTicksManager = ValuesManager<HealTick>;
pub type HealTicks = Values<HealTick>;

#[derive(Debug, Clone)]
pub struct ValuesManager<T> {
    values: Vec<T>,
}

impl<T> Default for ValuesManager<T> {
    #[inline]
    fn default() -> Self {
        Self {
            values: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Values<T> {
    Leaf(Vec<T>),
    Branch(Range<usize>),
}

impl<T: Clone> ValuesManager<T> {
    pub fn track_group(&mut self, tracked: impl FnOnce(&mut Self)) -> Values<T> {
        let start = self.values.len();
        tracked(self);

        Values::Branch(start..self.values.len())
    }

    #[inline]
    pub fn get<'a>(&'a self, hits: &'a Values<T>) -> &'a [T] {
        match hits {
            Values::Leaf(values) => &values,
            Values::Branch(range) => &self.values[range.clone()],
        }
    }

    pub fn add_leaf(&mut self, values: &[T]) {
        self.values.extend_from_slice(values);
    }

    pub fn clear(&mut self) {
        self.values.clear();
    }
}

impl<T: Clone> Values<T> {
    #[inline]
    pub const fn empty_branch() -> Self {
        Self::Branch(0..0)
    }

    #[inline]
    pub const fn empty_leaf() -> Self {
        Self::Leaf(Vec::new())
    }

    #[inline]
    pub fn get<'a>(&'a self, manager: &'a ValuesManager<T>) -> &'a [T] {
        manager.get(self)
    }

    #[inline]
    pub fn is_leaf(&self) -> bool {
        if let Self::Leaf(_) = self {
            return true;
        }

        false
    }

    #[inline]
    pub fn is_branch(&self) -> bool {
        if let Self::Branch(_) = self {
            return true;
        }

        false
    }

    #[inline]
    pub fn push(&mut self, value: T) {
        let Self::Leaf(values) = self else {
            panic!("cannot push value to non leaf values")
        };
        values.push(value);
    }

    #[inline]
    pub fn get_leaf(&self) -> &[T] {
        let Self::Leaf(values) = self else {
            panic!("values is not a leaf")
        };
        &values
    }
}

impl<T: Clone> Default for Values<T> {
    #[inline]
    fn default() -> Self {
        Self::empty_branch()
    }
}
