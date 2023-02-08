use std::fmt::Write;

use bitflags::bitflags;
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

#[derive(Clone, Copy, Debug, Default)]
pub struct ShieldHullValues {
    pub all: f64,
    pub shield: f64,
    pub hull: f64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ShieldHullOptionalValues {
    pub all: Option<f64>,
    pub shield: Option<f64>,
    pub hull: Option<f64>,
}

#[derive(Clone, Debug, Default)]
pub struct DamageMetrics {
    pub shield_hits: u64,
    pub hull_hits: u64,
    pub hits: u64,
    pub total_damage: ShieldHullValues,
    pub total_damage_prevented_to_hull_by_shields: f64,
    pub total_base_damage: f64,
    pub dps: ShieldHullValues,
    pub average_hit: ShieldHullOptionalValues,
    pub critical_chance: f64,
    pub flanking: f64,
    pub damage_resistance_percentage: Option<f64>,
}

bitflags! {
    pub struct ValueFlags: u8{
        const NONE = 0;
        const CRITICAL = 1;
        const FLANK = 1 << 1;
        const KILL = 1 << 2;
        const IMMUNE = 1 << 3;
    }
}

impl Default for ValueFlags {
    fn default() -> Self {
        Self::NONE
    }
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

impl ValueFlags {
    pub fn parse(input: &str) -> Self {
        let mut flags = ValueFlags::NONE;
        for flag in input.split('|') {
            flags |= match flag {
                "Critical" => ValueFlags::CRITICAL,
                "Flank" => ValueFlags::FLANK,
                "Kill" => ValueFlags::KILL,
                "Immune" => ValueFlags::IMMUNE,
                _ => ValueFlags::NONE,
            };
        }

        flags
    }
}

impl DamageMetrics {
    pub fn calculate(hits: &[Hit], combat_duration: f64) -> Self {
        let mut total_shield_damage = 0.0;
        let mut total_hull_damage = 0.0;
        let mut shield_hits = 0;
        let mut hull_hits = 0;
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
        }

        let total_damage = total_hull_damage + total_shield_damage;
        let total_damage = ShieldHullValues {
            all: total_damage,
            hull: total_hull_damage,
            shield: total_shield_damage,
        };

        let critical_chance = if hull_hits == 0 {
            0.0
        } else {
            crits as f64 / hull_hits as f64
        };
        let critical_chance = critical_chance * 100.0;

        let flanking = if hull_hits == 0 {
            0.0
        } else {
            flanks as f64 / hull_hits as f64
        };
        let flanking = flanking * 100.0;

        let hits = shield_hits + hull_hits;
        let dps = ShieldHullValues::dps(&total_damage, combat_duration);
        let average_hit =
            ShieldHullOptionalValues::average_hit(&total_damage, shield_hits, hull_hits, hits);

        let damage_resistance_percentage =
            damage_resistance_percentage(&total_damage, total_base_damage, total_shield_drain);

        Self {
            shield_hits,
            hull_hits,
            hits,
            total_damage,
            total_damage_prevented_to_hull_by_shields,
            total_base_damage,
            dps,
            average_hit,
            critical_chance,
            flanking,
            damage_resistance_percentage,
        }
    }
}

impl ShieldHullValues {
    fn dps(total_damage: &Self, combat_duration: f64) -> Self {
        // avoid absurd high numbers by having a combat duration of at least 1 sec
        Self {
            all: total_damage.all / combat_duration.max(1.0),
            shield: total_damage.shield / combat_duration.max(1.0),
            hull: total_damage.hull / combat_duration.max(1.0),
        }
    }
}

impl ShieldHullOptionalValues {
    fn average_hit(
        total_damage: &ShieldHullValues,
        shield_hits: u64,
        hull_hits: u64,
        hits: u64,
    ) -> Self {
        Self {
            all: if hits == 0 {
                None
            } else {
                Some(total_damage.all / hits as f64)
            },
            shield: if shield_hits == 0 {
                None
            } else {
                Some(total_damage.shield / shield_hits as f64)
            },
            hull: if hull_hits == 0 {
                None
            } else {
                Some(total_damage.hull / hull_hits as f64)
            },
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
