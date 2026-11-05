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
    pub level: f32,
    /// Where on its unit it is, for the backend to lay black stains by, and how black they go.
    pub stain: [f32; 3],
}

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
    Plain = 0,
    Water = 1,
    Sand = 2,
    Lava = 3,
}

/// A frame: the world, for the backend to place, light and shade, then what goes on the screen.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub ground: Vec<WorldVertex>,
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
    /// How a liquid looks to a backend with no shader for it.
    pub flat_liquid: [f32; 3],
    pub size: (f32, f32),
}

impl Frame {
    /// The frame as plain colored triangles in the order given, placed and lit here, for a
    /// backend with no world pipeline. It has no depth and no shadows.
    #[cfg(any(test, not(target_os = "macos")))]
    pub fn flattened(&self) -> Vec<Vertex> {
        let has_liquid = matches!(self.material, Material::Water | Material::Lava);
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
            let [r, g, b] = if has_liquid && v.level < 0.0 {
                self.flat_liquid
            } else {
                [v.r, v.g, v.b]
            };
            [r * light, g * light, b * light, 1.0]
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
    ground: *const WorldVertex,
    ground_count: u32,
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
    fn a_flattened_frame_places_the_world_under_the_rest_with_liquid_in_its_flat_color() {
        let mut identity = [0.0; 16];
        for i in [0, 5, 10, 15] {
            identity[i] = 1.0;
        }
        let corner = |level: f32| WorldVertex {
            level,
            ..WorldVertex::solid((0.0, 0.0, 0.0), [0.0, 1.0, 0.0], [0.5, 0.5, 0.5])
        };
        let over = Vertex::new((3.0, 4.0), [1.0, 1.0, 1.0, 1.0]);
        let frame = Frame {
            ground: vec![corner(-1.0), corner(-1.0), corner(1.0)],
            material: Material::Water,
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
            flat_liquid: [0.0, 0.0, 1.0],
            size: (100.0, 100.0),
        };
        let flat = frame.flattened();
        assert_eq!(flat.len(), 4);
        assert_eq!(
            (flat[0].x, flat[0].y),
            (50.0, 50.0),
            "the middle of the screen"
        );
        assert_eq!([flat[0].r, flat[0].b], [0.0, 1.0]);
        assert_eq!(flat[2].r, 0.5);
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
