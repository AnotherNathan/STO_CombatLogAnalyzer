use educe::Educe;

use super::*;

#[derive(Clone, Copy, Debug)]
pub struct BaseHealTick {
    pub amount: f64,
    pub flags: ValueFlags,
    pub specific: SpecificHealTick,
}

#[derive(Clone, Copy, Debug, Educe)]
#[educe(Deref, DerefMut)]
pub struct HealTick {
    #[educe(Deref, DerefMut)]
    pub tick: BaseHealTick,
    pub time_millis: u32, // offset to start of combat
}

#[derive(Clone, Copy, Debug)]
pub enum SpecificHealTick {
    Shield,
    Hull,
}

#[derive(Clone, Default, Debug)]
pub struct HealMetrics {
    pub ticks: ShieldAndHullCounts,
    pub ticks_per_second: ShieldHullValues,
    pub total_heal: ShieldHullValues,
    pub hps: ShieldHullValues,
    pub average_heal: ShieldHullOptionalValues,
    pub critical_percentage: Option<f64>,
}

impl BaseHealTick {
    pub fn shield(amount: f64, flags: ValueFlags) -> Self {
        Self {
            amount: amount.abs(),
            flags,
            specific: SpecificHealTick::Shield,
        }
    }

    pub fn hull(amount: f64, flags: ValueFlags) -> Self {
        Self {
            amount: amount.abs(),
            flags,
            specific: SpecificHealTick::Hull,
        }
    }

    pub fn to_tick(self, time_millis: u32) -> HealTick {
        HealTick {
            tick: self,
            time_millis,
        }
    }
}

impl HealMetrics {
    pub fn calculate(ticks: &[HealTick], active_duration: f64) -> Self {
        let mut total_shield_heal = 0.0;
        let mut total_hull_heal = 0.0;
        let mut shield_ticks = 0;
        let mut hull_ticks = 0;
        let mut crits = 0;

        for tick in ticks.iter() {
            match tick.specific {
                SpecificHealTick::Shield => {
                    shield_ticks += 1;
                    total_shield_heal += tick.amount;
                }
                SpecificHealTick::Hull => {
                    hull_ticks += 1;
                    total_hull_heal += tick.amount;
                }
            }

            if tick.flags.contains(ValueFlags::CRITICAL) {
                crits += 1;
            }
        }

        let total_heal = ShieldHullValues {
            all: total_hull_heal + total_shield_heal,
            hull: total_hull_heal,
            shield: total_shield_heal,
        };

        let ticks = ShieldAndHullCounts {
            all: shield_ticks + hull_ticks,
            hull: hull_ticks,
            shield: shield_ticks,
        };
        let ticks_per_second = ShieldHullValues::per_seconds(&ticks.to_values(), active_duration);

        let hps = ShieldHullValues::per_seconds(&total_heal, active_duration);

        let average_heal =
            ShieldHullOptionalValues::average(&total_heal, shield_ticks, hull_ticks, ticks.all);

        let critical_percentage = percentage_u64(crits, hull_ticks);

        Self {
            ticks,
            ticks_per_second,
            total_heal,
            hps,
            average_heal,
            critical_percentage,
        }
    }
}
