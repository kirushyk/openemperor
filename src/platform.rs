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

/// A corner of something solid in the world. The backend places it through the camera's matrix,
/// lights it by its normal and keeps what is nearest by depth.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldVertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub normal: [f32; 3],
    pub r: f32,
    pub g: f32,
    pub b: f32,
    /// How much of the light the fog of war lets through.
    pub seen: f32,
    /// On the ground, its height; where a liquid lies, how far under its surface it is, below 0.
    /// On an object, how much it shines: 0 for not at all, through paint, to 1 for bare metal.
    /// On a decal, how much of its picture shows; below 0, a light's glow, adding as much of
    /// its picture to what lies behind it.
    pub level: f32,
    /// Where on its unit it is, for the backend to lay black stains by, and how black they go.
    /// Below 0, the unit is invisible to its enemies and shimmers for its own side.
    pub stain: [f32; 3],
    /// Where on a picture given to `Platform::set_pictures` it is, and which: below 0, none,
    /// and it takes its color.
    pub picture: [f32; 3],
    /// The house color that its picture's house parts take.
    pub team: [f32; 3],
}

/// What a corner with no picture has for one.
pub const NO_PICTURE: [f32; 3] = [0.0, 0.0, -1.0];

impl WorldVertex {
    /// A corner of an object standing on the map, not shining.
    pub fn solid(point: (f32, f32, f32), normal: [f32; 3], color: [f32; 3]) -> Self {
        WorldVertex::shiny(point, normal, color, 0.0)
    }

    /// A corner of an object standing on the map, shining by `shine`.
    pub fn shiny(
        (x, y, z): (f32, f32, f32),
        normal: [f32; 3],
        [r, g, b]: [f32; 3],
        shine: f32,
    ) -> Self {
        WorldVertex {
            x,
            y,
            z,
            normal,
            r,
            g,
            b,
            seen: 1.0,
            level: shine,
            stain: [0.0; 3],
            picture: NO_PICTURE,
            team: [0.0; 3],
        }
    }
}

/// A corner of something see-through in the world: placed on the screen already, with the depth
/// the backend tests it against what is solid. Its color is multiplied by its alpha.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EffectVertex {
    pub x: f32,
    pub y: f32,
    pub depth: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl EffectVertex {
    pub fn of(v: Vertex, depth: f32) -> Self {
        EffectVertex {
            x: v.x,
            y: v.y,
            depth,
            r: v.r,
            g: v.g,
            b: v.b,
            a: v.a,
        }
    }
}

/// A light that burns for a moment somewhere on the map, for the backend to light things with.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointLight {
    pub place: [f32; 3],
    pub radius: f32,
    pub color: [f32; 3],
    pub unused: f32,
}

/// How many lights the backends take. Further ones are left out.
pub const MAX_LIGHTS: usize = 16;

/// What the ground shader makes of the ground.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Material {
    /// Only what the ground's own pictures show, lit.
    Pictured = 0,
}

/// A frame: the world, for the backend to place, light and shade, then what goes on the screen.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub ground: Vec<WorldVertex>,
    /// What lies on the ground and lets it show through, unlit, and lights' glows: each corner
    /// as much of its picture as its level says.
    pub decals: Vec<WorldVertex>,
    pub material: Material,
    /// Everything solid standing on the map.
    pub objects: Vec<WorldVertex>,
    /// What casts a shadow without being drawn as solid.
    pub casters: Vec<WorldVertex>,
    pub effects: Vec<EffectVertex>,
    /// What goes over all of it, on the screen.
    pub over: Vec<Vertex>,
    pub lights: Vec<PointLight>,
    /// Seconds, to keep the ground moving.
    pub time: f32,
    pub view: [f32; 16],
    /// The same view with everything seen in a mirror at height 0.
    pub mirror: [f32; 16],
    /// The world as the sun sees it, for shadows.
    pub sun_view: [f32; 16],
    pub camera: [f32; 3],
    pub sun: [f32; 3],
    /// The light everything gets from all around, and what a face turned to the sun gets.
    pub ambient: [f32; 3],
    pub sunlight: [f32; 3],
    /// How far along x and z the picture given to `Platform::set_wash` reaches over the
    /// ground; 0 for none.
    pub wash: [f32; 2],
    pub size: (f32, f32),
}

impl Frame {
    /// The frame as plain colored triangles in the order given, placed and lit here, for a
    /// backend with no world pipeline. It has no depth and no shadows, and no decals, which
    /// are only their pictures.
    #[cfg(any(test, not(target_os = "macos")))]
    pub fn flattened(&self) -> Vec<Vertex> {
        let place = |v: &WorldVertex| {
            let m = &self.view;
            let w = m[3] * v.x + m[7] * v.y + m[11] * v.z + m[15];
            let x = (m[0] * v.x + m[4] * v.y + m[8] * v.z + m[12]) / w;
            let y = (m[1] * v.x + m[5] * v.y + m[9] * v.z + m[13]) / w;
            (w > 0.0).then_some(((x + 1.0) / 2.0 * self.size.0, (1.0 - y) / 2.0 * self.size.1))
        };
        let shade = |v: &WorldVertex| {
            let facing: f32 = (0..3).map(|i| v.normal[i] * self.sun[i]).sum();
            let light = (0.45 + 0.55 * facing.max(0.0)) * v.seen;
            [v.r * light, v.g * light, v.b * light, 1.0]
        };
        let mut flat = Vec::new();
        for triangle in self.ground.chunks(3).chain(self.objects.chunks(3)) {
            let placed: Option<Vec<(f32, f32)>> = triangle.iter().map(place).collect();
            if let Some(placed) = placed {
                flat.extend(
                    placed
                        .iter()
                        .zip(triangle)
                        .map(|(&point, v)| Vertex::new(point, shade(v))),
                );
            }
        }
        flat.extend(self.effects.iter().map(|v| Vertex {
            x: v.x,
            y: v.y,
            r: v.r,
            g: v.g,
            b: v.b,
            a: v.a,
        }));
        flat.extend(self.over.iter().copied());
        flat
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

/// A modifier held while a number key is pressed. Control wins if both are down.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Modifier {
    Shift,
    Control,
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
    /// 1 to 9 pressed on the number row, as well as the `KeyDown` for the same press.
    NumberKey(usize, Option<Modifier>),
}

/// Mirrors `OeWorld` in `shim/oe_shim.h`.
#[cfg(target_os = "macos")]
#[repr(C)]
struct RawWorld {
    ground: *const WorldVertex,
    ground_count: u32,
    decals: *const WorldVertex,
    decals_count: u32,
    objects: *const WorldVertex,
    objects_count: u32,
    casters: *const WorldVertex,
    casters_count: u32,
    effects: *const EffectVertex,
    effects_count: u32,
    over: *const Vertex,
    over_count: u32,
    lights: *const PointLight,
    lights_count: u32,
    material: i32,
    time: f32,
    view: [f32; 16],
    mirror: [f32; 16],
    sun_view: [f32; 16],
    camera: [f32; 3],
    sun: [f32; 3],
    ambient: [f32; 3],
    sunlight: [f32; 3],
    wash: [f32; 2],
}

unsafe extern "C" {
    fn oe_init(title: *const std::ffi::c_char, width: *mut i32, height: *mut i32) -> i32;
    fn oe_poll_event(event: *mut RawEvent) -> i32;
    #[cfg(not(target_os = "macos"))]
    fn oe_draw(vertices: *const Vertex, count: u32, r: f32, g: f32, b: f32);
    #[cfg(target_os = "macos")]
    fn oe_draw_world(world: *const RawWorld, clear_r: f32, clear_g: f32, clear_b: f32);
    fn oe_set_backdrop(rgba: *const u8, width: i32, height: i32);
    fn oe_set_pictures(rgba: *const u8, side: i32, count: i32);
    fn oe_set_wash(rgba: *const u8, width: i32, height: i32);
    fn oe_play_sound(samples: *const f32, frames: u32, channels: i32, rate: i32, looped: i32);
    fn oe_stop_sound();
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
                11 => Event::NumberKey(
                    raw.a as usize,
                    match raw.b {
                        1 => Some(Modifier::Shift),
                        2 => Some(Modifier::Control),
                        _ => None,
                    },
                ),
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
            decals: frame.decals.as_ptr(),
            decals_count: frame.decals.len() as u32,
            objects: frame.objects.as_ptr(),
            objects_count: frame.objects.len() as u32,
            casters: frame.casters.as_ptr(),
            casters_count: frame.casters.len() as u32,
            effects: frame.effects.as_ptr(),
            effects_count: frame.effects.len() as u32,
            over: frame.over.as_ptr(),
            over_count: frame.over.len() as u32,
            lights: frame.lights.as_ptr(),
            lights_count: frame.lights.len().min(MAX_LIGHTS) as u32,
            material: frame.material as i32,
            time: frame.time,
            view: frame.view,
            mirror: frame.mirror,
            sun_view: frame.sun_view,
            camera: frame.camera,
            sun: frame.sun,
            ambient: frame.ambient,
            sunlight: frame.sunlight,
            wash: frame.wash,
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

/// A picture of rows of RGBA bytes, top row first.
pub struct Image<'a> {
    pub rgba: &'a [u8],
    pub width: u32,
    pub height: u32,
}

impl Platform {
    /// Shows a picture under all that is drawn from now on, as large as fits the screen, or
    /// takes it away.
    pub fn set_backdrop(&mut self, image: Option<Image>) {
        match image {
            Some(image) if image.rgba.len() >= (image.width * image.height * 4) as usize => {
                // SAFETY: the bytes hold the whole picture, and the shim copies them.
                unsafe {
                    oe_set_backdrop(image.rgba.as_ptr(), image.width as i32, image.height as i32)
                }
            }
            // SAFETY: a null picture takes the backdrop away.
            _ => unsafe { oe_set_backdrop(std::ptr::null(), 0, 0) },
        }
    }

    /// Gives the pictures objects are painted with: squares `side` pixels wide, one after
    /// another, each of rows of RGBA bytes, top row first. Alpha 0 is a hole; from 128 up it
    /// says how much house color shows there, none at 255.
    pub fn set_pictures(&mut self, rgba: &[u8], side: u32) {
        let square = (side * side * 4) as usize;
        if square == 0 || rgba.len() < square {
            return;
        }
        let count = rgba.len() / square;
        // SAFETY: the bytes hold `count` whole squares, and the shim copies them.
        unsafe { oe_set_pictures(rgba.as_ptr(), side as i32, count as i32) }
    }

    /// The picture laid over the ground, rows of RGBA from the top: each pixel's color, and in
    /// its alpha how much of it shows over the ground lit. An empty one takes it away.
    pub fn set_wash(&mut self, rgba: &[u8], width: u32, height: u32) {
        let pixels = (width * height * 4) as usize;
        let rgba = if pixels > 0 && rgba.len() >= pixels {
            rgba.as_ptr()
        } else {
            std::ptr::null()
        };
        // SAFETY: the bytes hold `width` by `height` pixels or none are given, and the shim
        // copies them.
        unsafe { oe_set_wash(rgba, width as i32, height as i32) }
    }

    /// Plays interleaved samples, once or over and over, in place of whatever was playing.
    pub fn play_sound(&mut self, samples: &[f32], channels: u32, rate: u32, looped: bool) {
        let frames = samples.len() / channels.max(1) as usize;
        // SAFETY: the samples hold `frames` frames of `channels`, and the shim copies them.
        unsafe {
            oe_play_sound(
                samples.as_ptr(),
                frames as u32,
                channels as i32,
                rate as i32,
                i32::from(looped),
            )
        }
    }

    pub fn stop_sound(&mut self) {
        // SAFETY: stopping is always safe, playing or not.
        unsafe { oe_stop_sound() }
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
    fn a_flattened_frame_places_the_world_under_the_rest() {
        let mut identity = [0.0; 16];
        for i in [0, 5, 10, 15] {
            identity[i] = 1.0;
        }
        let corner =
            |blue: f32| WorldVertex::solid((0.0, 0.0, 0.0), [0.0, 1.0, 0.0], [0.5, 0.5, blue]);
        let over = Vertex::new((3.0, 4.0), [1.0, 1.0, 1.0, 1.0]);
        let frame = Frame {
            ground: vec![corner(1.0), corner(1.0), corner(0.5)],
            decals: Vec::new(),
            material: Material::Pictured,
            objects: Vec::new(),
            casters: Vec::new(),
            effects: Vec::new(),
            over: vec![over],
            lights: Vec::new(),
            time: 0.0,
            view: identity,
            mirror: identity,
            sun_view: identity,
            camera: [0.0; 3],
            sun: [0.0, 1.0, 0.0],
            ambient: [0.5; 3],
            sunlight: [0.5; 3],
            wash: [0.0; 2],
            size: (100.0, 100.0),
        };
        let flat = frame.flattened();
        assert_eq!(flat.len(), 4);
        assert_eq!(
            (flat[0].x, flat[0].y),
            (50.0, 50.0),
            "the middle of the screen"
        );
        assert_eq!([flat[0].r, flat[0].b], [0.5, 1.0]);
        assert_eq!(flat[2].b, 0.5);
        assert_eq!(flat[3], over);
    }

    #[test]
    fn a_vertex_carries_its_color_multiplied_by_its_alpha_and_light_covers_nothing() {
        let v = Vertex::new((0.0, 0.0), [1.0, 0.5, 0.0, 0.5]);
        assert_eq!([v.r, v.g, v.b, v.a], [0.5, 0.25, 0.0, 0.5]);
        let v = Vertex::light((0.0, 0.0), [0.3, 0.3, 0.3]);
        assert_eq!([v.r, v.g, v.b, v.a], [0.3, 0.3, 0.3, 0.0]);
    }
}
