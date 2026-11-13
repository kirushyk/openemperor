//! The original game's models, in its XBF files, and the TGA pictures they are painted with.

/// A model: the pictures its faces are painted with, and its objects, each with its own.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Model {
    pub textures: Vec<String>,
    pub objects: Vec<Object>,
    pub sequences: Vec<Sequence>,
}

/// A named run of the model's animation, such as "Move" or "Fire 0": the frames it plays,
/// from the first of each range up to its last, one range after another.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sequence {
    pub name: String,
    pub ranges: Vec<(usize, usize)>,
}

/// A part of a model, placed by its transform within its parent, given for row vectors: a
/// point times it is where it is in the parent. For a part that moves, it is where the first
/// frame of its animation puts it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Object {
    pub name: String,
    pub transform: [[f32; 4]; 4],
    pub animation: Option<Animation>,
    pub vertices: Vec<Vertex>,
    pub faces: Vec<Face>,
    pub children: Vec<Object>,
}

/// Where a part is at each frame of its model's animation: which of its poses it takes then.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Animation {
    pub poses: Vec<[[f32; 4]; 4]>,
    pub frames: Vec<u16>,
}

impl Animation {
    /// Its pose at a frame, the last one's for those past its end.
    pub fn at(&self, frame: usize) -> Option<&[[f32; 4]; 4]> {
        let pose = self.frames.get(frame).or(self.frames.last())?;
        self.poses.get(usize::from(*pose))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

/// A triangle: its corners' vertices, the picture it is painted with, if any, and where on
/// that picture each corner is, from its top left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Face {
    pub corners: [usize; 3],
    pub texture: Option<usize>,
    pub uv: [[f32; 2]; 3],
}

const VERSION: i32 = 1;
/// What an object carries after its faces.
const VERTEX_LIGHTS: i32 = 1;
const FACE_FLAGS: i32 = 2;
const VERTEX_ANIMATION: i32 = 4;
const OBJECT_ANIMATION: i32 = 8;
/// How deep objects may sit inside one another before the file is taken to be broken.
const DEEPEST: usize = 64;

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn bytes(&mut self, count: usize) -> Option<&[u8]> {
        let bytes = self.data.get(self.at..self.at.checked_add(count)?)?;
        self.at += count;
        Some(bytes)
    }

    fn skip(&mut self, count: usize) -> Option<()> {
        self.bytes(count).map(|_| ())
    }

    fn i32(&mut self) -> Option<i32> {
        Some(i32::from_le_bytes(self.bytes(4)?.try_into().ok()?))
    }

    fn i16(&mut self) -> Option<i16> {
        Some(i16::from_le_bytes(self.bytes(2)?.try_into().ok()?))
    }

    fn f32(&mut self) -> Option<f32> {
        Some(f32::from_le_bytes(self.bytes(4)?.try_into().ok()?))
    }

    fn f64(&mut self) -> Option<f64> {
        Some(f64::from_le_bytes(self.bytes(8)?.try_into().ok()?))
    }

    /// A count, which must not be negative.
    fn count(&mut self) -> Option<usize> {
        usize::try_from(self.i32()?).ok()
    }
}

/// The model in an XBF file, if it is one.
pub fn parse(data: &[u8]) -> Option<Model> {
    let reader = &mut Reader { data, at: 0 };
    if reader.i32()? != VERSION {
        return None;
    }
    let effects = reader.count()?;
    let sequences = sequences(reader.bytes(effects)?);
    let names = reader.count()?;
    // Some names have a 2 in front of them; empty ones stand between the rest.
    let textures = reader
        .bytes(names)?
        .split(|&byte| byte == 0)
        .map(|name| {
            name.iter()
                .filter(|&&byte| byte != 2)
                .map(|&byte| char::from(byte))
        })
        .map(String::from_iter)
        .filter(|name| !name.is_empty())
        .collect();
    let mut objects = Vec::new();
    while reader.data.get(reader.at..reader.at + 4)? != (-1i32).to_le_bytes() {
        objects.push(object(reader, 0)?);
    }
    Some(Model {
        textures,
        objects,
        sequences,
    })
}

/// How long a sequence's name is kept in its list, and each of its ranges: three numbers and a
/// flag, three bytes of padding, then its first and its last frame.
const SEQUENCE_NAME: usize = 32;
const SEQUENCE_RANGE: usize = 24;
const PADDING: [u8; 3] = [0xcc; 3];

/// The sequences listed at the end of a model's effects, before its last four bytes. Where the
/// list starts is not said: it is the first place after a string's end that all the rest reads
/// as one.
fn sequences(effects: &[u8]) -> Vec<Sequence> {
    (1..effects.len())
        .filter(|&at| effects[at - 1] == 0 && effects[at].is_ascii_graphic())
        .find_map(|at| sequence_list(effects, at))
        .unwrap_or_default()
}

/// The list read from `at` on, each a name, a number, and its ranges, as long as the next does
/// not start with a letter of another name.
fn sequence_list(effects: &[u8], mut at: usize) -> Option<Vec<Sequence>> {
    let end = effects.len().checked_sub(4)?;
    let mut list = Vec::new();
    while at < end {
        let name = effects.get(at..at + SEQUENCE_NAME)?;
        let name = &name[..name.iter().position(|&byte| byte == 0)?];
        if !name
            .iter()
            .all(|byte| byte.is_ascii_graphic() || *byte == b' ')
        {
            return None;
        }
        at += SEQUENCE_NAME + 4;
        let mut ranges = Vec::new();
        loop {
            let range = effects
                .get(at..at + SEQUENCE_RANGE)
                .filter(|_| at + SEQUENCE_RANGE <= end)?;
            let frame = |n: usize| u32::from_le_bytes(range[n..n + 4].try_into().unwrap());
            let (first, last) = (frame(16), frame(20));
            if range[13..16] != PADDING || first > last {
                return None;
            }
            ranges.push((first as usize, last as usize));
            at += SEQUENCE_RANGE;
            if at >= end || effects[at].is_ascii_graphic() {
                break;
            }
        }
        list.push(Sequence {
            name: name.iter().map(|&byte| char::from(byte)).collect(),
            ranges,
        });
    }
    (at == end && !list.is_empty()).then_some(list)
}

fn object(reader: &mut Reader, depth: usize) -> Option<Object> {
    if depth > DEEPEST {
        return None;
    }
    let vertex_count = reader.count()?;
    let flags = reader.i32()?;
    let face_count = reader.count()?;
    let child_count = reader.count()?;
    let mut transform = [[0.0; 4]; 4];
    for row in &mut transform {
        for cell in row {
            *cell = reader.f64()? as f32;
        }
    }
    let length = reader.count()?;
    let name = reader.bytes(length)?;
    let name = name.split(|&byte| byte == 0).next().unwrap_or(name);
    let name = name.iter().map(|&byte| char::from(byte)).collect();
    let children = (0..child_count)
        .map(|_| object(reader, depth + 1))
        .collect::<Option<Vec<_>>>()?;
    let mut vertices = Vec::with_capacity(vertex_count.min(reader.data.len() / 24));
    for _ in 0..vertex_count {
        let mut six = [0.0; 6];
        for value in &mut six {
            *value = reader.f32()?;
        }
        vertices.push(Vertex {
            position: [six[0], six[1], six[2]],
            normal: [six[3], six[4], six[5]],
        });
    }
    let mut faces = Vec::with_capacity(face_count.min(reader.data.len() / 44));
    for _ in 0..face_count {
        let corners = [reader.count()?, reader.count()?, reader.count()?];
        let texture = usize::try_from(reader.i32()?).ok();
        let _smoothing = reader.i32()?;
        let mut uv = [[0.0; 2]; 3];
        for corner in &mut uv {
            *corner = [reader.f32()?, reader.f32()?];
        }
        if corners.iter().all(|&corner| corner < vertex_count) {
            faces.push(Face {
                corners,
                texture,
                uv,
            });
        }
    }
    if flags & VERTEX_LIGHTS != 0 {
        reader.skip(3 * vertex_count)?;
    }
    if flags & FACE_FLAGS != 0 {
        reader.skip(4 * face_count)?;
    }
    if flags & VERTEX_ANIMATION != 0 {
        skip_vertex_animation(reader)?;
    }
    let mut animation = None;
    if flags & OBJECT_ANIMATION != 0 {
        animation = object_animation(reader)?;
    }
    if let Some(first) = animation.as_ref().and_then(|animation| animation.at(0)) {
        transform = *first;
    }
    Some(Object {
        name,
        transform,
        animation,
        vertices,
        faces,
        children,
    })
}

/// Vertices moved frame by frame: only the frames' numbers, unless a negative count says
/// eight bytes for each vertex of each frame follow, and then perhaps a map of every frame.
fn skip_vertex_animation(reader: &mut Reader) -> Option<()> {
    let length = reader.count()?;
    let negative_entries = reader.i32()?;
    let frames = reader.count()?;
    reader.skip(4 * frames)?;
    if negative_entries >= 0 {
        return Some(());
    }
    let _kind = reader.i16()?;
    let mapped = reader.i16()? != 0;
    let entries = reader.count()?;
    reader.skip(8 * entries)?;
    if mapped {
        reader.skip(4 * length)?;
    }
    Some(())
}

/// Where the object's animation has it at each frame, if it says: it gives a full or a
/// shortened matrix for every frame, shortened ones picked out for each frame, or keys each
/// with a turn, a stretch and a move, as flagged, a frame taking the latest key up to it and the
/// earliest standing for the frames before.
fn object_animation(reader: &mut Reader) -> Option<Option<Animation>> {
    let length = reader.count()? + 1;
    let every_frame = |reader: &mut Reader, columns: usize| -> Option<Animation> {
        Some(Animation {
            poses: (0..length)
                .map(|_| matrix(reader, columns))
                .collect::<Option<_>>()?,
            frames: (0..length)
                .map(|n| u16::try_from(n).ok())
                .collect::<Option<_>>()?,
        })
    };
    let animation = match reader.i32()? {
        -1 => every_frame(reader, 4)?,
        -2 => every_frame(reader, 3)?,
        -3 => {
            let count = reader.count()?;
            let frames: Vec<u16> = (0..length)
                .map(|_| {
                    u16::try_from(reader.i16()?)
                        .ok()
                        .filter(|&n| usize::from(n) < count)
                })
                .collect::<Option<_>>()?;
            let poses = (0..count)
                .map(|_| matrix(reader, 3))
                .collect::<Option<_>>()?;
            Animation { poses, frames }
        }
        keys => {
            let mut keyed = Vec::new();
            for _ in 0..usize::try_from(keys).ok()? {
                let frame = reader.i16()?;
                let parts = reader.i16()? >> 12;
                keyed.push((frame, key_matrix(reader, parts)?));
            }
            if keyed.is_empty() {
                return Some(None);
            }
            keyed.sort_by_key(|&(frame, _)| frame);
            let frames = (0..length)
                .map(|n| {
                    let after = keyed.partition_point(|&(frame, _)| i64::from(frame) <= n as i64);
                    u16::try_from(after.saturating_sub(1)).ok()
                })
                .collect::<Option<_>>()?;
            let poses = keyed.into_iter().map(|(_, pose)| pose).collect();
            Animation { poses, frames }
        }
    };
    Some(Some(animation))
}

/// A matrix of four rows, each of `columns` numbers, the rest of it as the identity's.
fn matrix(reader: &mut Reader, columns: usize) -> Option<[[f32; 4]; 4]> {
    let mut matrix = [[0.0, 0.0, 0.0, 0.0]; 4];
    matrix[3][3] = 1.0;
    for row in &mut matrix {
        for cell in &mut row[..columns] {
            *cell = reader.f32()?;
        }
    }
    Some(matrix)
}

/// An animation key's matrix: a turn by a quaternion, then a stretch, then a move, each there
/// if its flag is.
fn key_matrix(reader: &mut Reader, parts: i16) -> Option<[[f32; 4]; 4]> {
    let three = |reader: &mut Reader, flag: i16, default: f32| -> Option<[f32; 3]> {
        if parts & flag == 0 {
            return Some([default; 3]);
        }
        Some([reader.f32()?, reader.f32()?, reader.f32()?])
    };
    let mut turn = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    if parts & 1 != 0 {
        let [x, y, z] = three(reader, 1, 0.0)?;
        let w = reader.f32()?;
        turn = [
            [
                1.0 - 2.0 * (y * y + z * z),
                2.0 * (x * y + z * w),
                2.0 * (x * z - y * w),
            ],
            [
                2.0 * (x * y - z * w),
                1.0 - 2.0 * (x * x + z * z),
                2.0 * (y * z + x * w),
            ],
            [
                2.0 * (x * z + y * w),
                2.0 * (y * z - x * w),
                1.0 - 2.0 * (x * x + y * y),
            ],
        ];
    }
    let stretch = three(reader, 2, 1.0)?;
    let [x, y, z] = three(reader, 4, 0.0)?;
    let row = |r: usize| {
        let [a, b, c] = turn[r];
        [a * stretch[0], b * stretch[1], c * stretch[2], 0.0]
    };
    Some([row(0), row(1), row(2), [x, y, z, 1.0]])
}

/// A picture, its pixels in rows from the top, each red, green, blue and how opaque.
#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<[u8; 4]>,
}

impl Image {
    /// The pixel at a place on the picture given from its top left as shares of its width and
    /// height, repeated past its edges.
    pub fn at(&self, [u, v]: [f32; 2]) -> [u8; 4] {
        let wrap = |share: f32, size: usize| {
            ((share.rem_euclid(1.0) * size as f32) as usize).min(size - 1)
        };
        self.pixels[wrap(v, self.height) * self.width + wrap(u, self.width)]
    }
}

/// The picture in an uncompressed true-color TGA file, if it is one.
pub fn tga(data: &[u8]) -> Option<Image> {
    const TRUE_COLOR: u8 = 2;
    const TOP_FIRST: u8 = 0x20;
    let head = data.get(..18)?;
    if head[1] != 0 || head[2] != TRUE_COLOR {
        return None;
    }
    let width = usize::from(u16::from_le_bytes([head[12], head[13]]));
    let height = usize::from(u16::from_le_bytes([head[14], head[15]]));
    let (depth, descriptor) = (head[16], head[17]);
    let has_alpha = descriptor & 0x0f != 0;
    let size = usize::from(depth / 8);
    if width == 0 || height == 0 || !matches!(depth, 16 | 24 | 32) {
        return None;
    }
    let start = 18 + usize::from(head[0]);
    let body = data.get(start..start + width * height * size)?;
    let pixel = |bytes: &[u8]| match bytes {
        &[low, high] => {
            let value = u16::from_le_bytes([low, high]);
            let five = |shift: u16| ((value >> shift & 31) * 255 / 31) as u8;
            [five(10), five(5), five(0), 255]
        }
        &[b, g, r] => [r, g, b, 255],
        &[b, g, r, a] => [r, g, b, if has_alpha { a } else { 255 }],
        _ => [0, 0, 0, 255],
    };
    let rows: Vec<&[u8]> = body.chunks_exact(width * size).collect();
    let pixels = if descriptor & TOP_FIRST != 0 {
        rows.into_iter()
            .flat_map(|row| row.chunks_exact(size).map(pixel))
            .collect()
    } else {
        rows.into_iter()
            .rev()
            .flat_map(|row| row.chunks_exact(size).map(pixel))
            .collect()
    };
    Some(Image {
        width,
        height,
        pixels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn object_bytes(name: &str, vertices: &[[f32; 3]], faces: &[[i32; 4]], flags: i32) -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in [vertices.len() as i32, flags, faces.len() as i32, 0] {
            bytes.extend(value.to_le_bytes());
        }
        for n in 0..16 {
            let identity = if n % 5 == 0 { 1.0 } else { 0.0 };
            bytes.extend(f64::to_le_bytes(identity));
        }
        bytes.extend((name.len() as i32 + 1).to_le_bytes());
        bytes.extend(name.as_bytes());
        bytes.push(0);
        for [x, y, z] in vertices {
            for value in [*x, *y, *z, 0.0, 1.0, 0.0] {
                bytes.extend(value.to_le_bytes());
            }
        }
        for face in faces {
            for value in face {
                bytes.extend(value.to_le_bytes());
            }
            bytes.extend(0i32.to_le_bytes());
            for value in [0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0] {
                bytes.extend(value.to_le_bytes());
            }
        }
        bytes
    }

    #[test]
    fn a_model_gives_its_textures_and_its_objects_past_their_animations() {
        let mut bytes = Vec::new();
        bytes.extend(VERSION.to_le_bytes());
        bytes.extend(0i32.to_le_bytes());
        let names = b"\x02=paint.tga\0\0rust.tga\0";
        bytes.extend((names.len() as i32).to_le_bytes());
        bytes.extend(names);
        let triangle = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        bytes.extend(object_bytes(
            "hull",
            &triangle,
            &[[0, 1, 2, 1]],
            OBJECT_ANIMATION,
        ));
        // Two frames, each a shortened matrix, the first moving it up by 5.
        bytes.extend(1i32.to_le_bytes());
        bytes.extend((-2i32).to_le_bytes());
        for value in [
            1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 5.0, 0.0,
        ] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend([0; 48]);
        bytes.extend(object_bytes("lid", &triangle, &[[0, 1, 2, -1]], 0));
        bytes.extend((-1i32).to_le_bytes());
        let model = parse(&bytes).expect("a model");
        assert_eq!(model.textures, ["=paint.tga", "rust.tga"]);
        let names: Vec<&str> = model.objects.iter().map(|o| o.name.as_str()).collect();
        assert_eq!(names, ["hull", "lid"]);
        assert_eq!(model.objects[0].faces[0].texture, Some(1));
        assert_eq!(model.objects[1].faces[0].texture, None);
        assert_eq!(model.objects[0].vertices[1].position, [1.0, 0.0, 0.0]);
        assert_eq!(model.objects[0].transform[3], [0.0, 5.0, 0.0, 1.0]);
        assert_eq!(model.objects[1].transform[3], [0.0, 0.0, 0.0, 1.0]);
        let animation = model.objects[0].animation.as_ref().expect("an animation");
        assert_eq!(
            animation.at(1).map(|pose| pose[3]),
            Some([0.0, 0.0, 0.0, 1.0])
        );
        assert_eq!(animation.at(9), animation.at(1));
        assert_eq!(model.objects[1].animation, None);
    }

    #[test]
    fn a_part_picks_its_pose_for_each_frame() {
        let mut bytes = object_bytes("arm", &[], &[], OBJECT_ANIMATION);
        // Three frames, between two poses: the second one up by 2.
        bytes.extend(2i32.to_le_bytes());
        bytes.extend((-3i32).to_le_bytes());
        bytes.extend(2i32.to_le_bytes());
        for pick in [0i16, 1, 1] {
            bytes.extend(pick.to_le_bytes());
        }
        for up in [0.0f32, 2.0] {
            for value in [1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, up, 0.0] {
                bytes.extend(value.to_le_bytes());
            }
        }
        let arm = object(
            &mut Reader {
                data: &bytes,
                at: 0,
            },
            0,
        )
        .expect("a part");
        let animation = arm.animation.expect("an animation");
        let up = |frame| animation.at(frame).map(|pose| pose[3][1]);
        assert_eq!((up(0), up(1), up(2)), (Some(0.0), Some(2.0), Some(2.0)));
        assert_eq!(arm.transform[3][1], 0.0);
    }

    #[test]
    fn the_sequences_are_read_from_the_end_of_the_effects() {
        let entry = |name: &str, ranges: &[(u32, u32)]| {
            let mut bytes = name.as_bytes().to_vec();
            bytes.push(0);
            bytes.resize(SEQUENCE_NAME, 0xcc);
            bytes.extend(1i32.to_le_bytes());
            for &(first, last) in ranges {
                bytes.extend([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
                bytes.extend(PADDING);
                bytes.extend(first.to_le_bytes());
                bytes.extend(last.to_le_bytes());
            }
            bytes
        };
        let mut effects = b"FXDataHeader\0\x05\0\0\0Front wheel\0".to_vec();
        effects.extend(entry("Stationary", &[(0, 0)]));
        effects.extend(entry("Move", &[(12, 20), (20, 33)]));
        effects.extend(entry("Fire 0", &[(40, 45)]));
        effects.extend([0; 4]);
        let sequence = |name: &str, ranges: &[(usize, usize)]| Sequence {
            name: name.into(),
            ranges: ranges.to_vec(),
        };
        assert_eq!(
            sequences(&effects),
            [
                sequence("Stationary", &[(0, 0)]),
                sequence("Move", &[(12, 20), (20, 33)]),
                sequence("Fire 0", &[(40, 45)]),
            ]
        );
        assert_eq!(sequences(b"FXDataHeader\0\0\0\0\0"), []);
    }

    #[test]
    fn a_tga_stored_from_its_bottom_row_up_comes_out_top_first() {
        let mut bytes = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 2, 0, 24, 0];
        // The bottom pixel, blue, then the top one, red.
        bytes.extend([255, 0, 0, 0, 0, 255]);
        let image = tga(&bytes).expect("a picture");
        assert_eq!(image.pixels, [[255, 0, 0, 255], [0, 0, 255, 255]]);
        assert_eq!(image.at([0.5, 0.25]), [255, 0, 0, 255]);
        assert_eq!(image.at([0.5, 1.75]), [0, 0, 255, 255]);
    }
}
