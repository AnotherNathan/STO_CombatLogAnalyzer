use super::*;
use std::fmt::Debug;

pub trait AnalysisGroup: Clone + Debug {
    fn name(&self) -> NameHandle;
    fn sub_groups(&self) -> &NameMap<Self>;
    fn sub_groups_mut(&mut self) -> &mut NameMap<Self>;

    fn is_pool(&self) -> bool;
}

pub(super) trait AnalysisGroupInternal: AnalysisGroup {
    fn new(name: NameHandle, is_pool: bool) -> Self;

    fn get_non_pool_sub_group(&mut self, sub_group: NameHandle) -> &mut Self {
        let candidate = self.get_sub_group_or_create_non_pool(sub_group);
        if !candidate.is_pool() {
            return candidate;
        }

        candidate.get_non_pool_sub_group(sub_group)
    }

    fn get_pool_sub_group(&mut self, sub_group: NameHandle) -> &mut Self {
        let candidate = self.sub_groups().get(&sub_group);
        if candidate.map(|c| c.is_pool()).unwrap_or(false) {
            return self.get_sub_group_or_create_non_pool(sub_group);
        }

        // make a new pool and move the non pool sub group on there
        let mut pool = Self::new(sub_group, true);
        if let Some(non_pool_sub_group) = self.sub_groups_mut().remove(&sub_group) {
            pool.sub_groups_mut().insert(sub_group, non_pool_sub_group);
        }
        self.sub_groups_mut().insert(sub_group, pool);
        self.get_pool_sub_group(sub_group)
    }

    fn get_sub_group_or_create_non_pool(&mut self, sub_group: NameHandle) -> &mut Self {
        if !self.sub_groups().contains_key(&sub_group) {
            self.sub_groups_mut()
                .insert(sub_group, Self::new(sub_group, false));
        }

        self.sub_groups_mut().get_mut(&sub_group).unwrap()
    }
}

#[derive(Clone, Debug, Educe, Default)]
#[educe(Deref, DerefMut)]
pub struct DamageGroup {
    pub name: NameHandle,
    pub sub_groups: NameMap<Self>,

    is_pool: bool,

    #[educe(Deref, DerefMut)]
    pub damage_metrics: DamageMetrics,
    pub max_one_hit: MaxOneHit,
    pub damage_percentage: ShieldHullOptionalValues,
    pub hits_percentage: ShieldHullOptionalValues,
    pub hits: Vec<Hit>,
    pub damage_types: NameSet,
}

impl AnalysisGroup for DamageGroup {
    #[inline]
    fn name(&self) -> NameHandle {
        self.name
    }

    #[inline]
    fn sub_groups(&self) -> &NameMap<Self> {
        &self.sub_groups
    }

    #[inline]
    fn sub_groups_mut(&mut self) -> &mut NameMap<Self> {
        &mut self.sub_groups
    }

    #[inline]
    fn is_pool(&self) -> bool {
        self.is_pool
    }
}

impl AnalysisGroupInternal for DamageGroup {
    fn new(name: NameHandle, is_pool: bool) -> Self {
        Self {
            name,
            is_pool,
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug, Educe, Default)]
#[educe(Deref, DerefMut)]
pub struct HealGroup {
    pub name: NameHandle,
    pub sub_groups: NameMap<Self>,

    is_pool: bool,

    #[educe(Deref, DerefMut)]
    pub heal_metrics: HealMetrics,

    pub heal_percentage: ShieldHullOptionalValues,
    pub ticks_percentage: ShieldHullOptionalValues,

    pub ticks: Vec<HealTick>,
}

impl AnalysisGroup for HealGroup {
    #[inline]
    fn name(&self) -> NameHandle {
        self.name
    }

    #[inline]
    fn sub_groups(&self) -> &NameMap<Self> {
        &self.sub_groups
    }

    #[inline]
    fn sub_groups_mut(&mut self) -> &mut NameMap<Self> {
        &mut self.sub_groups
    }

    #[inline]
    fn is_pool(&self) -> bool {
        self.is_pool
    }
}

impl AnalysisGroupInternal for HealGroup {
    fn new(name: NameHandle, is_pool: bool) -> Self {
        Self {
            name,
            is_pool,
            ..Default::default()
        }
    }
}

impl DamageGroup {
    pub(super) fn recalculate_metrics(&mut self, combat_duration: f64) {
        if self.sub_groups.len() > 0 {
            self.max_one_hit.reset();
            self.hits.clear();
            self.damage_types.clear();

            for sub_group in self.sub_groups.values_mut() {
                sub_group.recalculate_metrics(combat_duration);
                self.hits.extend_from_slice(&sub_group.hits);
                self.max_one_hit
                    .update(sub_group.max_one_hit.name, sub_group.max_one_hit.damage);
                for damage_type in sub_group.damage_types.iter() {
                    if !self.damage_types.contains(damage_type) {
                        self.damage_types.insert(damage_type.clone());
                    }
                }
            }
        } else {
            self.max_one_hit = MaxOneHit::from_hits(self.name, &self.hits);
        }

        self.damage_metrics = DamageMetrics::calculate(&self.hits, combat_duration);
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
        path: &[NameHandle],
        hit: BaseHit,
        flags: ValueFlags,
        damage_type: NameHandle,
        combat_start_offset_millis: u32,
        name_manager: &NameManager,
    ) {
        if path.len() == 1 {
            let indirect_source = self.get_non_pool_sub_group(path[0]);
            indirect_source
                .hits
                .push(hit.to_hit(combat_start_offset_millis));
            indirect_source.add_damage_type_non_pool(damage_type, name_manager);

            return;
        }

        let indirect_source = self.get_pool_sub_group(*path.last().unwrap());
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
    pub(super) fn recalculate_metrics(&mut self, combat_duration: f64) {
        if self.sub_groups.len() > 0 {
            self.ticks.clear();

            for sub_group in self.sub_groups.values_mut() {
                sub_group.recalculate_metrics(combat_duration);
                self.ticks.extend_from_slice(&sub_group.ticks);
            }
        }

        self.heal_metrics = HealMetrics::calculate(&self.ticks, combat_duration);
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
        path: &[NameHandle],
        tick: BaseHealTick,
        flags: ValueFlags,
        combat_start_offset_millis: u32,
    ) {
        if path.len() == 1 {
            let indirect_source = self.get_non_pool_sub_group(path[0]);
            indirect_source
                .ticks
                .push(tick.to_tick(combat_start_offset_millis));

            return;
        }

        let indirect_source = self.get_pool_sub_group(*path.last().unwrap());
        indirect_source.add_heal(
            &path[..path.len() - 1],
            tick,
            flags,
            combat_start_offset_millis,
        );
    }
}
