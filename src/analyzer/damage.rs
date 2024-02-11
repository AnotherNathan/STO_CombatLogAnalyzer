use super::*;
use educe::Educe;

#[derive(Clone, Copy, Debug)]
pub struct BaseHit {
    pub damage: f64,
    pub flags: ValueFlags,
    pub specific: SpecificHit,
}

#[derive(Clone, Copy, Debug, Educe)]
#[educe(Deref, DerefMut)]
pub struct Hit {
    #[educe(Deref, DerefMut)]
    pub hit: BaseHit,
    pub time_millis: u32, // offset to start of combat
}

#[derive(Clone, Copy, Debug)]
pub enum SpecificHit {
    Shield { damage_prevented_to_hull: f64 },
    ShieldDrain,
    Hull { base_damage: f64 },
}

#[derive(Clone, Debug, Default)]
pub struct DamageMetrics {
    pub hits: ShieldHullCounts,
    pub hits_per_second: ShieldHullValues,
    pub misses: u64,
    pub accuracy_percentage: Option<f64>,
    pub total_damage: ShieldHullValues,
    pub total_shield_drain: f64,
    pub total_damage_prevented_to_hull_by_shields: f64,
    pub total_base_damage: f64,
    pub base_dps: f64,
    pub dps: ShieldHullValues,
    pub average_hit: ShieldHullOptionalValues,
    pub critical_percentage: Option<f64>,
    pub flanking: Option<f64>,
    pub damage_resistance_percentage: Option<f64>,
    pub crits: u64,
    pub flanks: u64,
}

#[derive(Clone, Debug, Default)]
pub struct DamageMetricsDelta {
    pub hits: ShieldHullCounts,
    pub misses: u64,
    pub total_damage: ShieldHullValues,
    pub total_shield_drain: f64,
    pub total_damage_prevented_to_hull_by_shields: f64,
    pub total_base_damage: f64,
    pub crits: u64,
    pub flanks: u64,
}

#[derive(Clone, Debug, Default)]
pub struct MaxOneHit {
    pub name: NameHandle,
    pub damage: f64,
}

impl MaxOneHit {
    pub fn update_from_hits(&mut self, name: NameHandle, hits: &[Hit]) {
        hits.iter().for_each(|h| self.update(name, h.damage));
    }

    pub fn update(&mut self, name: NameHandle, damage: f64) {
        if self.damage < damage {
            self.damage = damage;
            self.name = name;
        }
    }
}

impl BaseHit {
    pub fn shield(damage: f64, flags: ValueFlags, damage_prevented_to_hull: f64) -> Self {
        Self {
            damage: damage.abs(),
            flags,
            specific: SpecificHit::Shield {
                damage_prevented_to_hull: damage_prevented_to_hull.abs(),
            },
        }
    }

    pub fn shield_drain(damage: f64, flags: ValueFlags) -> Self {
        Self {
            damage: damage.abs(),
            flags,
            specific: SpecificHit::ShieldDrain,
        }
    }

    pub fn hull(damage: f64, flags: ValueFlags, base_damage: f64) -> Self {
        Self {
            damage: damage.abs(),
            flags,
            specific: SpecificHit::Hull {
                base_damage: base_damage.abs(),
            },
        }
    }

    pub fn to_hit(self, time_millis: u32) -> Hit {
        Hit {
            hit: self,
            time_millis,
        }
    }
}

impl DamageMetrics {
    pub fn calc_and_apply_delta(&mut self, delta_hits: &[Hit]) -> DamageMetricsDelta {
        let mut delta = DamageMetricsDelta::default();

        for hit in delta_hits.iter() {
            match hit.specific {
                SpecificHit::Shield { .. } | SpecificHit::ShieldDrain => delta.hits.shield += 1,
                SpecificHit::Hull { .. } => delta.hits.hull += 1,
            }

            if hit.flags.contains(ValueFlags::IMMUNE) {
                continue;
            }

            match hit.specific {
                SpecificHit::Shield {
                    damage_prevented_to_hull,
                } => {
                    delta.total_damage.shield += hit.damage;
                    delta.total_damage_prevented_to_hull_by_shields += damage_prevented_to_hull;
                }
                SpecificHit::Hull { base_damage } => {
                    delta.total_damage.hull += hit.damage;
                    delta.total_base_damage += base_damage;
                }
                SpecificHit::ShieldDrain => {
                    delta.total_damage.shield += hit.damage;
                    delta.total_shield_drain += hit.damage;
                }
            }

            if hit.flags.contains(ValueFlags::CRITICAL) {
                delta.crits += 1;
            }

            if hit.flags.contains(ValueFlags::FLANK) {
                delta.flanks += 1;
            }

            if hit.flags.contains(ValueFlags::MISS) {
                delta.misses += 1;
            }
        }

        delta.hits.all = delta.hits.shield + delta.hits.hull;
        delta.total_damage.all = delta.total_damage.hull + delta.total_damage.shield;

        self.apply_delta(&delta);
        delta
    }

    pub fn apply_delta(&mut self, delta: &DamageMetricsDelta) {
        self.hits += delta.hits;
        self.total_damage += delta.total_damage;
        self.total_base_damage += delta.total_base_damage;
        self.total_damage_prevented_to_hull_by_shields +=
            delta.total_damage_prevented_to_hull_by_shields;
        self.total_shield_drain += delta.total_shield_drain;
        self.crits += delta.crits;
        self.flanks += delta.flanks;
        self.misses += delta.misses;

        self.critical_percentage = percentage_u64(self.crits, self.hits.hull);

        self.flanking = percentage_u64(self.flanks, self.hits.hull);
        self.accuracy_percentage = percentage_u64(self.misses, self.hits.hull).map(|m| 100.0 - m);

        self.damage_resistance_percentage = damage_resistance_percentage(
            &self.total_damage,
            self.total_base_damage,
            self.total_shield_drain,
        );
    }

    pub fn recalculate_time_based_metrics(&mut self, combat_duration: f64) {
        self.base_dps = self.total_base_damage / combat_duration.max(1.0);
        self.hits_per_second =
            ShieldHullValues::per_seconds(&self.hits.to_values(), combat_duration);

        self.dps = ShieldHullValues::per_seconds(&self.total_damage, combat_duration);
        self.average_hit = ShieldHullOptionalValues::average(
            &self.total_damage,
            self.hits.shield,
            self.hits.hull,
            self.hits.all,
        );
    }
}

pub fn damage_resistance_percentage(
    total_damage: &ShieldHullValues,
    total_base_damage: f64,
    total_shield_drain: f64,
) -> Option<f64> {
    if total_base_damage == 0.0 {
        return None;
    }

    let total_damage_without_drain = total_damage.all - total_shield_drain;

    let res = 1.0 - total_damage_without_drain / total_base_damage;
    Some(res * 100.0)
}
