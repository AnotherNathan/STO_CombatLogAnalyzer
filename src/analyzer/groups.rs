use super::{values_manager::Values, *};
use std::fmt::Debug;

pub trait AnalysisGroup: Clone + Debug {
    type Value: Clone;

    fn name(&self) -> NameHandle;
    fn sub_groups(&self) -> &NameMap<Self>;
    fn sub_groups_mut(&mut self) -> &mut NameMap<Self>;

    fn values(&self) -> &Values<Self::Value>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GroupPathSegment {
    Group(NameHandle),
    Value(NameHandle),
}

impl Default for GroupPathSegment {
    #[inline]
    fn default() -> Self {
        Self::Group(Default::default())
    }
}

pub(super) trait AnalysisGroupInternal: AnalysisGroup {
    fn new_leaf(segment: GroupPathSegment) -> Self;
    fn new_branch(segment: GroupPathSegment) -> Self;

    fn segment(&self) -> GroupPathSegment;

    #[inline]
    fn is_leaf(&self) -> bool {
        self.values().is_leaf()
    }
    #[inline]
    fn is_branch(&self) -> bool {
        self.values().is_branch()
    }

    fn get_sub_group(&self, sub_group: GroupPathSegment) -> Option<&Self> {
        self.sub_groups().get(&sub_group.name())
    }

    fn get_sub_group_mut(&mut self, sub_group: GroupPathSegment) -> Option<&mut Self> {
        self.sub_groups_mut().get_mut(&sub_group.name())
    }

    fn get_leaf_sub_group(&mut self, sub_group: GroupPathSegment) -> &mut Self {
        let candidate = self.get_sub_group(sub_group);

        match candidate {
            Some(candidate) if candidate.is_leaf() && candidate.segment() == sub_group => {
                self.get_sub_group_mut(sub_group).unwrap()
            }
            Some(_) => self
                .get_sub_group_mut(sub_group)
                .unwrap()
                .get_leaf_sub_group(sub_group),
            None => {
                let leaf = Self::new_leaf(sub_group);
                self.sub_groups_mut().insert(sub_group.name(), leaf);
                self.get_sub_group_mut(sub_group).unwrap()
            }
        }
    }
    fn get_branch_sub_group(&mut self, sub_group: GroupPathSegment) -> &mut Self {
        let candidate = self.get_sub_group(sub_group);

        match candidate {
            Some(candidate) if candidate.is_branch() && candidate.segment() == sub_group => {
                self.get_sub_group_mut(sub_group).unwrap()
            }
            Some(candidate)
                if (candidate.is_branch() && candidate.segment().is_value()
                    || candidate.is_leaf()) =>
            {
                let value_or_leaf = self.sub_groups_mut().remove(&sub_group.name()).unwrap();
                let mut branch = Self::new_branch(sub_group);
                branch
                    .sub_groups_mut()
                    .insert(value_or_leaf.name(), value_or_leaf);
                self.sub_groups_mut().insert(sub_group.name(), branch);
                self.get_sub_group_mut(sub_group).unwrap()
            }
            Some(_) => self
                .get_sub_group_mut(sub_group)
                .unwrap()
                .get_branch_sub_group(sub_group),
            None => {
                let branch = Self::new_branch(sub_group);
                self.sub_groups_mut().insert(sub_group.name(), branch);
                self.get_sub_group_mut(sub_group).unwrap()
            }
        }
    }
}

#[derive(Clone, Debug, Educe, Default)]
#[educe(Deref, DerefMut)]
pub struct DamageGroup {
    pub segment: GroupPathSegment,
    pub sub_groups: NameMap<Self>,

    #[educe(Deref, DerefMut)]
    pub damage_metrics: DamageMetrics,
    pub max_one_hit: MaxOneHit,
    pub damage_percentage: ShieldHullOptionalValues,
    pub hits_percentage: ShieldHullOptionalValues,
    pub hits: Hits,
    pub damage_types: NameSet,

    pub kills: NameMap<u32>,
}

impl AnalysisGroup for DamageGroup {
    #[inline]
    fn name(&self) -> NameHandle {
        self.segment.name()
    }

    #[inline]
    fn sub_groups(&self) -> &NameMap<Self> {
        &self.sub_groups
    }

    #[inline]
    fn sub_groups_mut(&mut self) -> &mut NameMap<Self> {
        &mut self.sub_groups
    }

    type Value = Hit;

    #[inline]
    fn values(&self) -> &Values<Self::Value> {
        &self.hits
    }
}

impl AnalysisGroupInternal for DamageGroup {
    fn new_leaf(segment: GroupPathSegment) -> Self {
        Self {
            segment,
            hits: Values::empty_leaf(),
            ..Default::default()
        }
    }

    fn new_branch(segment: GroupPathSegment) -> Self {
        Self {
            segment,
            hits: Values::empty_branch(),
            ..Default::default()
        }
    }

    #[inline]
    fn segment(&self) -> GroupPathSegment {
        self.segment
    }
}

#[derive(Clone, Debug, Educe, Default)]
#[educe(Deref, DerefMut)]
pub struct HealGroup {
    pub segment: GroupPathSegment,
    pub sub_groups: NameMap<Self>,

    #[educe(Deref, DerefMut)]
    pub heal_metrics: HealMetrics,

    pub heal_percentage: ShieldHullOptionalValues,
    pub ticks_percentage: ShieldHullOptionalValues,

    pub ticks: HealTicks,
}

impl AnalysisGroup for HealGroup {
    #[inline]
    fn name(&self) -> NameHandle {
        self.segment.name()
    }

    #[inline]
    fn sub_groups(&self) -> &NameMap<Self> {
        &self.sub_groups
    }

    #[inline]
    fn sub_groups_mut(&mut self) -> &mut NameMap<Self> {
        &mut self.sub_groups
    }

    type Value = HealTick;

    #[inline]
    fn values(&self) -> &Values<Self::Value> {
        &self.ticks
    }
}

impl AnalysisGroupInternal for HealGroup {
    fn new_leaf(segment: GroupPathSegment) -> Self {
        Self {
            segment,
            ticks: Values::empty_leaf(),
            ..Default::default()
        }
    }

    fn new_branch(segment: GroupPathSegment) -> Self {
        Self {
            segment,
            ticks: Values::empty_branch(),
            ..Default::default()
        }
    }

    #[inline]
    fn segment(&self) -> GroupPathSegment {
        self.segment
    }
}

impl DamageGroup {
    pub(super) fn recalculate_metrics(
        &mut self,
        combat_duration: f64,
        hits_manager: &mut HitsManager,
        apply_delta: &mut dyn FnMut(&DamageMetricsDelta, &MaxOneHit),
    ) {
        if self.is_leaf() {
            hits_manager.add_leaf(self.hits.get_leaf());
            let delta_hits = &self.hits.get(hits_manager)[self.damage_metrics.hits.all as usize..];
            if delta_hits.len() > 0 {
                self.max_one_hit.update_from_hits(self.name(), delta_hits);
                let delta = self.damage_metrics.calc_and_apply_delta(delta_hits);
                apply_delta(&delta, &self.max_one_hit);
            }
        } else {
            self.kills.clear();

            self.hits = hits_manager.track_group(|hits_manager| {
                for sub_group in self.sub_groups.values_mut() {
                    sub_group.recalculate_metrics(combat_duration, hits_manager, &mut |d, m| {
                        self.damage_metrics.apply_delta(d);
                        self.max_one_hit.update(m.name, m.damage);
                        if self.segment.is_value() {
                            self.max_one_hit.name = self.segment.name();
                        }
                        apply_delta(d, &self.max_one_hit);
                    });
                    for damage_type in sub_group.damage_types.iter() {
                        if !self.damage_types.contains(damage_type) {
                            self.damage_types.insert(damage_type.clone());
                        }
                    }

                    for (&name, &kills) in sub_group.kills.iter() {
                        *self.kills.entry(name).or_default() += kills;
                    }
                }
            });
        }
        self.damage_metrics
            .recalculate_time_based_metrics(combat_duration);
    }

    pub(super) fn recalculate_percentages(
        &mut self,
        parent_total_damage: &ShieldHullValues,
        parent_hits: &ShieldHullCounts,
    ) {
        self.damage_percentage =
            ShieldHullOptionalValues::percentage(&self.total_damage, parent_total_damage);
        self.hits_percentage = ShieldHullOptionalValues::percentage(
            &self.damage_metrics.hits.to_values(),
            &parent_hits.to_values(),
        );
        self.sub_groups.values_mut().for_each(|s| {
            s.recalculate_percentages(&self.damage_metrics.total_damage, &self.damage_metrics.hits)
        });
    }

    pub(super) fn add_damage(
        &mut self,
        path: &[GroupPathSegment],
        hit: BaseHit,
        flags: ValueFlags,
        damage_type: NameHandle,
        combat_start_offset_millis: u32,
        name_manager: &NameManager,
    ) {
        if path.len() == 1 {
            let indirect_source = self.get_leaf_sub_group(path[0]);
            indirect_source
                .hits
                .push(hit.to_hit(combat_start_offset_millis));
            indirect_source.add_damage_type_non_pool(damage_type, name_manager);

            if flags.contains(ValueFlags::KILL) {
                *indirect_source.kills.entry(path[0].name()).or_default() += 1;
            }

            return;
        }

        let indirect_source = self.get_branch_sub_group(*path.last().unwrap());
        indirect_source.add_damage(
            &path[..path.len() - 1],
            hit,
            flags,
            damage_type,
            combat_start_offset_millis,
            name_manager,
        );
    }

    pub(super) fn add_damage_type_non_pool(
        &mut self,
        damage_type: NameHandle,
        name_manager: &NameManager,
    ) {
        let shield_handle = name_manager.get_handle("Shield");
        if damage_type == NameHandle::UNKNOWN {
            return;
        }

        if self.damage_types.contains(&damage_type) {
            return;
        }

        if self.damage_types.contains(&damage_type) {
            return;
        }

        if Some(damage_type) == shield_handle && !self.damage_types.is_empty() {
            return;
        }

        if shield_handle
            .map(|s| damage_type != s && self.damage_types.contains(&s))
            .unwrap_or(false)
        {
            self.damage_types.remove(&shield_handle.unwrap());
        }

        self.damage_types.insert(damage_type);
    }
}

impl HealGroup {
    pub(super) fn recalculate_metrics(
        &mut self,
        combat_duration: f64,
        ticks_manager: &mut HealTicksManager,
        apply_delta: &mut dyn FnMut(&HealMetricsDelta),
    ) {
        if self.is_leaf() {
            ticks_manager.add_leaf(self.ticks.get_leaf());
            let delta_ticks =
                &self.ticks.get(ticks_manager)[self.heal_metrics.ticks.all as usize..];
            if delta_ticks.len() > 0 {
                let delta = self.heal_metrics.calc_and_apply(delta_ticks);
                apply_delta(&delta);
            }
        } else {
            self.ticks = ticks_manager.track_group(|ticks_manager| {
                for sub_group in self.sub_groups.values_mut() {
                    sub_group.recalculate_metrics(combat_duration, ticks_manager, &mut |d| {
                        self.heal_metrics.apply_delta(d);
                        apply_delta(d);
                    });
                }
            });
        }
        self.heal_metrics
            .recalculate_time_based_metrics(combat_duration);
    }

    pub(super) fn recalculate_percentages(
        &mut self,
        parent_total_heal: &ShieldHullValues,
        parent_ticks: &ShieldHullCounts,
    ) {
        self.heal_percentage =
            ShieldHullOptionalValues::percentage(&self.total_heal, parent_total_heal);
        self.ticks_percentage = ShieldHullOptionalValues::percentage(
            &self.heal_metrics.ticks.to_values(),
            &parent_ticks.to_values(),
        );
        self.sub_groups.values_mut().for_each(|s| {
            s.recalculate_percentages(&self.heal_metrics.total_heal, &self.heal_metrics.ticks)
        });
    }

    pub(super) fn add_heal(
        &mut self,
        path: &[GroupPathSegment],
        tick: BaseHealTick,
        flags: ValueFlags,
        combat_start_offset_millis: u32,
    ) {
        if path.len() == 1 {
            let indirect_source = self.get_leaf_sub_group(path[0]);
            indirect_source
                .ticks
                .push(tick.to_tick(combat_start_offset_millis));

            return;
        }

        let indirect_source = self.get_branch_sub_group(*path.last().unwrap());
        indirect_source.add_heal(
            &path[..path.len() - 1],
            tick,
            flags,
            combat_start_offset_millis,
        );
    }
}

impl GroupPathSegment {
    #[inline]
    pub fn name(&self) -> NameHandle {
        match *self {
            GroupPathSegment::Group(n) => n,
            GroupPathSegment::Value(n) => n,
        }
    }

    #[inline]
    pub fn is_value(&self) -> bool {
        if let Self::Value(_) = self {
            return true;
        }

        false
    }
}
