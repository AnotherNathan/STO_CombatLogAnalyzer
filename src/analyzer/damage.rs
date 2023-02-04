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
    pub damage_resistance_percentage: ShieldHullOptionalValues,
    pub damage_resistance: ShieldHullOptionalValues,
}

bitflags! {
    pub struct ValueFlags: u8{
        const NONE = 0;
        const CRITICAL = 1;
        const FLANK = 1 << 1;
        const KILL = 1 << 2;
        const IMMUNE = 1 << 2;
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
                SpecificHit::Shield {
                    damage_prevented_to_hull,
                } => {
                    shield_hits += 1;
                    total_shield_damage += hit.damage;
                    total_damage_prevented_to_hull_by_shields += damage_prevented_to_hull;
                }
                SpecificHit::Hull { base_damage } => {
                    hull_hits += 1;
                    total_hull_damage += hit.damage;
                    total_base_damage += base_damage;
                }
                SpecificHit::ShieldDrain => {
                    shield_hits += 1;
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

        let damage_resistance_percentage = ShieldHullOptionalValues::damage_resistance_percentage(
            &total_damage,
            total_base_damage,
            total_damage_prevented_to_hull_by_shields,
            total_shield_drain,
        );

        let damage_resistance =
            ShieldHullOptionalValues::damage_resistance(&damage_resistance_percentage);

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
            damage_resistance,
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
    const NONE: Self = Self {
        all: None,
        hull: None,
        shield: None,
    };

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

    fn damage_resistance_percentage(
        total_damage: &ShieldHullValues,
        total_base_damage: f64,
        total_damage_prevented_to_hull_by_shields: f64,
        total_shield_drain: f64,
    ) -> Self {
        if total_base_damage == 0.0 {
            return Self::NONE;
        }

        let total_damage_without_drain = total_damage.all - total_shield_drain;

        let all_res = 1.0 - total_damage_without_drain / total_base_damage;
        let all = Some(all_res * 100.0);

        let total_damage_if_there_were_no_shields =
            total_damage.hull + total_damage_prevented_to_hull_by_shields;
        let hull_res = 1.0 - total_damage_if_there_were_no_shields / total_base_damage;

        let hull = Some(hull_res * 100.0);

        let hull_gain = 1.0 - hull_res;

        let total_shield_damage_without_drain = total_damage.shield - total_shield_drain;
        if hull_gain == 0.0 || total_shield_damage_without_drain == 0.0 {
            return Self {
                all,
                hull,
                ..Self::NONE
            };
        }

        let base_damage_to_hull = total_damage.hull / hull_gain;
        let base_damage_to_shield = total_base_damage - base_damage_to_hull;

        let shield_res = if base_damage_to_shield == 0.0 {
            return Self {
                all,
                hull,
                ..Self::NONE
            };
        } else {
            1.0 - total_shield_damage_without_drain / base_damage_to_shield
        };

        let shield = Some(shield_res * 100.0);

        Self { all, shield, hull }
    }

    fn damage_resistance(damage_resistance_percentage: &Self) -> Self {
        Self {
            all: damage_resistance_percentage
                .all
                .map(|a| calc_damage_resistance_point_from_percentage(a)),
            shield: damage_resistance_percentage
                .shield
                .map(|s| calc_damage_resistance_point_from_percentage(s)),
            hull: damage_resistance_percentage
                .hull
                .map(|h| calc_damage_resistance_point_from_percentage(h)),
        }
    }
}

fn calc_damage_resistance_point_from_percentage(resistance_percentage: f64) -> f64 {
    if resistance_percentage >= 0.0 {
        return calc_positive_damage_resistance_point_from_percentage(resistance_percentage);
    }
    calc_negative_damage_resistance_point_from_percentage(resistance_percentage)
}

fn calc_positive_damage_resistance_point_from_percentage(resistance_percentage: f64) -> f64 {
    let g = 1.0 - resistance_percentage / 100.0;

    let _75_2 = 75.0 * 75.0;
    let _150_2 = 150.0 * 150.0;

    let a = g - 0.25;
    let b = 300.0 * g - 0.25 * 300.0;
    let c = _150_2 * g - 0.25 * _150_2 - 3.0 * _75_2;

    let r = (-b + f64::sqrt(b * b - 4.0 * a * c)) / (2.0 * a);
    r
}

fn calc_negative_damage_resistance_point_from_percentage(resistance_percentage: f64) -> f64 {
    let g = 1.0 - resistance_percentage / 100.0;

    let _75_2 = 75.0 * 75.0;
    let _150_2 = 150.0 * 150.0;

    let a = 1.0 - 0.25 * g;
    let b = -300.0 + 0.25 * g * 300.0;
    let c = _150_2 - 0.25 * g * _150_2 - 3.0 * g * _75_2;

    let r = (-b - f64::sqrt(b * b - 4.0 * a * c)) / (2.0 * a);
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calc_positive_damage_resistance() {
        // see drr.png
        assert_positive_damage_resistance(calc_positive_resistance_percentage(0.0));
        assert_positive_damage_resistance(calc_positive_resistance_percentage(5.0));
        assert_positive_damage_resistance(calc_positive_resistance_percentage(10.0));
        assert_positive_damage_resistance(calc_positive_resistance_percentage(40.0));
        assert_positive_damage_resistance(calc_positive_resistance_percentage(200.0));
        assert_positive_damage_resistance(calc_positive_resistance_percentage(600.0));
    }

    #[test]
    fn calc_negative_damage_resistance() {
        // see drr.png
        assert_negative_damage_resistance(calc_negative_resistance_percentage(-0.0));
        assert_negative_damage_resistance(calc_negative_resistance_percentage(-5.0));
        assert_negative_damage_resistance(calc_negative_resistance_percentage(-10.0));
        assert_negative_damage_resistance(calc_negative_resistance_percentage(-40.0));
        assert_negative_damage_resistance(calc_negative_resistance_percentage(-200.0));
        assert_negative_damage_resistance(calc_negative_resistance_percentage(-600.0));
    }

    fn calc_positive_resistance_percentage(resistance_points: f64) -> f64 {
        let a = 75.0 / (150.0 + resistance_points);
        let dr = 3.0 * (0.25 - a * a);
        dr * 100.0
    }

    fn calc_negative_resistance_percentage(resistance_points: f64) -> f64 {
        let a = 75.0 / (150.0 - resistance_points);
        let g = 1.0 / (0.25 + 3.0 * a * a);
        (1.0 - g) * 100.0
    }

    fn assert_positive_damage_resistance(expected_resistance_points: f64) {
        let resistance_percentage = calc_positive_resistance_percentage(expected_resistance_points);
        assert_damage_resistance(resistance_percentage, expected_resistance_points);
    }

    fn assert_negative_damage_resistance(expected_resistance_points: f64) {
        let resistance_percentage = calc_negative_resistance_percentage(expected_resistance_points);
        assert_damage_resistance(resistance_percentage, expected_resistance_points);
    }

    fn assert_damage_resistance(resistance_percentage: f64, expected_resistance_points: f64) {
        let calculated_resistance_points =
            calc_damage_resistance_point_from_percentage(resistance_percentage);
        assert!(
            (calculated_resistance_points - expected_resistance_points).abs() < 1.0e-3,
            "%: {} | calc: {} | expect: {}",
            resistance_percentage,
            calculated_resistance_points,
            expected_resistance_points
        );
    }
}
