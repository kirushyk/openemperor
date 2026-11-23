# GAME.EXE

A map of the original's executable: what is in it, where, and how much of it we have. It was
made from the file alone, from its tables, messages and switch statements; nothing in it was
run. Each piece we lack has a backlog task, named in the last column.

## The file

`GAME.EXE` is in `INSTALL/Game1.cab` on the first disc (`cabextract -F GAME.EXE Game1.cab`).
It is 2 449 408 bytes, built with Visual C++ 6 and linked on 11 May 2001 at 16:03:20. It has
no symbols, no relocations and no names of its C++ classes, but its messages name many of them
(`CAiConstructionManager`, `CWorldAttrib`, `CBlackBox`…), and 45 of its source files
(`local-notes.md` lists them).

| Section | File offset | Size in the file | In memory | Size in memory | What |
| --- | --- | --- | --- | --- | --- |
| `.text` | `0x1000` | `0x1c3000` | `0x401000` | `0x1c2679` | Code, about 14 000 functions |
| `.rdata` | `0x1c4000` | `0x1e000` | `0x5c4000` | `0x1ddda` | Imports, library messages, float constants |
| `.data` | `0x1e2000` | `0x72000` | `0x5e2000` | `0x590065` | The game's tables and messages; past `0x254000` only in memory |
| `.rsrc` | `0x254000` | `0x2000` | `0xb73000` | `0x1ff0` | Icon and version |

The file is laid out as it is loaded, so **an offset into the file is the address in memory
less `0x400000`**. Every offset below is into the file.

Linked in: zlib 1.1.3 (`deflate` at `0x1cf438`, `inflate` at `0x1cff68`), DirectDraw and
Direct3D 7 with D3DX (their error names from `0x1d0204`), DirectInput, DirectSound, Bink
(`binkw32.dll`), Winsock, IMM, Westwood's chat library for Westwood Online, and SNMP for
finding the local address.

## How the map was made

- Strings: every run of four or more printable bytes ending in a zero, in `.rdata` and `.data`.
- References: every four bytes in `.text` that equal a string's address in memory. Most
  messages are used once, so the code that uses one is the code of that subsystem.
- Functions: the targets of every `call` in `.text`, with every pointer into `.text` from the
  data (virtual tables and jump tables). These are a little too many, as a switch's cases
  count as starts, so a function's size here is the distance to the next start.
- Switches: `jmp [4*reg+table]` (`ff 24 85`/`ff 24 8d`) with the table's run of addresses
  into `.text`; the largest are listed under [Switches](#switches).
- Function names: messages that name their own function, as `CWorldAttrib::GetBF, Invalid line
  %d` or `CheckForWinCondition : …`, give 252 names. The game's own (110) each have a task where
  we lack what they do; the networking ones are listed in TASK-177.

## Tables

| Table | Offset | Size, to the end of its last name | Records | Ours |
| --- | --- | --- | --- | --- |
| Script functions and keywords | `0x1fe0b0` | `0x410c` | 181 × `0x5c`: the name first; its place is its token | All named in [tok-format.md](tok-format.md); 56 of the 162 functions carried out, 128 being used by the scripts (TASK-140 to TASK-149) |
| Script interpreter's switch | `0xf0011` (`cmp ecx,0xb1`), table `0xf4a00` | `0x2c8` | 178 addresses, one a function | Each case's offset and size are in [tok-format.md](tok-format.md) |
| Script file header fields | `0x1fdc30` | `0x43f` | `0x28` each: `[HEADER]`, `SubHouse`, `Lifespan`, `Phase`, `PlayerHouse`, `AIHouse`, … `AllyGU` | Not read (TASK-171) |
| Rules.txt keywords | `0x20e79c` | `0x10428` | 333 × `0xc8`: sections `General` … `TurretJointTypes`, then the keys | Copied by hand into the code; unused keys in TASK-150 to TASK-154 |
| ai.ini keys and sections | `0x1e588c` | `0x74b` | Packed names, `MinPercentDevastatorExplodesHealth` … `BuildingConstructionRatios` | Not read (TASK-158) |
| AI script chunks and keywords | `0x1e8778`, `0x1e8b08` | `0x322`, `0x12b` | Packed names: `everything` … `frequency`; `DEFINES` … `ENDDESCRIPTION` | Not read (TASK-159) |
| Animation names | `0x1edb58` | `0x807` | `0x1e` each: `Stationary`, `Idle 0` … `Refinery Pad 2` | Read from the models; the unplayed ones are in [not-implemented.md](not-implemented.md) |
| Cursor names | `0x1f4898` | `0x2d4` | Packed: `PLACEBUILDING` … `POINTER`; models `cursors\CU_%s_H0.XBF` at `0x1f4b6c` | 14 used (`src/render/original.rs`) |
| Battlefield's spoken events | `0x1f5214` | `0x28f` | Packed: `NOSTRING`, `LoseGame` … `HarvAttack` | Most (TASK-170) |
| Key commands | `0x1f9978` | `0x4a9` | Packed: `HarvRetreat` … `ScrollUp`; key names from `0x1fa004` | Some (TASK-162) |
| Options | `0x1f813c` | `0xb70` | `0x6c` each: `// Game Options \\`, `PlayedIntro` … `NetworkCard` | Not kept (TASK-163) |
| Sound table attributes | `0x1ee820`, `0x220434` | | Flags `DECAY` … `LOWEST`; attributes `RANGE` … `SOUNDS` | `Volume` and `Limit` (TASK-169) |
| Camera settings | `0x21fb78` | `0xc6` | `CAMERA.INI`, `StartZoom` … `Scrolling` | Read (`src/game.rs`) |
| Territory files | `0x1f257c` | `0xc0e` | `0xc8` each: `arrakis territories.txt` … `hk civil war2 connections.txt` | Arrakis's (`src/campaign.rs`) |

## Switches

| Jump | Table | Cases | What it switches on | Mapped |
| --- | --- | --- | --- | --- |
| `0xf0025` | `0xf4a00` | 178 | Mission-script function number | Each case, in [tok-format.md](tok-format.md) and TASK-140 to TASK-149's subtasks |
| `0x127040` | `0x128ea4` | 265 (keys 47–311) | Rules.txt keyword, in `CWorldAttrib::GameObjectAttributes`: its key number less 47; the bullet and warhead keys go to the default, being read by `BulletAttributes` | Case per key, not yet what each stores |
| `0x129387` | `0x129bb4` | 89 | Rules.txt keyword, in `CWorldAttrib::GeneralAttributes` | No |
| `0xc55f1` | `0xc6cc4` | 51 | An order in the order queue (`ProcessOrder, Invalid Order: %d`): every command a player or the AI gives, superweapons among them | No |
| `0xbdddd` | `0xbfc20` | 52 | A key command (the names at `0x1f9978`) | No |
| `0xbd0e3` | `0xbd2b4` | 50 | Battlefield clicks: primary building, waypoints, tooltips | No |
| `0xbbe68` | `0xbd018` | 30 | Placing a building or wall (`BuildingThud`, `WallThud`, `CannotDeployHere`) | No |
| `0x5c4a6`, `0x5c976` | `0x5c8f4`, `0x5cea8` | 22, 22 | The AI's unit classes (`Identified … unit %s`) | By their messages, TASK-159.3 |
| `0x9c0e0` | `0x9c62c` | 42 | Interface script tokens (check boxes, labels) | No |
| `0x192850` | `0x192f50` | 28 | Kinds of object saved and loaded (`CGObjectList::Load`) | No |
| `0x7cb00`, `0x7cef7`, `0x9e4e8`, `0x12380f`, `0x1897b7` | | 20–50 | Network and co-op messages, Westwood Online events | No (TASK-177) |
| `0x1a3f98`, `0x1a4193`, `0x1a4587` | | 35–85 | DirectDraw error names | Not wanted |

## Data files it reads

| File | Its name at | Read at | Ours |
| --- | --- | --- | --- |
| `RULES.TXT` | `0x21ec8c` | `0x12bb2e` (`CWorldAttrib::LoadRulesTextFile`) | Copied by hand |
| `artini.txt` | `0x1f6948` | `0x6f863` (`CArtIni`) | Read |
| `sfx.txt`, `music.txt`, `dialog.txt` | `0x1ef370`, `0x1ef364`, `0x1ef358` | `0x7721e` | Sounds and dialogue tables read, in part |
| `Movies.txt` | `0x1f5aec` | `0xa88fd` | Movie names by hand |
| `CAMERA.INI` | `0x21fb78` | `0x131c7d` | Read |
| `MAPINFO.INI` | `0x1f9124` | `0xd7455` | Not read |
| `ai.ini`, `ai_%s.ini`, `ai_difficulty.ini` | `0x1e6020`, `0x1e6014`, `0x1e63a8` | `0x3101b`, `0x31aaf` | Not read (TASK-158, TASK-154) |
| `objectsets.txt`, `AI_scripts.txt` | `0x1e759c`, `0x1c4e58` | `0x3c59c` | Not read (TASK-159) |
| `Difficulty.txt` | `0x1f1b18` | | Not read (TASK-172) |
| `PhaseRules.txt` | `0x1f33d4` | | Read |
| `Forced Missions.txt`, mission files `AT*.txt`… | `0x2029b0`, `0x202d30` | `0xf6e3f`, `0xf7ab6` | Not read (TASK-171) |
| `newunits.ini` | `0x222aa0` | `0x16ce6a` | Read |
| `health\script.txt` | `0x202ff0` | `0xf8ac9` | Not read (TASK-165) |
| `BoundBox\Script.txt` | `0x1f119c` | `0x83b75` | Not read: the selection boxes' look |
| `Radar\%s.ini` | `0x20de44` | `0x119e38` | Not read (TASK-167) |
| `Tooltips\%s.ini` | `0x1f692c` | `0xb7614` | Not read (TASK-168) |
| `KEYS.MAP` | `0x1f9f28` | `0xd9003` | Not read (TASK-162) |
| `spice.ini` | `0x221170` | | Read |
| `combiner.ini` | `0x20c644` | `0x10e8e8` | Not read: which models are merged, for speed |
| `texture.dat` | `0x22095c` | `0x139154` | Not read: textures' shadow heights |
| `subtitles\subtitle.ini` | `0x2214e8` | | Not read (TASK-176) |
| `screenres.ini`, `EMPEROR.INI`, `setup.ini`, `resource.cfg` | `0x1f8f94`, `0x1f5944`, `0x21fc74`, `0x1e2ec8` | | Ours has no need of them, `EMPEROR.INI` aside (TASK-163) |

## Subsystems

What the messages and file names show the original does. *Anchor* is a message or name that
only that subsystem uses, and *Code* where the code refers to it.

| Subsystem | Anchor | Code | Ours | Task |
| --- | --- | --- | --- | --- |
| Mission scripts | Function names `0x1fe0b0` | Switch `0xf0011` | 56 of the 128 functions the scripts use | TASK-140 to TASK-149 |
| Picture-in-picture camera | `PIPRelease` record `0x200eb0` | Cases `0xf3760`–`0xf3a5b` | Sent to the main camera | TASK-148 |
| Rules | `0x21f2cc` | `0x12bb2e` | Most figures | TASK-150 to TASK-154 |
| Turrets | `TurretControl.CPP` `0x221b00` | | Our own aiming | TASK-152 |
| Sandstorms | `sandstormobject.cpp` `0x21f36c` | `0x12be08` | Tornado's look only | TASK-155 |
| Sandworms | `AiSurfaceWorm.cpp` `0x1eade0`, `unitWorm.cpp` `0x222bc0` | `0x6d9b2`, `0x171578` | The tutorial's scripted worm | TASK-156, TASK-87 |
| Crates | `crate.cpp` `0x1f4658`, `CashCrate` … `StepOnCrate` `0x1f467c`–`0x1f46a8` | `0x9edeb`, `0x9f1bc`–`0x9f3f9` | Money crates from wrecks | TASK-157 |
| AI: base building | `AiConstructionManager.cpp` `0x1e5684` | `0x2e667` | Our own | TASK-158 |
| AI: tactics and scripts | `0x1ea4a4`, `0x1ea420` | `0x4e292`, `0x4e19f` | Our own | TASK-159 |
| AI: patterns, units | `0x1e7988`–`0x1e8024`, `0x1ed028`–`0x1ed690` | `0x3f3f1`, `0x405d4`, `0x40288` | Our own | TASK-160 |
| Saving and loading | `%s.igs` `0x1f1efc`, `CWorld::Save` `0x2264c8` | `0x8f584`, `0x19700e` | Buttons that do nothing | TASK-161 |
| Keys | `KEYS.MAP` `0x1f9f28` | `0xd9003` | Fixed keys | TASK-162 |
| Options | `0x20cfa8` | `0x10f9d4` | Not kept | TASK-163 |
| Score screen | `0x21f410`, results `0x1f05c8`–`0x1f072c` | `0x12cb7b`, `0x7f854` | None | TASK-164 |
| Health halos | `health\script.txt` `0x202ff0` | `0xf8ac9` | Our own | TASK-165 |
| Placing and waypoints | `placement\build_*.xbf` `0x222cbc`, `waypoints\WP_%s_H0.XBF` `0x222de8` | `0x17618b`, `0x1789d1` | Our own | TASK-166 |
| Radar | `Radar\%s.ini` `0x20de44` | `0x119e38` | Our own minimap | TASK-167 |
| Tooltips | `Tooltips\%s.ini` `0x1f692c` | `0xb7614` | None | TASK-168, TASK-135 |
| Sound | `0x1ee678` | `0x7209d` | In part | TASK-169 |
| Spoken messages | Events `0x1f5214` | `0xa3151` | Most | TASK-170 |
| Mission files and house opinions | `[HEADER]` `0x1fdc30`, `HOUSE OPINIONS` `0x202b58` | `0xf7353`, `0xf74e0` | None | TASK-171 |
| Campaign reserves, AI turns | `Difficulty.txt` `0x1f1b18`, `MoveReserves` `0x1f1a14` | `0x8d95b`, `0x92f9c` | By chance | TASK-172 |
| Reinforcements, retreat | `CReinforcements` `0x20e264`, `CRetreat` `0x20e6ac` | `0x123374`, `0x124973` | In part | TASK-173 |
| Model effects | `FXDataHeader` `0x1c75c3`, `0x1f603c` | `0xb3bdf`, `0xb4376` | In part | TASK-174, TASK-129 |
| Spice mounds, debris, scrap, splats | `SpiceMound.CPP` `0x22112c`, `debris.cpp` `0x1f4bf8`, `Splat.CPP` `0x22117c` | `0x141db6`, `0xa103c`, `0x143417` | None | TASK-175 |
| Movies, splash, credits | `0x2214e8`, `0x1f90c8`, `0x2223c8` | `0xd6c3b`, `0x15ade5` | Movies play | TASK-176 |
| Upgraded mini-house units | `UnitProjector.CPP` `0x222b2c`, `unitNiabTank.cpp` `0x222ad8` | `0x17024f`, `0x16e007` | None | TASK-16 |
| The Emperor Worm | `unitBigWorm.cpp` `0x222760` | `0x166ee8` | None | TASK-34 |
| Tech levels, starport | Rules `TechLevel` `0x215e5c`, `0x220a18` | `0x13bcd6` | Starport done; tech levels not | TASK-27 |
| Low power | `PowerDown` `0x1f7744` | `0xcfcd9` | Our guess | TASK-28 |
| LAN, Westwood Online, replays | `CLANWrapper` `0x1fc6d4`, `game.rec` `0x20e0d4` | `0x11e636` | None | TASK-177 |
| Debug and diagnostics | `xanadu.log` `0x1e3e44`, `debugstream.ini` `0x1f597c`, memory checks `0x1e2a64` | | Not wanted | |

## What we have that the map confirms

Rules figures, models and their animations, the art list, the sound and dialogue tables in
part, the camera's settings, maps and their points, the campaign's Arrakis, phases and new
units, the starport, veterancy, walls and upgrades: each has its loader or table above, and
the code agrees with what the names say, except where the tasks above say otherwise.
