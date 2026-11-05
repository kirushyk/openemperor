//! Safe wrapper over the C shim. Mirrors `shim/oe_shim.h`; the same code serves every OS.

use std::ffi::CString;

/// A corner of a triangle, its color already multiplied by its alpha. The backends blend it
/// over what is there by adding its color to what its alpha leaves showing.
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
        Vertex {
            x,
            y,
            r: r * a,
            g: g * a,
            b: b * a,
            a,
        }
    }

    /// Light: it adds its color to what is there and covers none of it.
    pub fn light((x, y): (f32, f32), [r, g, b]: [f32; 3]) -> Self {
        Vertex {
            x,
            y,
            r,
            g,
            b,
            a: 0.0,
        }
    }
}

/// A corner of the ground, which a shader details per pixel from where it is in the world.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundVertex {
    pub x: f32,
    pub y: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
    /// How much of the light the fog of war lets through.
    pub seen: f32,
    pub world_x: f32,
    /// Its height, in a material with water how far above the surface or below it.
    pub level: f32,
    pub world_z: f32,
}

/// What the ground shader makes of the ground.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Material {
    Plain = 0,
    Water = 1,
    Sand = 2,
}

/// A frame of the battlefield or of anything else: the ground, then what is over it.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub ground: Vec<GroundVertex>,
    pub material: Material,
    /// The world as seen in a mirror at the water's surface, for the water to reflect.
    pub mirror: Vec<Vertex>,
    pub over: Vec<Vertex>,
    /// Seconds, to keep the water moving.
    pub time: f32,
    pub camera: [f32; 3],
    pub sun: [f32; 3],
    /// How water looks to a backend with no shader for it.
    pub flat_water: [f32; 3],
}

impl Frame {
    /// The frame as plain colored triangles, for a backend with no ground shader.
    #[cfg(any(test, not(target_os = "macos")))]
    pub fn flattened(&self) -> Vec<Vertex> {
        let ground = self.ground.iter().map(|v| {
            let is_water = self.material == Material::Water && v.level < 0.0;
            let [r, g, b] = if is_water {
                self.flat_water
            } else {
                [v.r, v.g, v.b]
            };
            Vertex::new((v.x, v.y), [r * v.seen, g * v.seen, b * v.seen, 1.0])
        });
        ground.chain(self.over.iter().copied()).collect()
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

/// Mirrors `OeWorld` in `shim/oe_shim.h`.
#[cfg(target_os = "macos")]
#[repr(C)]
struct RawWorld {
    ground: *const GroundVertex,
    ground_count: u32,
    mirror: *const Vertex,
    mirror_count: u32,
    over: *const Vertex,
    over_count: u32,
    material: i32,
    time: f32,
    camera: [f32; 3],
    sun: [f32; 3],
}

unsafe extern "C" {
    fn oe_init(title: *const std::ffi::c_char, width: *mut i32, height: *mut i32) -> i32;
    fn oe_poll_event(event: *mut RawEvent) -> i32;
    #[cfg(not(target_os = "macos"))]
    fn oe_draw(vertices: *const Vertex, count: u32, r: f32, g: f32, b: f32);
    #[cfg(target_os = "macos")]
    fn oe_draw_world(world: *const RawWorld, clear_r: f32, clear_g: f32, clear_b: f32);
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
        let mut raw = RawEvent {
            kind: 0,
            a: 0,
            b: 0,
            x: 0.0,
            y: 0.0,
        };
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

    #[cfg(target_os = "macos")]
    pub fn draw(&mut self, frame: &Frame, clear: [f32; 3]) {
        let world = RawWorld {
            ground: frame.ground.as_ptr(),
            ground_count: frame.ground.len() as u32,
            mirror: frame.mirror.as_ptr(),
            mirror_count: frame.mirror.len() as u32,
            over: frame.over.as_ptr(),
            over_count: frame.over.len() as u32,
            material: frame.material as i32,
            time: frame.time,
            camera: frame.camera,
            sun: frame.sun,
        };
        // SAFETY: the pointers are valid for their counts, and the shim copies them all before
        // returning.
        unsafe { oe_draw_world(&raw const world, clear[0], clear[1], clear[2]) }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn draw(&mut self, frame: &Frame, clear: [f32; 3]) {
        let vertices = frame.flattened();
        // SAFETY: the shim copies the slice before returning.
        unsafe {
            oe_draw(
                vertices.as_ptr(),
                vertices.len() as u32,
                clear[0],
                clear[1],
                clear[2],
            );
        }
    }
}

impl Drop for Platform {
    fn drop(&mut self) {
        // SAFETY: `Platform` only exists after a successful oe_init.
        unsafe { oe_shutdown() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flattened_frame_has_the_ground_under_the_rest_and_water_in_its_flat_color() {
        let corner = |level: f32| GroundVertex {
            x: 1.0,
            y: 2.0,
            r: 0.5,
            g: 0.5,
            b: 0.5,
            seen: 0.5,
            world_x: 0.0,
            level,
            world_z: 0.0,
        };
        let over = Vertex::new((3.0, 4.0), [1.0, 1.0, 1.0, 1.0]);
        let frame = Frame {
            ground: vec![corner(-1.0), corner(1.0)],
            material: Material::Water,
            mirror: Vec::new(),
            over: vec![over],
            time: 0.0,
            camera: [0.0; 3],
            sun: [0.0; 3],
            flat_water: [0.0, 0.0, 1.0],
        };
        let flat = frame.flattened();
        assert_eq!(flat.len(), 3);
        assert_eq!([flat[0].r, flat[0].b, flat[0].a], [0.0, 0.5, 1.0]);
        assert_eq!([flat[1].r, flat[1].b], [0.25, 0.25]);
        assert_eq!(flat[2], over);
    }

    #[test]
    fn a_vertex_carries_its_color_multiplied_by_its_alpha_and_light_covers_nothing() {
        let v = Vertex::new((0.0, 0.0), [1.0, 0.5, 0.0, 0.5]);
        assert_eq!([v.r, v.g, v.b, v.a], [0.5, 0.25, 0.0, 0.5]);
        let v = Vertex::light((0.0, 0.0), [0.3, 0.3, 0.3]);
        assert_eq!([v.r, v.g, v.b, v.a], [0.3, 0.3, 0.3, 0.0]);
    }
}
