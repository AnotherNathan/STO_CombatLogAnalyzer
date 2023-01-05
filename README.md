# STO_CombatLogAnalyzer
Tool to parse and analyze the combat log file from Star Trek Online.

---
## Getting started
1. Download the application from the Releases page.

2. Make sure you turned off log rotation (see https://www.sto-league.com/how-to-disable-automatically-rotated-log-files/).

3. Go into the game and type "/Combatlog 1" into the chat window.
   
4. Fight something.

5. Start STO_CombatLogAnalyzer, open the Settings and enter the path to the combatlog file of the game located at "\<path to STO installation\>\Star Trek Online\Live\logs\GameClient\combatlog.log."
Click Ok at the bottom of the settings window.

6. Click the refresh button.

---
## Building the tool from Source
Install the rust tool chain from https://www.rust-lang.org/.

And the build with

```
C:\path\to\STO_CombatLogAnalyzer> cargo build --release
```

And that is it.

