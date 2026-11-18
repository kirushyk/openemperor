#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod archive;
mod battlefield;
mod campaign;
#[cfg(target_os = "macos")]
mod discs;
mod fog;
mod font;
mod game;
mod hud;
mod inflate;
mod maps;
mod mission;
#[cfg(target_os = "macos")]
mod music;
mod planet;
mod platform;
mod render;
#[cfg(target_os = "macos")]
mod sounds;
mod strategy;
mod units;
#[cfg(target_os = "macos")]
mod video;
mod weapons;
mod xbf;

use std::time::Instant;

/// Where the game's first disc is looked for if none of its discs is found mounted; and on
/// it, the intro movie, the bag of music, and the menus' track in it.
#[cfg(target_os = "macos")]
const DISC: &str = "/Volumes/EMPEROR1";
#[cfg(target_os = "macos")]
const INTRO_MOVIE: &str = "INSTALL/DATA/MOVIES/LEGALS.BIK";
/// The movie on the disc that plays when a faction is picked for the campaign, before the
/// player is asked to play for it.
#[cfg(target_os = "macos")]
fn faction_movie(faction: game::Faction) -> &'static str {
    match faction {
        game::Faction::Atreides => "INSTALL/DATA/MOVIES/I00_F03E.BIK",
        game::Faction::Harkonnen => "INSTALL/DATA/MOVIES/I00_F04E.BIK",
        game::Faction::Ordos => "INSTALL/DATA/MOVIES/I00_F05E.BIK",
    }
}
/// The movie a house's campaign opens with, on the house's disc: the original's `Phase0a`.
#[cfg(target_os = "macos")]
fn opening_movie(house: game::Faction) -> &'static str {
    match house {
        game::Faction::Atreides => "A00_F00E.BIK",
        game::Faction::Ordos => "O00_F00E.BIK",
        game::Faction::Harkonnen => "H01_F00E.BIK",
    }
}

/// What `--help` prints.
const USAGE: &str = "\
Usage: openemperor [OPTIONS] [MOVIE]

Plays Emperor: Battle for Dune with the original game's data, read off its discs mounted
under /Volumes (EMPEROR1 installs it; EMPEROR2 to EMPEROR4 add each house's movies, music
and speech).

Arguments:
  [MOVIE]            A movie to play in place of the disc's intro

Options:
  --campaign-maps    List the campaign's maps among the skirmish's
  --mission <NAME>   Go straight into a campaign territory's battle, by its script's name
                     on the disc (ATP1M7FR: Atreides, phase 1, attack (M) or defence (D),
                     territory 7, with the Fremen)
                     or a Heighliner's, as AtreidesHeighlinerMission
  --missions         List the battles --mission can go into, and quit
  -h, --help         Print this help and quit
";

/// What the command line asks for.
#[derive(Debug, Default, PartialEq)]
struct Options {
    help: bool,
    campaign_maps: bool,
    list_missions: bool,
    mission: Option<String>,
    movie: Option<std::path::PathBuf>,
}

impl Options {
    fn parse(mut args: impl Iterator<Item = std::ffi::OsString>) -> Result<Self, String> {
        let mut options = Options::default();
        while let Some(arg) = args.next() {
            match arg.to_str() {
                Some("-h" | "--help") => options.help = true,
                Some("--campaign-maps") => options.campaign_maps = true,
                Some("--missions") => options.list_missions = true,
                Some("--mission") => {
                    let name = args.next().ok_or("--mission needs a mission's name")?;
                    options.mission = Some(name.to_string_lossy().into_owned());
                }
                Some(other) if other.starts_with('-') => {
                    return Err(format!("Unknown option {other}; see --help"));
                }
                _ => options.movie = Some(arg.into()),
            }
        }
        Ok(options)
    }
}

/// Prints the battles `--mission` can go into: each script's name, and what it is.
fn list_missions() {
    let names = campaign::mission_names();
    if names.is_empty() {
        eprintln!("No missions found: is the game's first disc mounted?");
        std::process::exit(1);
    }
    for name in names {
        if let Some((house, phase, mission)) = campaign::named_mission(name) {
            let title = campaign::Campaign::new(house, 1).title(mission);
            println!("{name:<12} {}, phase {phase}: {title}", house.label());
        }
    }
}

fn main() {
    let options = match Options::parse(std::env::args_os().skip(1)) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    };
    if options.help {
        print!("{USAGE}");
        return;
    }

    // A different placing of the sides each run; the clock is all that is wanted of it.
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |since| since.subsec_nanos());
    // The disc's maps are listed before anything asks for one, the campaign's too if asked.
    // Whichever of the game's discs are mounted: the one that installs it has what every
    // house shares, the others a house's own music and movies.
    #[cfg(target_os = "macos")]
    let all_discs = discs::mounted();
    #[cfg(target_os = "macos")]
    let disc = all_discs
        .iter()
        .find(|disc| discs::installs(disc))
        .cloned()
        .unwrap_or_else(|| std::path::PathBuf::from(DISC));
    #[cfg(target_os = "macos")]
    let disc = disc.as_path();
    #[cfg(target_os = "macos")]
    maps::load(disc, options.campaign_maps);
    #[cfg(target_os = "macos")]
    campaign::load(disc);
    if options.list_missions {
        list_missions();
        return;
    }
    // Known by the battle it names, so a Heighliner's name may be given without its spaces.
    let on_disc = |name: &str| {
        let battle = campaign::named_mission(name);
        battle.is_some()
            && campaign::mission_names()
                .iter()
                .any(|listed| campaign::named_mission(listed) == battle)
    };
    if let Some(name) = &options.mission
        && !on_disc(name)
    {
        eprintln!("No mission {name} on the disc; --missions lists them");
        std::process::exit(2);
    }
    let (mut platform, width, height) = match platform::Platform::new(render::TITLE) {
        Ok(opened) => opened,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(1);
        }
    };
    let mut game = game::Game::new(width as f32, height as f32).with_start_seed(seed);
    #[cfg(target_os = "macos")]
    let mounted = disc.is_dir();
    // The disc's own models, read while the menus show, take the place of ours once read.
    #[cfg(target_os = "macos")]
    if mounted {
        render::load_original_models(disc);
        units::fire_from(render::original_muzzles);
        let disc = disc.to_path_buf();
        let houses = all_discs.clone();
        std::thread::spawn(move || {
            sounds::load(&disc);
            sounds::load_house_dialog(&houses);
        });
    }
    // Which of its lines a unit says, each time another.
    #[cfg(target_os = "macos")]
    let mut line_random = seed | 1;
    #[cfg(target_os = "macos")]
    let (mut movie, menu_music, battle_music) = {
        // A movie given on the command line plays in place of the disc's, and none before a
        // mission gone straight into.
        let path = options
            .movie
            .clone()
            .or_else(|| (mounted && options.mission.is_none()).then(|| disc.join(INTRO_MOVIE)));
        let intro = path.as_deref().and_then(video::Movie::open);
        if let Some(movie) = &intro {
            game.start_movie();
            platform.play_sound(
                movie.sound(),
                video::SOUND_CHANNELS,
                video::SOUND_RATE,
                false,
            );
        }
        // From every disc that is mounted: each house's has its own music.
        let bags = discs::music_bags(&all_discs);
        let menu_music = mounted.then(|| music::menu_tracks(&bags));
        let battle_music = mounted.then(|| music::battle_tracks(&bags));
        (intro, menu_music, battle_music)
    };
    if let Some(name) = &options.mission {
        game.play_mission(name);
    }
    // The screen a movie was last looked for on, so that each faction's plays once per pick.
    #[cfg(target_os = "macos")]
    let mut movie_screen = game.screen;
    // When the menus' piece playing now is over, if one is.
    #[cfg(target_os = "macos")]
    let mut menu_track_ends: Option<Instant> = None;
    // When the battle track playing now is over, if one is.
    #[cfg(target_os = "macos")]
    let mut battle_track_ends: Option<Instant> = None;
    // Which map's ground the disc's pictures, read with its models, were given to the backend
    // with, once they have been.
    #[cfg(target_os = "macos")]
    let mut pictures_given = None;
    // The battlefield's sounds of firing and dying, waiting their turn and playing.
    #[cfg(target_os = "macos")]
    let mut effects = sounds::Effects::default();
    // Until when the mission's Mentat speaks, whom nothing else said cuts off.
    #[cfg(target_os = "macos")]
    let mut mentat_until = Instant::now();
    // The volumes last given to the backend, to give them again only when they change.
    let mut volumes = None;
    let mut last = Instant::now();
    while !game.quit {
        while let Some(event) = platform.poll_event() {
            game.handle(event);
        }
        let now = Instant::now();
        let seconds = (now - last).as_secs_f32();
        game.update(seconds);
        last = now;
        // What is heard on the battlefield, as the disc's models play it.
        #[cfg(target_os = "macos")]
        {
            for noise in game.take_noises() {
                effects.hear(render::original_noise_sounds(&noise));
            }
            for effect in effects.due(seconds) {
                let line = &effect.line;
                platform.play_effect(&line.samples, line.channels, line.rate, effect.volume);
            }
        }
        // How loud the options say the music, the units and the battlefield are.
        let settings = &game.settings;
        let wanted = [
            settings.music_volume,
            settings.voice_volume,
            settings.sound_volume,
        ];
        if volumes != Some(wanted) {
            let [music, lines, effects] = wanted.map(|volume| volume as f32 / 100.0);
            platform.set_volumes(music, lines, effects);
            volumes = Some(wanted);
        }
        // What the battlefield tells the player is said over the music.
        #[cfg(target_os = "macos")]
        for speech in game.take_speeches() {
            if now < mentat_until {
                continue;
            }
            if let Some(line) = sounds::spoken(speech, game.faction_of(0)) {
                platform.play_line(&line.samples, line.channels, line.rate);
            }
        }
        // What the mission's Mentat says, off whichever disc has it.
        #[cfg(target_os = "macos")]
        for number in game.take_mission_lines() {
            if let Some(line) = sounds::mission_line(number) {
                platform.play_line(&line.samples, line.channels, line.rate);
                mentat_until = now + std::time::Duration::from_secs_f32(line.seconds());
            }
        }
        // What the player's units answer is said over the music.
        #[cfg(target_os = "macos")]
        if let Some((said, speakers)) = game.take_reply()
            && now >= mentat_until
        {
            let pick = (battlefield::next_random(&mut line_random) * 1024.0) as usize;
            if let Some(line) = sounds::reply(said, &speakers, pick) {
                platform.play_line(&line.samples, line.channels, line.rate);
            }
        }
        #[cfg(target_os = "macos")]
        if let Some(playing) = &mut movie {
            if playing.advance(game.screen_elapsed()) {
                platform.set_backdrop(Some(platform::Image {
                    rgba: playing.frame(),
                    width: playing.width,
                    height: playing.height,
                }));
            }
            if playing.is_over() {
                game.end_movie();
            }
            // Over, or skipped with a key or a click.
            if !game.is_playing_movie() {
                movie = None;
                platform.set_backdrop(None);
                platform.stop_sound();
            }
        }
        // The menus' pieces of every mounted disc play one after another in the menus, and
        // only there.
        #[cfg(target_os = "macos")]
        if let Some(tracks) = &menu_music
            && movie.is_none()
        {
            if game.is_in_menus() {
                if menu_track_ends.is_none_or(|ends| now >= ends)
                    && let Ok(sound) = tracks.try_recv()
                {
                    let lasts = std::time::Duration::from_secs_f32(music::seconds(&sound));
                    platform.play_sound(&sound, video::SOUND_CHANNELS, video::SOUND_RATE, false);
                    menu_track_ends = Some(now + lasts);
                }
            } else if menu_track_ends.take().is_some() {
                platform.stop_sound();
            }
        }
        // A faction's movie plays when its screen opens, after the menus' music has stopped.
        #[cfg(target_os = "macos")]
        if game.screen != movie_screen {
            movie_screen = game.screen;
            let opened = match game.screen {
                game::Screen::Movie(faction) if mounted => {
                    video::Movie::open(&disc.join(faction_movie(faction)))
                }
                // A house's opening movie is on its own disc.
                game::Screen::Opening(house) => discs::movie(&all_discs, opening_movie(house))
                    .and_then(|movie| video::Movie::open(std::path::Path::new(&movie))),
                _ => None,
            };
            if let Some(opened) = opened {
                game.start_movie();
                platform.play_sound(
                    opened.sound(),
                    video::SOUND_CHANNELS,
                    video::SOUND_RATE,
                    false,
                );
                movie = Some(opened);
                // Its sound takes over from the menus' piece, which starts again after it.
                menu_track_ends = None;
            }
        }
        // In battle the disc's long tracks play one after another, and stop when it is over.
        #[cfg(target_os = "macos")]
        if let Some(tracks) = &battle_music {
            if game.in_battle {
                if battle_track_ends.is_none_or(|ends| now >= ends)
                    && let Ok(sound) = tracks.try_recv()
                {
                    let lasts = std::time::Duration::from_secs_f32(music::seconds(&sound));
                    platform.play_sound(&sound, video::SOUND_CHANNELS, video::SOUND_RATE, false);
                    battle_track_ends = Some(now + lasts);
                }
            } else if battle_track_ends.take().is_some() && menu_track_ends.is_none() {
                platform.stop_sound();
            }
        }
        #[cfg(target_os = "macos")]
        if pictures_given != Some(game.battle_map)
            && let Some(pictures) = render::original_pictures(game.battle_map)
        {
            platform.set_pictures(&pictures, render::PICTURE_SIDE as u32);
            let (wash, width, height) = render::original_wash(game.battle_map).unwrap_or_default();
            platform.set_wash(wash, width as u32, height as u32);
            pictures_given = Some(game.battle_map);
        }
        platform.draw(&render::build_frame(&game), render::clear_color(&game));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Options, String> {
        Options::parse(args.iter().map(std::ffi::OsString::from))
    }

    #[test]
    fn the_command_line_reads_its_options_and_a_movie() {
        assert_eq!(parse(&[]), Ok(Options::default()));
        let options = parse(&["--mission", "ATP1M7FR", "--campaign-maps"]).unwrap();
        assert_eq!(options.mission.as_deref(), Some("ATP1M7FR"));
        assert!(options.campaign_maps && !options.list_missions);
        assert!(parse(&["--missions"]).unwrap().list_missions);
        assert!(parse(&["-h"]).unwrap().help);
        let movie = parse(&["intro.bik"]).unwrap().movie;
        assert_eq!(movie, Some("intro.bik".into()));
        assert!(parse(&["--mission"]).is_err());
        assert!(parse(&["--nonsense"]).is_err());
    }
}
