//! Safe wrapper over the C shim. Mirrors `shim/oe_shim.h`; the same code serves every OS.

use std::ffi::CString;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    pub x: f32,
    pub y: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Vertex {
    pub fn new((x, y): (f32, f32), [r, g, b, a]: [f32; 4]) -> Self {
        Vertex { x, y, r, g, b, a }
    }
}

#[repr(C)]
struct RawEvent {
    kind: i32,
    a: i32,
    b: i32,
    x: f32,
    y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Key {
    Escape,
    Backspace,
    Enter,
    Tab,
    /// Pause/Break, which Mac keyboards do not have.
    Pause,
    /// A printable ASCII character.
    Char(char),
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    Quit,
    Resize(i32, i32),
    MouseMove(f32, f32),
    MouseDown,
    MouseUp,
    /// The left button let go with Shift held.
    ShiftMouseUp,
    KeyDown(Key),
    Focus(bool),
    /// Notches turned, positive away from the player.
    Wheel(f32),
    /// The right button, or a two-finger click or tap on a Mac trackpad.
    RightDown,
    /// A Control key pressed or let go.
    Control(bool),
}

unsafe extern "C" {
    fn oe_init(title: *const std::ffi::c_char, width: *mut i32, height: *mut i32) -> i32;
    fn oe_poll_event(event: *mut RawEvent) -> i32;
    fn oe_draw(vertices: *const Vertex, count: u32, r: f32, g: f32, b: f32);
    fn oe_shutdown();
}

pub struct Platform;

impl Platform {
    /// Opens the fullscreen window and returns it with its size in pixels.
    pub fn new(title: &str) -> Result<(Self, i32, i32), String> {
        let title = CString::new(title).map_err(|e| e.to_string())?;
        let (mut width, mut height) = (0, 0);
        // SAFETY: the pointers are valid for the duration of the call.
        let ok = unsafe { oe_init(title.as_ptr(), &raw mut width, &raw mut height) };
        if ok == 0 {
            return Err("could not create the fullscreen window".into());
        }
        Ok((Platform, width, height))
    }

    pub fn poll_event(&mut self) -> Option<Event> {
        let mut raw = RawEvent { kind: 0, a: 0, b: 0, x: 0.0, y: 0.0 };
        loop {
            // SAFETY: `raw` is a valid, correctly laid out OeEvent.
            if unsafe { oe_poll_event(&raw mut raw) } == 0 {
                return None;
            }
            let event = match raw.kind {
                1 => Event::Quit,
                2 => Event::Resize(raw.a, raw.b),
                3 => Event::MouseMove(raw.x, raw.y),
                4 => Event::MouseDown,
                5 if raw.a != 0 => Event::ShiftMouseUp,
                5 => Event::MouseUp,
                6 => Event::KeyDown(match (raw.a, raw.b) {
                    (1, _) => Key::Escape,
                    (2, _) => Key::Backspace,
                    (3, _) => Key::Enter,
                    (4, _) => Key::Tab,
                    (5, _) => Key::Pause,
                    (_, 0x20..=0x7E) => Key::Char(raw.b as u8 as char),
                    _ => Key::Other,
                }),
                7 => Event::Focus(raw.a != 0),
                8 => Event::Wheel(raw.y),
                9 => Event::RightDown,
                10 => Event::Control(raw.a != 0),
                _ => continue,
            };
            return Some(event);
        }
    }

    pub fn draw(&mut self, vertices: &[Vertex], clear: [f32; 3]) {
        // SAFETY: the shim copies the slice before returning.
        unsafe {
            oe_draw(vertices.as_ptr(), vertices.len() as u32, clear[0], clear[1], clear[2]);
        }
    }
}

impl Drop for Platform {
    fn drop(&mut self) {
        // SAFETY: `Platform` only exists after a successful oe_init.
        unsafe { oe_shutdown() }
    }
}
