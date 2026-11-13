#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod archive;
mod battlefield;
mod fog;
mod font;
mod game;
mod hud;
mod inflate;
#[cfg(target_os = "macos")]
mod music;
mod planet;
mod platform;
mod render;
mod strategy;
mod units;
#[cfg(target_os = "macos")]
mod video;
mod weapons;
mod xbf;

use std::time::Instant;

/// Where the game's own disc is, once its owner has mounted it; and on it, the intro movie,
/// the bag of music, and the menus' track in it.
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
#[cfg(target_os = "macos")]
const MUSIC_BAG: &str = "INSTALL/DATA/MUSIC/music.bag";
#[cfg(target_os = "macos")]
const MENU_TRACK: &str = "IN_Menu";

fn main() {
    let (mut platform, width, height) = match platform::Platform::new(render::TITLE) {
        Ok(opened) => opened,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(1);
        }
    };

    // A different placing of the sides each run; the clock is all that is wanted of it.
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |since| since.subsec_nanos());
    let mut game = game::Game::new(width as f32, height as f32).with_start_seed(seed);
    #[cfg(target_os = "macos")]
    let disc = std::path::Path::new(DISC);
    #[cfg(target_os = "macos")]
    let mounted = disc.is_dir();
    // The disc's own models, read while the menus show, take the place of ours once read.
    #[cfg(target_os = "macos")]
    if mounted {
        render::load_original_models(disc);
    }
    #[cfg(target_os = "macos")]
    let (mut movie, menu_music, battle_music) = {
        // A movie given on the command line plays in place of the disc's.
        let path = std::env::args_os()
            .nth(1)
            .map(std::path::PathBuf::from)
            .or_else(|| mounted.then(|| disc.join(INTRO_MOVIE)));
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
        let bag = disc.join(MUSIC_BAG);
        let menu_music = if mounted {
            music::find(&bag, MENU_TRACK)
                .map(|track| video::decode_sound(&track.location(&bag)))
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let battle_music = mounted.then(|| music::battle_tracks(&bag));
        (intro, menu_music, battle_music)
    };
    // The screen a movie was last looked for on, so that each faction's plays once per pick.
    #[cfg(target_os = "macos")]
    let mut movie_screen = game.screen;
    #[cfg(target_os = "macos")]
    let mut music_playing = false;
    // When the battle track playing now is over, if one is.
    #[cfg(target_os = "macos")]
    let mut battle_track_ends: Option<Instant> = None;
    // Whether the disc's pictures, read with its models, have been given to the backend.
    #[cfg(target_os = "macos")]
    let mut pictures_given = false;
    let mut last = Instant::now();
    while !game.quit {
        while let Some(event) = platform.poll_event() {
            game.handle(event);
        }
        let now = Instant::now();
        game.update((now - last).as_secs_f32());
        last = now;
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
        // The menus' music plays over and over in the menus, and only there.
        #[cfg(target_os = "macos")]
        if movie.is_none() && !menu_music.is_empty() && game.is_in_menus() != music_playing {
            music_playing = !music_playing;
            if music_playing {
                platform.play_sound(&menu_music, video::SOUND_CHANNELS, video::SOUND_RATE, true);
            } else {
                platform.stop_sound();
            }
        }
        // A faction's movie plays when its screen opens, after the menus' music has stopped.
        #[cfg(target_os = "macos")]
        if game.screen != movie_screen {
            movie_screen = game.screen;
            if let game::Screen::Movie(faction) = game.screen
                && mounted
                && let Some(opened) = video::Movie::open(&disc.join(faction_movie(faction)))
            {
                game.start_movie();
                platform.play_sound(
                    opened.sound(),
                    video::SOUND_CHANNELS,
                    video::SOUND_RATE,
                    false,
                );
                movie = Some(opened);
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
            } else if battle_track_ends.take().is_some() && !music_playing {
                platform.stop_sound();
            }
        }
        #[cfg(target_os = "macos")]
        if !pictures_given && let Some(pictures) = render::original_pictures() {
            platform.set_pictures(pictures, render::PICTURE_SIDE as u32);
            pictures_given = true;
        }
        platform.draw(&render::build_frame(&game), render::clear_color(&game));
    }
}
