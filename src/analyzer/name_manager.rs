use std::{
    collections::{HashMap, HashSet},
    hash::{BuildHasher, Hasher},
};

use bitflags::bitflags;
use rustc_hash::FxHashMap;

use super::settings::RulesGroup;

#[derive(Debug, Default, Clone)]
pub struct NameManager {
    name_infos: NameMap<NameInfo>,
    name_to_handle: FxHashMap<String, NameHandle>,

    handle_source: u32,
}

pub type NameMap<T> = HashMap<NameHandle, T, NameHandleBuildHasher>;
pub type NameSet = HashSet<NameHandle, NameHandleBuildHasher>;

#[derive(Debug, Default, Clone)]
pub struct NameInfo {
    pub name: String,
    pub flags: NameFlags,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    pub struct NameFlags : u8{
        const NONE = 0;
        const PLAYER = 1<<0;
        const SOURCE = 1<<1;
        const SOURCE_UNIQUE = 1<<2;
        const INDIRECT_SOURCE = 1<<3;
        const INDIRECT_SOURCE_UNIQUE = 1<<4;
        const TARGET = 1<<5;
        const TARGET_UNIQUE = 1<<6;
        const VALUE = 1<<7;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct NameHandle(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct NameHandleBuildHasher {}

pub struct NameHandleHasher {
    value: u64,
}

impl NameManager {
    pub fn insert_some(&mut self, name: Option<&str>, flags: NameFlags) -> Option<NameHandle> {
        let name = name?;
        Some(self.insert(name, flags))
    }

    pub fn insert(&mut self, name: &str, flags: NameFlags) -> NameHandle {
        if name.is_empty() {
            return NameHandle::UNKNOWN;
        }

        if let Some(handle) = self.name_to_handle.get(name) {
            self.name_infos.get_mut(handle).unwrap().flags |= flags;
            return *handle;
        }

        let handle = NameHandle(self.handle_source);
        self.handle_source += 1;
        self.name_to_handle.insert(name.to_string(), handle);
        let info = NameInfo {
            name: name.to_string(),
            flags,
        };
        self.name_infos.insert(handle, info);
        handle
    }

    #[inline]
    pub fn name(&self, handle: NameHandle) -> &str {
        if handle == NameHandle::UNKNOWN {
            return "<unknown>";
        }

        &self
            .name_infos
            .get(&handle)
            .expect("failed to find name from handle")
            .name
    }

    #[inline]
    #[allow(dead_code)]
    pub fn get_name(&self, handle: NameHandle) -> Option<&str> {
        if handle == NameHandle::UNKNOWN {
            return Some("<unknown>");
        }

        Some(&self.name_infos.get(&handle)?.name)
    }

    #[inline]
    pub fn handle(&self, name: &str) -> NameHandle {
        if name.is_empty() {
            return NameHandle::UNKNOWN;
        }
        *self
            .name_to_handle
            .get(name)
            .expect("failed to find handle from name")
    }

    #[inline]
    pub fn get_handle(&self, name: &str) -> Option<NameHandle> {
        if name.is_empty() {
            return Some(NameHandle::UNKNOWN);
        }
        self.name_to_handle.get(name).copied()
    }

    pub fn matches(&self, rule: &RulesGroup) -> bool {
        rule.matches_source_or_target_names(self.source_targets())
            || rule.matches_source_or_target_unique_names(self.source_targets_unique())
            || rule.matches_indirect_source_names(self.indirect_sources())
            || rule.matches_indirect_source_unique_names(self.indirect_sources_unique())
            || rule.matches_damage_or_heal_names(self.values())
    }

    #[inline]
    pub fn source_targets(&self) -> impl Iterator<Item = &str> + '_ {
        self.names_by_flags(NameFlags::SOURCE | NameFlags::TARGET)
    }

    #[inline]
    pub fn source_targets_unique(&self) -> impl Iterator<Item = &str> + '_ {
        self.names_by_flags(NameFlags::SOURCE_UNIQUE | NameFlags::TARGET_UNIQUE)
    }

    #[inline]
    pub fn indirect_sources(&self) -> impl Iterator<Item = &str> + '_ {
        self.names_by_flags(NameFlags::INDIRECT_SOURCE)
    }

    #[inline]
    pub fn indirect_sources_unique(&self) -> impl Iterator<Item = &str> + '_ {
        self.names_by_flags(NameFlags::INDIRECT_SOURCE_UNIQUE)
    }

    #[inline]
    pub fn values(&self) -> impl Iterator<Item = &str> + '_ {
        self.names_by_flags(NameFlags::VALUE)
    }

    #[inline]
    fn names_by_flags(&self, flags: NameFlags) -> impl Iterator<Item = &str> + '_ {
        self.name_infos
            .values()
            .filter(move |i| i.flags.intersects(flags))
            .map(|i| i.name.as_str())
    }
}

impl NameHandle {
    pub const UNKNOWN: Self = Self(u32::MAX);

    #[inline]
    pub fn get<'a>(&self, name_manager: &'a NameManager) -> &'a str {
        name_manager.name(*self)
    }
}

impl Hasher for NameHandleHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.value
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        self.value = u32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as _;
    }

    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.value = i as _;
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.value = i as _;
    }
}

impl BuildHasher for NameHandleBuildHasher {
    type Hasher = NameHandleHasher;

    #[inline]
    fn build_hasher(&self) -> Self::Hasher {
        NameHandleHasher { value: 0 }
    }
}

impl NameFlags {
    #[inline]
    pub fn set_if(mut self, other: Self, value: bool) -> Self {
        self.set(other, value);
        self
    }
}

impl Default for NameHandle {
    #[inline]
    fn default() -> Self {
        Self::UNKNOWN
    }
}
