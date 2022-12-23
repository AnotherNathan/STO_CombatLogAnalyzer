use std::ops::Deref;
use std::ops::DerefMut;

use chrono::Duration;
use serde::*;

use super::parser::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisSettings {
    pub combatlog_file: String,
    pub combat_separation_time_seconds: f64,
    pub summon_and_pet_grouping_revers_rules: Vec<Rule<MatchRule>>,
    pub custom_group_rules: Vec<Rule<CustomGroupingRule>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rule<T> {
    pub enabled: bool,
    pub rule: T,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MatchRule {
    pub aspect: MatchAspect,
    pub expression: String,
    pub method: MatchMethod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MatchAspect {
    #[default]
    PetOrSummonName,
    DamageName,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MatchMethod {
    #[default]
    Equals,
    StartsWith,
    EndsWith,
    Contains,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CustomGroupingRule {
    pub group_name: String,
    pub match_rule: MatchRule,
}

impl MatchRule {
    pub fn matches(&self, record: &Record) -> bool {
        match self.aspect {
            MatchAspect::PetOrSummonName => match record.sub_source {
                Entity::NonPlayer { name, .. } => self.method.check_match(&self.expression, name),
                _ => false,
            },
            MatchAspect::DamageName => self.method.check_match(&self.expression, record.value_name),
        }
    }
}

impl MatchAspect {
    pub const fn display(self) -> &'static str {
        match self {
            MatchAspect::PetOrSummonName => "pet or summon name",
            MatchAspect::DamageName => "damage name",
        }
    }
}

impl MatchMethod {
    fn check_match(&self, expression: &str, value: &str) -> bool {
        match self {
            MatchMethod::Equals => value == expression,
            MatchMethod::StartsWith => value.starts_with(expression),
            MatchMethod::EndsWith => value.ends_with(expression),
            MatchMethod::Contains => value.contains(expression),
        }
    }

    pub const fn display(self) -> &'static str {
        match self {
            MatchMethod::Equals => "equals",
            MatchMethod::StartsWith => "starts with",
            MatchMethod::EndsWith => "ends with",
            MatchMethod::Contains => "contains",
        }
    }
}

impl Default for AnalysisSettings {
    fn default() -> Self {
        Self {
            combatlog_file: Default::default(),
            combat_separation_time_seconds: 1.5 * 60.0,
            summon_and_pet_grouping_revers_rules: Default::default(),
            custom_group_rules: Default::default(),
        }
    }
}

impl<T: Default> Default for Rule<T> {
    fn default() -> Self {
        Self {
            enabled: true,
            rule: Default::default(),
        }
    }
}

impl<T> Deref for Rule<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.rule
    }
}

impl<T> DerefMut for Rule<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.rule
    }
}
