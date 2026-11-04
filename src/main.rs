#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod battlefield;
mod font;
mod game;
mod hud;
mod planet;
mod platform;
mod render;
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

    let mut game = game::Game::new(width as f32, height as f32);
    let mut last = Instant::now();
    while !game.quit {
        while let Some(event) = platform.poll_event() {
            game.handle(event);
        }
        let now = Instant::now();
        game.update((now - last).as_secs_f32());
        last = now;
        platform.draw(&render::build(&game), render::clear_color(&game));
    }
}
