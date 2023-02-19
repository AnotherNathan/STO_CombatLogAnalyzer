# Change Log

## v0.3.0
### Major Changes
- added the ability to copy a combat summary to the clipboard
- added the ability to save combats
- added damage resistance metrics and a chart

### Other Changes
- added the ability to select the entire row in the damage tables instead of only the entry name
- fixed a bug that would cause the DPS graph to have incorrect spikes at the beginning of a line
- added base damage and DPS metrics
- fixed not being able to parse logs with non player characters (e.g. Boffs)
- tweaked light dark theme
- fixed a crash when entering 0 into the time slice text edit of a chart
- added distributed targeting and plasma storm to default sub source reversal rules
  - these are really weird since shield and hull damage are reported entirely separately in the log, in order to combat their weirdness they were added to the default settings
- added some more TFO names to the default settings
- renamed the settings file to STO_CombatLogAnalyzer_Settings.json to make it more clear that this file belongs to the parser
- integrated the default settings into the exe for people who copy around only the exe, so that they do not loose the default TFO name detection
- some small tweaks to UI here and there

### Internal Changes
- updated eframe + egui
- switched all tables to custom table and removed egui_extras
  - this fixes any sizing bugs that were present in egui_extras
  - and this allows now for supporting the selection of table rows
- cargo update

## v0.2.1 (29.01.2023)
### Fixes
- fixed some abilities (e.g. concentrate firepower) not being counted as outgoing damage

## v0.2.0 (28.01.2023)
### Major changes
- added a new Theme (Light Dark)
- added Summary tab
- added DPS, Damage and Summary diagrams

### Other Changes
- do not count direct self damage (e.g. from fly her apart) as output damage
- added average shield and hull hit metrics via tooltip
- restrict minimum window size

### Fixes
- fixed some incoming damage sources (e.g. CF or some DOTs) not being counted


## v0.1.0 (05.01.2023)
First work in progress release.

Contains basic damage in and out tables.
