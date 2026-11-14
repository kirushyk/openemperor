# Units

Every unit and building the original game has, next to ours, to keep track of what is still
missing. The original's names are the ones in its `Rules.txt`, from its `[UnitTypes]` and
`[BuildingTypes]` lists, and its models are the ones its `ArtIni.txt` names. Ours are the
English names the original shows, from its `Text strings.txt`, without the house in front.
In our code each one is named after the original's name in Rust case, as `Role::HkBuzzsaw` for
`HKBuzzsaw`; what every house shares goes without a house, as `Role::Scout` or `Role::Helipad`.

The **Drawn as** column says what is drawn while the original disc is mounted:

- **Disc**: the original's model, with its own textures and animations;
- **Ours**: our own model, because the disc has no model for it or we don't use it yet;
- **—**: we don't have the unit at all.

Without the disc, everything we have is drawn with our own models.

## Infantry

| Original | House | Ours | Built by (ours) | Drawn as | Notes |
| --- | --- | --- | --- | --- | --- |
| ATScout | Atreides | Scout | every house | Disc | Each house gets its own scout model. |
| HKScout | Harkonnen | Scout | every house | Disc | |
| ORScout | Ordos | Scout | every house | Disc | |
| ATEngineer | Atreides | Engineer | every house | Disc | Each house gets its own engineer model. |
| HKEngineer | Harkonnen | Engineer | every house | Disc | |
| OREngineer | Ordos | Engineer | every house | Disc | |
| ATInfantry | Atreides | Infantry | Atreides | Disc | |
| ATSniper | Atreides | Sniper | Atreides | Disc | |
| ATKindjal | Atreides | Kindjal Infantry | Atreides | Disc | Plays its deployed actions while dug in. |
| HKLightInf | Harkonnen | Light Infantry | Harkonnen | Disc | |
| HKTrooper | Harkonnen | Trooper | Harkonnen | Disc | |
| HKFlamer | Harkonnen | Flamethrower Infantry | Harkonnen | Disc | |
| ORChemical | Ordos | Chemical Trooper | Ordos | Disc | |
| ORSaboteur | Ordos | Saboteur | Ordos | Disc | |
| ORMortar | Ordos | Mortar Infantry | Ordos | Disc | Plays its deployed actions while dug in. |
| ORAATrooper | Ordos | — | — | — | The AA Trooper. |

## Vehicles

| Original | House | Ours | Built by (ours) | Drawn as | Notes |
| --- | --- | --- | --- | --- | --- |
| Harvester | all | Harvester | every house | Disc | |
| MCV | all | MCV | every house | Disc | Deployed, it is drawn as its house's construction yard. |
| HKAssault | Harkonnen | Assault Tank | Harkonnen | Disc | |
| HKBuzzsaw | Harkonnen | Buzzsaw | Harkonnen | Disc | |
| HKFlame | Harkonnen | Flame Tank | Harkonnen | Disc | |
| HKInkVine | Harkonnen | Inkvine Catapult | Harkonnen | Disc | |
| HKMissile | Harkonnen | Missile Tank | Harkonnen | Disc | |
| HKDevastator | Harkonnen | Devastator | Harkonnen | Disc | |
| ATTrike | Atreides | Sand Bike | Atreides | Disc | |
| ATMongoose | Atreides | Mongoose | Atreides | Disc | |
| ATMinotaurus | Atreides | Minotaurus | Atreides | Disc | |
| ATAPC | Atreides | APC | Atreides | Disc | |
| ATRepairUnit | Atreides | Repair Vehicle | Atreides | Disc | |
| ATSonicTank | Atreides | — | — | — | The Sonic Tank. The disc has its model (AT_SonicTank). |
| ORDustScout | Ordos | Dust Scout | Ordos | Disc | |
| ORLaserTank | Ordos | Laser Tank | Ordos | Disc | |
| ORKobra | Ordos | Kobra | Ordos | Disc | |
| ORAPC | Ordos | APC | Ordos | Disc | |
| ORDeviator | Ordos | — | — | — | The Deviator. The disc has its model (OR_Deviator). |

## Aircraft

| Original | House | Ours | Built by (ours) | Drawn as | Notes |
| --- | --- | --- | --- | --- | --- |
| Carryall | all | Carryall | every house | Disc | |
| ATADVCarryall | Atreides | Advanced Carryall | every house | Disc | Each house gets its own model. |
| HKADVCarryall | Harkonnen | Advanced Carryall | every house | Disc | |
| ORADVCarryall | Ordos | Advanced Carryall | every house | Disc | |
| ATOrni | Atreides | Ornithopter | every house | Disc | Only House Atreides has it in the original. |
| HKGunship | Harkonnen | Gunship | Harkonnen | Disc | |
| ATADP | Atreides | Air Drone | Atreides | Disc | |
| HKADP | Harkonnen | Air Defense Platform | every house | Disc | Only House Harkonnen has it in the original. |
| ORADP | Ordos | Anti-Aircraft Mine | Ordos | Disc | |
| OREITS | Ordos | Eye in the Sky | Ordos | Disc | |

## Superweapons

| Original | House | Ours | Drawn as | Notes |
| --- | --- | --- | --- | --- |
| ATHawkWeapon | Atreides | Hawk Strike | Ours | |
| HKDeathHand | Harkonnen | Death Hand | Ours | The disc has the missile's model (HK_Deathhand). |
| ORBeamWeapon | Ordos | Chaos Lightning | Ours | |

## Towers

| Original | House | Ours | Drawn as | Notes |
| --- | --- | --- | --- | --- |
| ATPillbox | Atreides | Machinegun Post | Disc | |
| ATRocketTurret | Atreides | Rocket Turret | Disc | |
| HKFlameTurret | Harkonnen | Flame-Turret | Disc | |
| HKGunTurret | Harkonnen | Gun-Turret | Disc | |
| ORGasTurret | Ordos | Gas Turret | Disc | |
| ORPopUpTurret | Ordos | Pop-up Turret | Disc | |

## Buildings

Each house has its own model of each of these.

| Original | Ours | Drawn as | Notes |
| --- | --- | --- | --- |
| ConYard | the deployed MCV | Disc | |
| Barracks | Barracks | Disc | |
| Factory | Factory | Disc | |
| Hanger | Hangar | Disc | |
| Helipad | Landing Pad | Disc for Atreides and Harkonnen, ours for Ordos | The original gives House Ordos no landing pad. We give it one, drawn with our model. |
| Refinery | Refinery | Disc | Our harvesters dock at our refinery's pad places, not the original's. |
| RefineryDock | part of the Refinery | — | |
| Outpost | Outpost | Disc | |
| Palace | Atreides Palace, Harkonnen Palace, Ordos Palace | Disc | |
| SmWindtrap | — | — | The original's power plant. |
| Wall | — | — | |
| Starport | — | — | |
| FactoryFrigate | — | — | |
| Smelter | — | — | Left out of the original's own build list (commented out). |

## The small factions

Our skirmish setup already lets you pick these as allies, but none of their units or
buildings exist yet.

| Faction | Original units | Original building | Ours |
| --- | --- | --- | --- |
| Ix | IXInfiltrator, IXProjector | IXResCentre | — |
| Tleilaxu | TLContaminator, TLLeech | TLFleshVat | — |
| Sardaukar | IMSardaukar, IMADVSardaukar, IMDropShip | IMBarracks | — |
| Fremen | FRFremen, FRADVFremen, WormRider | FRCamp | — |
| Guild | GUMaker, GUNIABTank | GUPalace | — |

## Campaign, neutral and scenery units

These appear in the original's missions or as scenery, not in a skirmish. We have none of them.

| Kind | Original |
| --- | --- |
| Campaign characters | IMGeneral, ATGeneral, HKGeneral, ORGeneral, IXScientist, TLScientist, IXSlave, CubScout, ATMilitia, StoryFRFremen, StoryFRADVFremen |
| Sandworms | SmallWorm, SurfaceWorm, BigWorm |
| Neutral vehicles and people | INYak, INYakHauder, INYakRider, INSandCrawler, INBuggy, INMedicalVehicle, INFemaleCiv, INMaleCiv, INTLTransport, INIXTransport, INTLWalker, INFRBiker, GUWormCatcher, SMQuad |
| Used by the game itself | Frigate, FakeHarvester, StuntATADVCarryall |
| Left out of the original's own list (commented out) | Scout (shared by every house), Thumper, Scavenger |
