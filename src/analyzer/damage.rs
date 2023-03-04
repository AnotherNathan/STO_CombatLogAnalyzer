use std::fmt::Write;

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
    pub total_damage_prevented_to_hull_by_shields: f64,
    pub total_base_damage: f64,
    pub base_dps: f64,
    pub dps: ShieldHullValues,
    pub average_hit: ShieldHullOptionalValues,
    pub critical_percentage: Option<f64>,
    pub flanking: Option<f64>,
    pub damage_resistance_percentage: Option<f64>,
}

#[derive(Clone, Debug, Default)]
pub struct MaxOneHit {
    pub name: String,
    pub damage: f64,
}

impl MaxOneHit {
    pub fn from_hits(name: &str, hits: &[Hit]) -> Self {
        Self {
            name: name.to_string(),
            damage: hits
                .iter()
                .map(|h| h.damage)
                .max_by(|d1, d2| d1.total_cmp(d2))
                .unwrap_or(0.0),
        }
    }

    pub fn update(&mut self, name: &str, damage: f64) {
        if self.damage < damage {
            self.damage = damage;
            self.name.clear();
            self.name.write_str(name).unwrap();
        }
    }

    pub fn reset(&mut self) {
        self.name.clear();
        self.damage = Default::default();
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
    pub fn calculate(hits: &[Hit], combat_duration: f64) -> Self {
        let mut total_shield_damage = 0.0;
        let mut total_hull_damage = 0.0;
        let mut shield_hits = 0;
        let mut hull_hits = 0;
        let mut misses = 0;
        let mut crits = 0;
        let mut flanks = 0;
        let mut total_damage_prevented_to_hull_by_shields = 0.0;
        let mut total_base_damage = 0.0;
        let mut total_shield_drain = 0.0;

        for hit in hits.iter() {
            match hit.specific {
                SpecificHit::Shield { .. } | SpecificHit::ShieldDrain => shield_hits += 1,
                SpecificHit::Hull { .. } => hull_hits += 1,
            }

            if hit.flags.contains(ValueFlags::IMMUNE) {
                continue;
            }

            match hit.specific {
                SpecificHit::Shield {
                    damage_prevented_to_hull,
                } => {
                    total_shield_damage += hit.damage;
                    total_damage_prevented_to_hull_by_shields += damage_prevented_to_hull;
                }
                SpecificHit::Hull { base_damage } => {
                    total_hull_damage += hit.damage;
                    total_base_damage += base_damage;
                }
                SpecificHit::ShieldDrain => {
                    total_shield_damage += hit.damage;
                    total_shield_drain += hit.damage;
                }
            }

            if hit.flags.contains(ValueFlags::CRITICAL) {
                crits += 1;
            }

            if hit.flags.contains(ValueFlags::FLANK) {
                flanks += 1;
            }

            if hit.flags.contains(ValueFlags::MISS) {
                misses += 1;
            }
        }

        let total_damage = total_hull_damage + total_shield_damage;
        let total_damage = ShieldHullValues {
            all: total_damage,
            hull: total_hull_damage,
            shield: total_shield_damage,
        };

        let critical_percentage = percentage_u64(crits, hull_hits);

        let flanking = percentage_u64(flanks, hull_hits);
        let accuracy_percentage = percentage_u64(misses, hull_hits).map(|m| 100.0 - m);

        let hits = ShieldHullCounts {
            all: shield_hits + hull_hits,
            hull: hull_hits,
            shield: shield_hits,
        };
        let hits_per_second = ShieldHullValues::per_seconds(&hits.to_values(), combat_duration);

        let dps = ShieldHullValues::per_seconds(&total_damage, combat_duration);
        let average_hit =
            ShieldHullOptionalValues::average(&total_damage, shield_hits, hull_hits, hits.all);

        let damage_resistance_percentage =
            damage_resistance_percentage(&total_damage, total_base_damage, total_shield_drain);

        let base_dps = total_base_damage / combat_duration.max(1.0);

        Self {
            hits,
            hits_per_second,
            misses,
            accuracy_percentage,
            total_damage,
            total_damage_prevented_to_hull_by_shields,
            total_base_damage,
            base_dps,
            dps,
            average_hit,
            critical_percentage,
            flanking,
            damage_resistance_percentage,
        }
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
