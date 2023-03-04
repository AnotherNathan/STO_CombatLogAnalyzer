use std::{
    borrow::{Borrow, BorrowMut},
    path::Path,
};

use serde::*;

use super::parser::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisSettings {
    pub combatlog_file: String,
    pub combat_separation_time_seconds: f64,
    pub summon_and_pet_grouping_revers_rules: Vec<MatchRule>,
    pub custom_group_rules: Vec<RulesGroup>,
    pub combat_name_rules: Vec<CombatNameRule>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CombatNameRule {
    pub name_rule: RulesGroup,
    pub additional_info_rules: Vec<RulesGroup>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchRule {
    pub aspect: MatchAspect,
    pub expression: String,
    pub method: MatchMethod,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MatchAspect {
    SourceOrTargetName,
    SourceOrTargetUniqueName,
    SubSourceName,
    SubUniqueSourceName,
    #[default]
    DamageOrHealName,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RulesGroup {
    pub name: String,
    pub rules: Vec<MatchRule>,
    pub enabled: bool,
}

impl AnalysisSettings {
    pub fn combatlog_file(&self) -> &Path {
        Path::new(&self.combatlog_file)
    }
}

impl RulesGroup {
    pub fn matches_record(&self, record: &Record) -> bool {
        if !self.enabled {
            return false;
        }

        self.rules.iter().any(|r| r.matches_record(record))
    }

    pub fn matches_source_or_target_names<'a>(
        &self,
        mut names: impl Iterator<Item = &'a String>,
    ) -> bool {
        if !self.enabled {
            return false;
        }

        names.any(|n| {
            self.rules
                .iter()
                .any(|r| r.matches_source_or_target_name(n))
        })
    }

    pub fn matches_source_or_target_unique_names<'a>(
        &self,
        mut names: impl Iterator<Item = &'a String>,
    ) -> bool {
        if !self.enabled {
            return false;
        }

        names.any(|n| {
            self.rules
                .iter()
                .any(|r| r.matches_source_or_target_unique_name(n))
        })
    }

    pub fn matches_sub_source_names<'a>(
        &self,
        mut names: impl Iterator<Item = &'a String>,
    ) -> bool {
        if !self.enabled {
            return false;
        }

        names.any(|n| self.rules.iter().any(|r| r.matches_sub_source_name(n)))
    }

    pub fn matches_sub_source_unique_names<'a>(
        &self,
        mut names: impl Iterator<Item = &'a String>,
    ) -> bool {
        if !self.enabled {
            return false;
        }

        names.any(|n| {
            self.rules
                .iter()
                .any(|r| r.matches_sub_source_unique_name(n))
        })
    }

    pub fn matches_damage_or_heal_names<'a>(
        &self,
        mut names: impl Iterator<Item = &'a String>,
    ) -> bool {
        if !self.enabled {
            return false;
        }

        names.any(|n| self.rules.iter().any(|r| r.matches_damage_or_heal_name(n)))
    }
}

impl MatchRule {
    pub fn matches_record(&self, record: &Record) -> bool {
        if !self.enabled {
            return false;
        }

        match self.aspect {
            MatchAspect::SourceOrTargetName => {
                self.method
                    .check_match_or_false(&self.expression, record.source.name())
                    || self
                        .method
                        .check_match_or_false(&self.expression, record.target.name())
            }
            MatchAspect::SourceOrTargetUniqueName => {
                self.method
                    .check_match_or_false(&self.expression, record.source.unique_name())
                    || self
                        .method
                        .check_match_or_false(&self.expression, record.target.unique_name())
            }
            MatchAspect::SubSourceName => self
                .method
                .check_match_or_false(&self.expression, record.sub_source.name()),
            MatchAspect::SubUniqueSourceName => self
                .method
                .check_match_or_false(&self.expression, record.sub_source.unique_name()),
            MatchAspect::DamageOrHealName => {
                self.method.check_match(&self.expression, record.value_name)
            }
        }
    }

    pub fn matches_source_or_target_name(&self, name: &str) -> bool {
        if !self.enabled || self.aspect != MatchAspect::SourceOrTargetName {
            return false;
        }

        self.method.check_match(&self.expression, name)
    }

    pub fn matches_source_or_target_unique_name(&self, name: &str) -> bool {
        if !self.enabled || self.aspect != MatchAspect::SourceOrTargetUniqueName {
            return false;
        }

        self.method.check_match(&self.expression, name)
    }

    pub fn matches_sub_source_name(&self, name: &str) -> bool {
        if !self.enabled || self.aspect != MatchAspect::SubSourceName {
            return false;
        }

        self.method.check_match(&self.expression, name)
    }

    pub fn matches_sub_source_unique_name(&self, name: &str) -> bool {
        if !self.enabled || self.aspect != MatchAspect::SubUniqueSourceName {
            return false;
        }

        self.method.check_match(&self.expression, name)
    }

    pub fn matches_damage_or_heal_name(&self, name: &str) -> bool {
        if !self.enabled || self.aspect != MatchAspect::DamageOrHealName {
            return false;
        }

        self.method.check_match(&self.expression, name)
    }
}

impl MatchAspect {
    pub const fn display(self) -> &'static str {
        match self {
            MatchAspect::SourceOrTargetName => "Source or Target Name",
            MatchAspect::SourceOrTargetUniqueName => "Source or Target Unique Name",
            MatchAspect::SubSourceName => "Sub-Source Name (e.g. a pet or summon)",
            MatchAspect::DamageOrHealName => "Damage / Heal Name",
            MatchAspect::SubUniqueSourceName => "Sub-Source Unique Name (e.g. a pet or summon)",
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

    fn check_match_or_false(&self, expression: &str, value: Option<&str>) -> bool {
        match value {
            Some(value) => self.check_match(expression, value),
            None => false,
        }
    }

    pub const fn display(self) -> &'static str {
        match self {
            MatchMethod::Equals => "Equals",
            MatchMethod::StartsWith => "Starts with",
            MatchMethod::EndsWith => "Ends with",
            MatchMethod::Contains => "Contains",
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
            combat_name_rules: Default::default(),
        }
    }
}

impl Default for MatchRule {
    fn default() -> Self {
        Self {
            enabled: true,
            aspect: Default::default(),
            expression: Default::default(),
            method: Default::default(),
        }
    }
}

impl Default for RulesGroup {
    fn default() -> Self {
        Self {
            name: Default::default(),
            rules: Default::default(),
            enabled: true,
        }
    }
}

impl Borrow<RulesGroup> for CombatNameRule {
    fn borrow(&self) -> &RulesGroup {
        &self.name_rule
    }
}

impl BorrowMut<RulesGroup> for CombatNameRule {
    fn borrow_mut(&mut self) -> &mut RulesGroup {
        &mut self.name_rule
    }
}
