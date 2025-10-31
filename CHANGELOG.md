# Change Log

## unreleased
### Major Changes
- added "total crit damage", "total non-crit damage", "average crit hit" and "average non-crit hit" damage metrics
- added new diagrams:
  - Hits per Second
  - Hits count
  - Heals per Second
  - Heals count

## v1.3.0
### Major Changes
- the ability to upload runs to the OSCR-Server and display them
- the overlay can now be used independent of the auto refresh setting

### Other Changes
- the auto refresh setting can now be set down to 0.1s via the settings slider (this was possible previously but only by editing the value directly)

### Fixes
- fixed that Overlay would not update if the main window was minimized
- fixed damage resistance summary copy not being labeled correctly

## v1.2.1
### Fixes
- fixed Overlay size being incorrect when the monitor scale is any value other than 100% (windows setting)

## v1.2.0
### Major Changes
- added an overlay to the tool
  - can be configured what numbers are displayed
  - can be freely moved with the mouse or docked in the current location (mouse clicks will go through the window)
- added damage out exclusion rules to the settings
  - this can be useful for ignoring things that may cause the combat time to be extended such as warp core explosions from pets
  - or it can be used to remove things from the log that wish to not be included in your DPS numbers
- in the details analysis tabs, you can now freely select multiple rows to be displayed in the diagrams, by holding the CTRL key while selecting table rows

### Fixes
- max one hit tooltips on the damage out table shows the damage source instead of the target
- combat duration % in the summary table show the actual % instead of the duration in time
- fixed damage resistance chart displaying no negative numbers on the y axis
- fixed crash when toggling auto refresh without having a valid log file path

### Other Changes
- file dialogs are now modal
- added the ability to enable/disable auto refresh without opening the settings window
- added more TFO combat names to the default settings
  - Battle of Wolf 359
  - Vault: Ensnared
- added the ability to drag on drop log files into the parser
- added the ability to clear the log without opening the settings window 

### Internal Changes
- updated eframe + egui

## v1.1.0
### Major Changes
- added the kill counts to damage tables
  - by hovering on the a kills table cell a detailed list is displayed, that shows what was killed or what caused the death
- the summary table now also contains separate player and NPC kill counts
- the damage out table can now be expanded one level more to display, to what target damage was dealt
- added more combat names to the default settings
  - Best Served Cold
  - Battle of the Binary Stars
  - Red Alert: Tholian
  - Battle of Korfez

### Other Changes
- added the ability to move rules up and down in the analysis tab of the settings
- the tool version is now displayed in the window title
- fixed a bug that caused the last bar of a bar chart part to be dropped entirely
- a line a DPS graph now has at least the length of 1 second so that it is clearly visible

### Internal Changes
- updated eframe + egui
- added value and name managers to reduce memory usage slightly

## v1.0.0
### Major Changes
- added incoming and outgoing healing tabs with some diagrams

### Other Changes
- fixed a bug that prevented table columns from shrinking in order to fit its content
- added more TFO names to the default settings
- added more clarifications on what the numbers for certain columns mean
- the rows in the summary table are now also selectable for better visualization
- fixed a bug that caused the damage numbers in the diagrams to be doubled when something is selected in the table
- added hits per second numbers
- added hit percentage numbers
- added misses and accuracy numbers
- added damage type infos
- improved status indication
  - it now shows if there was an error when trying to parse the log
  - when hover over the status icon the log file size is displayed
- added the ability to detect additional infos of a combat such as its difficulty (ISE detection has been added as well)
- changed the terminology for "Sub-Source" to "Indirect Source"
- added a little helper window to display all occurred names of a combat to simplify the creation of combat name rules
- removed plasma storm and distributed targeting again from the default settings as these are by far not the only ones with this hull and shield damage split issue
- fixed an issue where for hull ticks, the number for all ticks was displayed
- added an icon to the app
- various small improvements here and there
- disabled log creation in the default settings

### Major Changes
- added the ability to copy a combat summary to the clipboard
- added the ability to save combats
- added damage resistance metrics and a chart

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
