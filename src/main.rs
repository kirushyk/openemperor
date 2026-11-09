#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod battlefield;
mod fog;
mod font;
mod game;
mod hud;
mod planet;
mod platform;
mod render;
mod strategy;
mod units;
mod weapons;

use std::time::Instant;

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
    let mut last = Instant::now();
    while !game.quit {
        while let Some(event) = platform.poll_event() {
            game.handle(event);
        }
        let now = Instant::now();
        game.update((now - last).as_secs_f32());
        last = now;
        platform.draw(&render::build_frame(&game), render::clear_color(&game));
    }
}
