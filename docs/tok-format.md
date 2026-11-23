# Mission scripts: the `.tok` files

The original's missions are scripts, kept tokenised in `data\MISSIONS0001.RFD` of
`Game1.cab`: one `.tok` file a mission, such as `ATStart.tok` for the Atreides' first battle.
The archive's `missionlist.ini` lists the scripts by their source names (`ATStart.txt`), and
`header.tok` has a record of 228 bytes for each. The source text is not on the disc. What
follows was worked out from the files themselves, `ATStart.tok` above all; where it is a
guess it says so.

## The file

| Bytes | What |
| --- | --- |
| 0–3 | The size of the rest of the file after these eight bytes, little-endian. |
| 4–7 | How many lines the script has. |
| 8… | The lines, each ended by a zero byte. An empty line is just its zero. |

A line is one statement of the source, with its comments and spacing gone. Blank lines and
comment lines are kept as empty lines, so line numbers still match the source.

## Tokens

A line is made of plain characters and tokens. A token starts with a byte from `0x80` up;
every byte after it in the token also has its top bit set, and carries seven bits of a
number, lowest first.

| First byte | Bytes after | What it is |
| --- | --- | --- |
| `0x80` | 2 | A function or a keyword, by number. Below 128 are functions (`F0`…), from 128 keywords and operators (`K128`…). |
| `0x81` | 1 | A variable, by number, in the order the script declares them. |
| `0x82` | 2 | A kind of unit or building. |
| `0x83` | 2 | A line of speech. |

Everything else is plain text as written: digits, commas, brackets. Numbers are decimal
digits; a negative one is written with the keyword `K143`, as `K143(4)` for −4.

So the bytes `81 86 80 B4 81 80 85 80 28 31 29` read `v6 K180 F5(1)`: the variable 6 is set
to what function 5 gives for 1.

### Kinds of unit and building (`0x82`)

The number is 111 more than the kind's place among the entries of `Rules.txt`'s
`[BuildingTypes]` and `[UnitTypes]` lists, taken in the order they come in the file (the
lists come several times, each adding to its own). 127 is `ATWall`, 138 `ATConYard`, 246
`HKLightInf`, 270 `HKBuzzsaw`, 300 `MCV`. What the first 111 numbers are is not known; the
houses, bullets and warheads together make as many.

### Lines of speech (`0x83`)

The number is 502 more than the line's place among the entries of the `MissionMessages`
sections of the game's table of what is spoken, `E_Output_Pickup.txt` in
`data\STRINGS0001.RFD`: only those sections count, each from its `MissionMessages` line to its
`[END]`, and in them only lines with an id, not blank lines or comments. The briefings and
debriefings before each house's messages do not count, which is why counting every line of
the file only fitted the Atreides' first mission. 642 is `ATstart1`, the Mentat's welcome, 646
`ATtutorial1`, 901 `HKstart1` and 1098 `ORstart1`. The id at the start of that line names its
recording in `data\Sounds\sounds.txt` (`ATstart1` is `KK-M680E`), and the recording is in a
dialogue bag under that name without the language's letter: `KK-M680` in the Atreides disc's
`DIALOG.BAG`. Numbers below 502 name something else, not known yet.

## Keywords and functions

Their names are in `GAME.EXE` (see [Names, from GAME.EXE](#names-from-gameexe) below). The
table here is what their use in the scripts showed before the names were found; where a
reading and the name disagree, the name is the original's (TASK-140). A map's entrances are
the points of its area `Entrance`, at its edges, in the order of its file:

| Token | Seems to be | Seen as |
| --- | --- | --- |
| `K162`, `K163`, `K164` | Declarations of variables, of three types | `K162(v0)` at the top |
| `K165` … `K167` | `if (…)` and its end | `K165(F0() K168 550)` … `K167` |
| `K168` | `==` | |
| `K169` | `!=` | `F17(v27) K169 F14()`: a building not the player's |
| `K170`, `K171`, `K172`, `K173` | `>=`, `<=`, `>`, `<` | `v32 K171 25`: the tutorial's step once it is 25; `F0() K173 15000`: before update 15000 |
| `K174`, `K175` | `&&`, `\|\|` | |
| `K176`, `K177` | `FALSE`, `TRUE` | |
| `K178` | `+` | `F0() K178 260` |
| `K180` | `=` | |
| `K143` | A negative number | `K143(10)` |
| `K139` | Puts a new object on the map and gives it: side, kind, place, tiles across, tiles down, facing | `K139(v0,<ATConYard>,v13,0,0,1)` |
| `K150` | Sets something of a side | `K150(v2,1)` |
| `K131`, `K132` | A scene begins and ends: the camera is the script's, not scrolled | around the first battles' opening |
| `K128`, `K129`, `K130`, `K133`, `K156` | Not known | |
| `F0()` | The battle's time, in updates | compared with 49, 50, 100, 150 … |
| `F1(n)` | A whole number at random from 0 to `n - 1` (the original's code) | `v7 K180 F1(2000) K178 v7` |
| `F14()` | The player's side | `v0 K180 F14()` |
| `F15()` | The co-op campaign's second human side; the player's outside one (the original's code) | |
| `K140()` | Side 15, the last of the original's sixteen: nobody's things (the original's code) | `F21(K140(),…,v20)` |
| `F18()` | A new side | four of them, the enemy and others |
| `F3()` | The enemy's main base: the map's point `Primary` | |
| `F4(side)` | A side's base: the player's at the map's point `Default` | `v13 K180 F4(v0)` |
| `F6(side)`, `F7(side)` | The map's entrance nearest a side's base, and the one farthest from that (the original's code) | `F21(v4,…,F6(F16()))` |
| `F5(n)` | The map's point `Script` and the number | `F5(1)` is `Script1` |
| `F2()` | Whether the game is the co-op campaign: false here, as the original has it outside one | before the second player's gift |
| `F21(side, kind, place)` | Brings a unit of that kind in for the side at a place | reinforcements |
| `F23(object)` | Whether an object is destroyed | all three of the enemy's main buildings ends the mission |
| `F35(side)` | How many units a side has left | `F35(v2) K168 0` |
| `F45(line)` | Plays a line of speech | |
| `F60(a, b)` | Makes side `a` an enemy of `b` | given both ways |
| `F62(side, n)` | Gives a side money, on top of what it has (the original's code) | `F62(v0,10000)` |
| `F19(side)`, `F20(side)` | A side's money; its spice is the same figure (the original's code) | |
| `K152(side, n)` | Sets a side's money, only at the battle's first update (the original's code) | |
| `F83(truth)` | Ends the battle, won if true and lost if false | `F83(TRUE)` once the enemy's palace and yards are gone |
| `F37(side, kind)` | How many things of a kind a side has; for `F16()`, the map's own too | `F37(F16(),<HKPalace>) K168 0` |
| `F8()` | Where reinforcements come in; taken as the player's base | `F21(F14(),<FRFremen>,F8())` |
| `F9(n)`, `F10(place)`, `F11(place)` | The map's entrance `n` (the first when it has none of that number), the one nearest a place and the one farthest from it (the original's code) | `F11(F12(F16()))` |
| `F12(side)` | Where a side's units arrive; taken as its base | `F21(F14(),<ATMongoose>,F12(F14()))` |
| `F13(thing)` | Where a thing is | `F97(F13(v3),12)` |
| `K158(x, y)` | The place of the map's square in column `x`, row `y`: its corner, `x` × 32 and `y` × 32 (the original's code, which works out the middle too but gives the corner) | `F21(v5,…,K158(141,132))` |
| `F16()` | A side that is neither the player's nor one of the script's | `F59(v1,F16())` |
| `F17(thing)` | Whose a thing is, as a side | `F17(v3) K168 F14()`: the outpost captured |
| `F22(thing)` | Whether a thing still stands | |
| `F27(thing, side)` | Whether a side has hurt a thing; taken as: it is hurt, and the side is its enemy | `F27(v3,F14())` |
| `F32(a, b)` | Whether side `a`'s units are in sight of `b`'s | prisoners reached |
| `F44(thing)` | Whether a thing is destroyed | |
| `F54(thing, n)` | Sets a thing's health | a camp's windtraps at 90 to 320 |
| `F57(thing, kind, side)` | Turns a thing into a kind, for a side | prisoners freed as Fremen |
| `F59(a, b)` | Makes side `a` a friend of `b` | given both ways |
| `F24(thing, side)` | Whether a side has a unit at a thing; taken as within three squares of its edge | `F24(v61,F14())`: the tutorial's beacon reached |
| `F29(thing)`, `F30(thing)` | A thing's health and the most it has | `F29(v0) K168 F30(v0)`: the tutorial's windtrap mended |
| `F36(side)` | How many buildings a side has left | with `F35`, the tutorial's enemy base gone |
| `F38(side)` | Whether a side's units have all got where they were sent | after `F71` |
| `F42(a, b)` | Taken as: whether `b` has lost any of what the script gave it, to `a` | the tutorial's wave fought, not only met by the worm |
| `F64(side)` | A side's units go after the player's, and keep at it | the wave the worm left |
| `F71(side, place)` | Sends a side's units to a place | |
| `F82(place)` | A worm comes up at a place; not done yet, as there are no worms | `F82(F12(v43))` |
| `F84()` | The tutorial is over and done; taken as won | 27.5 s after its last line |
| `K135(thing)` | Takes a thing off the map | the tutorial's walls opened for the wave |
| `K144(n)` | A step of the tutorial, as the number; what else it does is not known | `v32 K180 K144(11)` |
| `F85()` | The battle is lost (the original's code) | |
| `F63`, `F96` | Not known; the tutorial runs without them | `F96(v36)` |
| `F91(a, b)` | Hands all of side `a`'s units over to side `b` | `F91(v18,F14())`: the Atreides' Heighliner's Mongooses, once in sight |
| `F93(side)`, `F94()` | Destroys everything of a side, and of every side (the original's code: no health left) | `F94()` as the Heighliner's time runs out |
| `F97(place, n)` | Opens up the fog `n` squares round a place, seen from then on | `F97(v35,20)` as a beacon goes down |
| `F98(place, n)`, `F99()` | Covers the map again `n` squares round a place, to be seen anew; opens up the whole map (the original's code: the human sides' shroud) | `F99()` in the start scripts |
| `F107(thing)`, `F108()` | The camera follows a thing, or what carries it, and stops following | `F107(v1)`: the tutorial's harvester, at its start |
| `F100(place)`, `F114(place)` | The camera looks at a place; the second in the midst of the battle | `F100(v13)`: the player's base |
| `F101(place, n)`, `F115(place, n)` | The camera glides to a place over `n` updates | `F101(v16,50)`: to the enemy's base |
| `F105(degrees, way)` | The camera circles where it looks, so many degrees an update, `1` one way and `2` the other | `F105(2,1)`: once round in the 180 updates before `F113` |
| `F106()` | The camera stops circling | |
| `F112()` | The camera's view is kept | before it glides away |
| `F113(n)` | The camera stops circling and glides back to the view kept over `n` updates | `F113(60)` |

## Names, from GAME.EXE

`GAME.EXE` keeps the name of every function and keyword in a table at file offset `0x1fe0b0`
to `0x2021bc`: 181 records of `0x5c` bytes, each starting with its name, its place in the table
being its token number. The script is run by one switch (`cmp ecx,0xb1` at `0xf0011`, then
`jmp [4*ecx+0x4f4a00]`), whose jump table at `0xf4a00` has 178 entries; the *Case* column is
where each one's code starts in the file and how long it is. The comparisons, `&&`, `||`,
`if`, `else` and `endif` are the parser's, and `K146`, `K147` and `K149` go to the
switch's default, which does nothing. *Uses* counts each over the 228 scripts of
`MISSIONS0001.RFD`. Offsets are into the file; the address in memory is `0x400000` more.

| Token | Name | Case (offset, bytes) | Uses |
| --- | --- | --- | --- |
| `F0` | `ModelTick` | 0xf0169, 0x19 | 3378 |
| `F1` | `Random` | 0xf2288, 0x21 | 55 |
| `F2` | `Multiplayer` | 0xf0182, 0x27 | 5 |
| `F3` | `GetUnusedBasePoint` | 0xf049b, 0x2f | 128 |
| `F4` | `GetSideBasePoint` | 0xf04ca, 0xbd | 95 |
| `F5` | `GetScriptPoint` | 0xf0820, 0xc1 | 244 |
| `F6` | `GetEntrancePoint` | 0xf0587, 0x62 | 1794 |
| `F7` | `GetExitPoint` | 0xf05e9, 0xd0 | 0 |
| `F8` | `GetNeutralEntrancePoint` | 0xf06b9, 0x2f | 2025 |
| `F9` | `GetEntrancePointByIndex` | 0xf06e8, 0x8e | 0 |
| `F10` | `GetEntranceNearToPos` | 0xf0776, 0x55 | 429 |
| `F11` | `GetEntranceFarFromPos` | 0xf07cb, 0x55 | 159 |
| `F12` | `GetSidePosition` | 0xf20cb, 0xd9 | 1236 |
| `F13` | `GetObjectPosition` | 0xf1308, 0x73 | 284 |
| `F14` | `GetPlayerSide` | 0xf0d5b, 0x3a | 4604 |
| `F15` | `GetSecondPlayerSide` | 0xf0d95, 0xed | 23 |
| `F16` | `GetEnemySide` | 0xf0e82, 0xb6 | 1527 |
| `F17` | `GetObjectSide` | 0xf11f8, 0x110 | 24 |
| `F18` | `CreateSide` | 0xf1152, 0x73 | 595 |
| `F19` | `GetSideCash` | 0xf1f6c, 0xa1 | 2 |
| `F20` | `GetSideSpice` | 0xf200d, 0xbe | 4 |
| `F21` | `NewObject` | 0xf03cc, 0xcf | 8939 |
| `F22` | `ObjectValid` | 0xf0d31, 0x2a | 210 |
| `F23` | `ObjectDestroyed` | 0xf08e1, 0x29 | 69 |
| `F24` | `ObjectNearToSide` | 0xf183b, 0xe5 | 89 |
| `F25` | `ObjectNearToSideBase` | 0xf1920, 0xe5 | 31 |
| `F26` | `ObjectNearToObject` | 0xf1a05, 0x107 | 0 |
| `F27` | `ObjectVisibleToSide` | 0xf090a, 0xec | 17 |
| `F28` | `ObjectTypeVisibleToSide` | 0xf09f6, 0xd5 | 3 |
| `F29` | `ObjectGetHealth` | 0xf0acb, 0x2b | 56 |
| `F30` | `ObjectMaxHealth` | 0xf0af6, 0x3c | 1 |
| `F31` | `SideVisibleToSide` | 0xf0b32, 0x1ff | 61 |
| `F32` | `SideNearToSide` | 0xf1b0c, 0x195 | 89 |
| `F33` | `SideNearToSideBase` | 0xf1ca1, 0x205 | 40 |
| `F34` | `SideNearToPoint` | 0xf1ea6, 0xc6 | 91 |
| `F35` | `SideUnitCount` | 0xf2c21, 0xd4 | 774 |
| `F36` | `SideBuildingCount` | 0xf2cf5, 0xf2 | 44 |
| `F37` | `SideObjectCount` | 0xf2de7, 0xa9 | 77 |
| `F38` | `SideAIDone` | 0xf2922, 0x92 | 61 |
| `F39` | `EventObjectDelivered` | 0xf2eda, 0x92 | 2 |
| `F40` | `EventObjectConstructed` | 0xf2f6c, 0xb5 | 27 |
| `F41` | `EventObjectTypeConstructed` | 0xf3021, 0xbe | 267 |
| `F42` | `EventSideAttacksSide` | 0xf30f3, 0x57 | 203 |
| `F43` | `EventObjectAttacksSide` | 0xf314a, 0x47 | 14 |
| `F44` | `EventObjectDestroyed` | 0xf30df, 0x14 | 143 |
| `F45` | `Message` | 0xf01a9, 0x24 | 1715 |
| `F46` | `GiftingMessage` | 0xf01cd, 0x33 | 3 |
| `F47` | `TimerMessage` | 0xf0200, 0x99 | 20 |
| `F48` | `TimerMessageRemove` | 0xf0299, 0x26 | 9 |
| `F49` | `CarryAllDelivery` | 0xf0f38, 0x59 | 45 |
| `F50` | `Delivery` | 0xf0f91, 0x67 | 24 |
| `F51` | `StarportDelivery` | 0xf0ff8, 0xf5 | 0 |
| `F52` | `BuildObject` | 0xf10ed, 0x65 | 60 |
| `F53` | `ObjectChangeSide` | 0xf11c5, 0x33 | 195 |
| `F54` | `ObjectSetHealth` | 0xf31df, 0x2c | 225 |
| `F55` | `ObjectInfect` | 0xf320b, 0x3d | 272 |
| `F56` | `ObjectDetonate` | 0xf3248, 0x65 | 49 |
| `F57` | `ObjectChange` | 0xf32ad, 0x53 | 34 |
| `F58` | `ObjectToolTip` | 0xf3300, 0x2f | 273 |
| `F59` | `SideFriendTo` | 0xf137b, 0x197 | 298 |
| `F60` | `SideEnemyTo` | 0xf1512, 0x19a | 1404 |
| `F61` | `SideNeutralTo` | 0xf16ac, 0x18f | 65 |
| `F62` | `AddSideCash` | 0xf21a4, 0xe4 | 312 |
| `F63` | `SideAIControl` | 0xf22a9, 0x4f | 158 |
| `F64` | `SideAIAggressive` | 0xf22f8, 0x62 | 503 |
| `F65` | `SideAIAggressiveTowards` | 0xf235a, 0x110 | 125 |
| `F66` | `SideAIBehaviourAggressive` | 0xf246a, 0x52 | 12 |
| `F67` | `SideAIBehaviourRetreat` | 0xf24bc, 0x52 | 5 |
| `F68` | `SideAIBehaviourNormal` | 0xf250e, 0x52 | 2 |
| `F69` | `SideAIEncounterIgnore` | 0xf2560, 0x84 | 42 |
| `F70` | `SideAIEncounterAttack` | 0xf25e4, 0x8f | 60 |
| `F71` | `SideAIMove` | 0xf2673, 0xdc | 249 |
| `F72` | `SideAIStop` | 0xf274f, 0xcd | 41 |
| `F73` | `SideAIAttackObject` | 0xf281c, 0x83 | 18 |
| `F74` | `SideAIGuardObject` | 0xf289f, 0x83 | 42 |
| `F75` | `SideAIExitMap` | 0xf29b4, 0x62 | 141 |
| `F76` | `SideAIEnterBuilding` | 0xf2a16, 0x83 | 6 |
| `F77` | `SideAIBehaviourDefensive` | 0xf3f50, 0x6a | 2 |
| `F78` | `SideAIHeadlessChicken` | 0xf3fba, 0x68 | 31 |
| `F79` | `SideAIShuffle` | 0xf4022, 0x68 | 3 |
| `F80` | `SideAttractsWorms` | 0xf2a99, 0xa9 | 1 |
| `F81` | `SideRepelsWorms` | 0xf2b42, 0xc0 | 0 |
| `F82` | `ForceWormStrike` | 0xf2c02, 0x1f | 8 |
| `F83` | `MissionOutcome` | 0xf3191, 0xf | 264 |
| `F84` | `EndGameWin` | 0xf31a0, 0x29 | 21 |
| `F85` | `EndGameLose` | 0xf31c9, 0x16 | 32 |
| `F86` | `NewCrateUnit` | 0xf332f, 0x58 | 12 |
| `F87` | `NewCrateBomb` | 0xf3387, 0x6e | 0 |
| `F88` | `NewCrateStealth` | 0xf3387, 0x6e | 0 |
| `F89` | `NewCrateCash` | 0xf3387, 0x6e | 41 |
| `F90` | `NewCrateShroud` | 0xf3387, 0x6e | 0 |
| `F91` | `SideChangeSide` | 0xf2e90, 0x17 | 72 |
| `F92` | `SetReinforcements` | 0xf2ea7, 0x21 | 21 |
| `F93` | `SideNuke` | 0xf33f5, 0x74 | 0 |
| `F94` | `SideNukeAll` | 0xf3469, 0x30 | 1 |
| `F95` | `RadarEnabled` | 0xf02bf, 0xe6 | 3 |
| `F96` | `RadarAlert` | 0xf03a5, 0x27 | 619 |
| `F97` | `RemoveShroud` | 0xf3499, 0x3b | 461 |
| `F98` | `ReplaceShroud` | 0xf34d4, 0x28 | 7 |
| `F99` | `RemoveMapShroud` | 0xf34fc, 0xa | 5 |
| `F100` | `CameraLookAtPoint` | 0xf3506, 0x3f | 9 |
| `F101` | `CameraPanToPoint` | 0xf3545, 0x41 | 109 |
| `F102` | `CameraScrollToPoint` | 0xf3586, 0x41 | 0 |
| `F103` | `CameraZoomTo` | 0xf35c7, 0x28 | 0 |
| `F104` | `CameraViewFrom` | 0xf35ef, 0x28 | 0 |
| `F105` | `CameraStartRotate` | 0xf3617, 0x28 | 63 |
| `F106` | `CameraStopRotate` | 0xf363f, 0x1e | 56 |
| `F107` | `CameraTrackObject` | 0xf365d, 0x26 | 1 |
| `F108` | `CameraStopTrack` | 0xf3683, 0x1e | 1 |
| `F109` | `CameraIsPanning` | 0xf36a1, 0x2a | 0 |
| `F110` | `CameraIsScrolling` | 0xf36cb, 0x2a | 0 |
| `F111` | `CameraIsSpinning` | 0xf36f5, 0x2a | 0 |
| `F112` | `CameraStore` | 0xf371f, 0x1e | 49 |
| `F113` | `CameraRestore` | 0xf373d, 0x23 | 48 |
| `F114` | `PIPCameraLookAtPoint` | 0xf3760, 0x4d | 108 |
| `F115` | `PIPCameraPanToPoint` | 0xf37ad, 0x48 | 30 |
| `F116` | `PIPCameraScrollToPoint` | 0xf37f5, 0x48 | 0 |
| `F117` | `PIPCameraZoomTo` | 0xf383d, 0x2f | 31 |
| `F118` | `PIPCameraViewFrom` | 0xf386c, 0x2f | 0 |
| `F119` | `PIPCameraStartRotate` | 0xf389b, 0x2f | 56 |
| `F120` | `PIPCameraStopRotate` | 0xf38ca, 0x25 | 55 |
| `F121` | `PIPCameraTrackObject` | 0xf38ef, 0x2d | 435 |
| `F122` | `PIPCameraStopTrack` | 0xf391c, 0x25 | 0 |
| `F123` | `PIPCameraIsPanning` | 0xf3941, 0x31 | 0 |
| `F124` | `PIPCameraIsScrolling` | 0xf3972, 0x31 | 0 |
| `F125` | `PIPCameraIsSpinning` | 0xf39a3, 0x31 | 0 |
| `F126` | `PIPCameraStore` | 0xf39d4, 0x25 | 1 |
| `F127` | `PIPCameraRestore` | 0xf39f9, 0x2a | 1 |
| `K128` | `PIPRelease` | 0xf3a23, 0x38 | 466 |
| `K129` | `FreezeGame` | 0xf3a5b, 0x12 | 69 |
| `K130` | `UnFreezeGame` | 0xf3a6d, 0x12 | 70 |
| `K131` | `DisableUI` | 0xf3a7f, 0xf | 77 |
| `K132` | `EnableUI` | 0xf3a8e, 0xa | 78 |
| `K133` | `ObjectDeploy` | 0xf3aa2, 0x88 | 14 |
| `K134` | `ObjectUndeploy` | 0xf3b2a, 0x5b | 0 |
| `K135` | `ObjectSell` | 0xf3edb, 0x75 | 21 |
| `K136` | `ObjectRemove` | 0xf3b85, 0x2c | 0 |
| `K137` | `NewObjectInAPC` | 0xf3bb1, 0x95 | 60 |
| `K138` | `ObjectIsCarried` | 0xf3c46, 0x73 | 0 |
| `K139` | `NewObjectOffsetOrientation` | 0xf3cc9, 0x1bb | 2313 |
| `K140` | `GetNeutralSide` | 0xf3e84, 0x16 | 2 |
| `K141` | `GetNeutralExitPoint` | 0xf408a, 0x54 | 1 |
| `K142` | `PlaySound` | 0xf3e9a, 0x15 | 0 |
| `K143` | `Neg` | 0xf3eaf, 0x17 | 1996 |
| `K144` | `SetValue` | 0xf3ec6, 0x15 | 22 |
| `K145` | `GetIsolatedEntrance` | 0xf40de, 0x54 | 0 |
| `K146` | `GetHideOut` | none: does nothing | 0 |
| `K147` | `GetConvoyWayPointFunction` | none: does nothing | 0 |
| `K148` | `GetValley` | 0xf4132, 0x6a | 0 |
| `K149` | `GetIsolatedInfantryRock` | none: does nothing | 0 |
| `K150` | `SetSideColor` | 0xf419c, 0x32 | 202 |
| `K151` | `GetSideColor` | 0xf41ce, 0x44 | 0 |
| `K152` | `SetSideCash` | 0xf4212, 0x4c | 1 |
| `K153` | `AirStrike` | 0xf425e, 0x5d | 8 |
| `K154` | `AirStrikeDone` | 0xf42bb, 0x25 | 5 |
| `K155` | `SetThreatLevel` | 0xf42e0, 0x21 | 25 |
| `K156` | `SetVeterancy` | 0xf4301, 0x42 | 1319 |
| `K157` | `FireSpecialWeapon` | 0xf4343, 0x47 | 0 |
| `K158` | `SetTilePos` | 0xf438a, 0x7d | 67 |
| `K159` | `CentreCursor` | 0xf3a98, 0xa | 0 |
| `K160` | `BreakPoint` | 0xf4407, 0x14 | 0 |
| `K161` | `NormalConditionLose` | 0xf441b, 0x77 | 12 |
| `K162` | `int` | 0xf002c, 0x73 | 3168 |
| `K163` | `obj` | 0xf009f, 0x65 | 1377 |
| `K164` | `pos` | 0xf0104, 0x65 | 496 |
| `K165` | `if` | the parser's | 5026 |
| `K166` | `else` | the parser's | 53 |
| `K167` | `endif` | the parser's | 5026 |
| `K168` | `==` | the parser's | 8160 |
| `K169` | `!=` | the parser's | 4 |
| `K170` | `>=` | the parser's | 139 |
| `K171` | `<=` | the parser's | 83 |
| `K172` | `>` | the parser's | 433 |
| `K173` | `<` | the parser's | 216 |
| `K174` | `&&` | the parser's | 3996 |
| `K175` | `\|\|` | the parser's | 135 |
| `K176` | `FALSE` | 0xf2ec8, 0x12 | 2128 |
| `K177` | `TRUE` | 0xf3cb9, 0x10 | 5134 |
| `K178` | `+` | the parser's | 1646 |
| `K179` | `-` | the parser's | 41 |
| `K180` | `=` | the parser's | 7055 |

## How the interpreter runs a function

Read from the switch's cases (TASK-140 to TASK-149 describe each function from them):

- **Arguments**: argument *k* is kept in three arrays at once, as an int at `[esp+0x14+4k]`, an
  object handle (16 bits) at `[esp+0x98+2k]` and a position (three floats) at `[esp+0xfc+12k]`;
  how many were given is at `[esp+0x6c]`. A function reads whichever its argument is.
- **Results**: a type at `[esp+0x3c]` (1 a number, a truth or a side; 2 an object; 3 a position)
  and the value at `[esp+0x40]`, a position at `[esp+0x48]`.
- **Objects** are looked up by handle in the world (`0x597880`); one that is gone gives 0 or
  false, or the error 'Object is not initialised' with the script's name and line. An object
  keeps its side at `+0x0c`, kind at `+0x10`, position at `+0x14`, health at `+0x3c`, the sides
  that see it as a bit mask at `+0x6c` and its rules entry at `+0x70`.
- **Co-op**: when the game is the co-op campaign (game type 3) and a side argument is the human
  side, a test is made for every side of the human team and is true if any is, and an amount is
  given to each. Some functions give the human side's turn to a random partner instead.
- **Events**: each script keeps a list of this update's events (`script+0x2c0`): kind 8 an object
  delivered, 6 and 9 one constructed, 11 one attacking a side. An `Event…` function's first
  argument is an object variable that receives the event's object.
- **Distances** are in the world's units, 32 to a square.

## The Atreides' first mission

`ATStart.tok` has 754 lines. It declares 53 variables, makes four sides besides the
player's, and gives the player 10 000 credits and a base already built: a construction yard
with walls round it, six windtraps, a barracks, an outpost, pillboxes and rocket turrets,
snipers, Kindjal infantry and Mongooses. The Harkonnen side has a palace, an outpost and a
hangar, three windtraps, four flame turrets, barrels, scouts and light infantry. At update 49
an MCV, two trikes and a Mongoose are brought in. Harkonnen light infantry, troopers and
buzzsaws come in waves by the entrances, and Atreides trikes and Mongooses in answer. The
Mentat speaks at the start and at updates 430, 490 and 550, and as each wave comes. The
mission is won when the three Harkonnen main buildings are destroyed.

## Points on the map

A map's `test.xbf` has named areas of named lists of points. Those the scripts use are the
first point of a list: `Primary` and `Default` in the area `Base`, `Script1` to `Script6` in
`Mission`, `Script7` on in `MoreScripts`. The Atreides' first map has its `Default` on the
left, where the script builds their base, and its `Primary` on the right, the Harkonnen's.

## What we do with them

`src/mission.rs` reads a `.tok` into its lines of tokens, and runs them: each line is a
statement (an `if`, its end, a variable set, or a call), all of them run once an update with
what is inside an `if` that does not hold left out, as the original's scripts are written to
be run. The functions and keywords in the table above do what it says; the others do
nothing yet. The update is taken to be a tenth of a second, which is a guess.

A battle for a territory runs the script of its house, phase, attack (`M`) or defence (`D`)
and territory, where the disc's `missionlist.ini` in `MISSIONS0001` lists one: the
Atreides' defence of Bilar Slopes, territory 7, in phase 1 is `ATP1D7FR`, the prison camp.
A script's sides are the enemy's if it makes them the player's enemy, the player's if it
makes them the enemy of one of those, and nobody's otherwise, as the camp's prisoners are;
what a side is follows each `F59` and `F60` again, as the scripts put things down before
saying whose enemy they are. A side the original keeps apart, such as Fremen freed to fight
beside the player, is the player's own here, as the battle has only two owners.

A house's first battle in the campaign runs its start script (`ATStart.tok` for the
Atreides): the battle starts with nothing but what the script puts down, the sides it makes
are all the player's enemy, and its lines of speech play off the disc that has them.
