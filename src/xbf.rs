//! The original game's models, in its XBF files, and the TGA pictures they are painted with.

/// A model: the pictures its faces are painted with, and its objects, each with its own.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Model {
    pub textures: Vec<String>,
    pub objects: Vec<Object>,
    pub sequences: Vec<Sequence>,
    pub slides: Vec<Slide>,
}

/// A part whose picture slides over it as the model's animation plays, such as a belt or a
/// track: by how much of the picture each frame, along u and along v.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Slide {
    pub part: String,
    pub step: [f32; 2],
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
    pub bending: Option<Bending>,
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

/// How a part's own shape bends over its model's animation: where its vertices are at each
/// frame kept, and for each frame of the animation, the kept frame it is at, the next, and how
/// far it is from the one to the other.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Bending {
    pub kept: Vec<Vec<[f32; 3]>>,
    pub frames: Vec<(u32, u32, f32)>,
}

impl Bending {
    /// Where a vertex is at a frame, the last one's for frames past the end.
    pub fn at(&self, frame: usize, vertex: usize) -> Option<[f32; 3]> {
        let &(from, to, share) = self.frames.get(frame).or(self.frames.last())?;
        let from = self.kept.get(from as usize)?.get(vertex)?;
        let to = self.kept.get(to as usize)?.get(vertex)?;
        Some(std::array::from_fn(|i| from[i] + (to[i] - from[i]) * share))
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
    let effects = reader.bytes(effects)?;
    let (sequences, slides) = (sequences(effects), slides(effects));
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
        slides,
    })
}

/// What starts one of a model's effects that slides a part's picture: its kind, any number,
/// then what the original files it under, after which come the part's name and the two steps.
const SLIDES: i32 = 7;
const SLIDE_FILED: i32 = 12;
/// The longest part name looked for.
const LONGEST_NAME: usize = 64;
/// The most of its picture a part slides by in a frame.
const LONGEST_STEP: f32 = 1.0;

/// The parts whose pictures slide, each by the first step said for it. The effects are not
/// read through, being of many kinds not known: these are found where they read as one.
fn slides(effects: &[u8]) -> Vec<Slide> {
    let word = |at: usize| {
        effects
            .get(at..at + 4)
            .map(|bytes| i32::from_le_bytes(bytes.try_into().unwrap()))
    };
    let float = |at: usize| word(at).map(|word| f32::from_bits(word as u32));
    let mut list: Vec<Slide> = Vec::new();
    for at in 0..effects.len() {
        if word(at) != Some(SLIDES) || word(at + 8) != Some(SLIDE_FILED) {
            continue;
        }
        let name = &effects[at + 12..];
        let Some(length) = name.iter().take(LONGEST_NAME).position(|&byte| byte == 0) else {
            continue;
        };
        let name = &name[..length];
        let after = at + 12 + length + 1;
        let Some(step) = float(after).zip(float(after + 4)).map(|(u, v)| [u, v]) else {
            continue;
        };
        if name.is_empty()
            || !name.iter().all(u8::is_ascii_graphic)
            || !step.iter().all(|step| step.abs() <= LONGEST_STEP)
        {
            continue;
        }
        let part: String = name.iter().map(|&byte| char::from(byte)).collect();
        if !list.iter().any(|slide| slide.part == part) {
            list.push(Slide { part, step });
        }
    }
    list
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
    let mut bending = None;
    if flags & VERTEX_ANIMATION != 0 {
        bending = vertex_animation(reader, vertex_count)?;
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
        bending,
        vertices,
        faces,
        children,
    })
}

/// What says that each frame of a vertex animation is listed with where its vertices are kept.
const MAPPED: u16 = 0x8000;

/// Vertices moved frame by frame: the frames kept, and unless a count of entries that is not
/// negative says there is no more, every vertex of every frame kept, each three whole numbers
/// to be halved as many times as said and two bytes more; then perhaps every frame of the
/// animation, with where its vertices start if it is kept, 0 for one between kept frames.
/// Without that the frames kept are the ones their numbers say.
fn vertex_animation(reader: &mut Reader, vertices: usize) -> Option<Option<Bending>> {
    let length = reader.count()?;
    let negative_entries = reader.i32()?;
    let numbers: Vec<i32> = (0..reader.count()?)
        .map(|_| reader.i32())
        .collect::<Option<_>>()?;
    if negative_entries >= 0 {
        return Some(None);
    }
    let halvings = reader.i16()?;
    let mapped = reader.i16()? as u16 & MAPPED != 0;
    let entries = reader.count()?;
    let data = reader.bytes(8 * entries)?.to_vec();
    let mut marks = vec![None; length];
    if mapped {
        for mark in &mut marks {
            let at = reader.count()?;
            *mark = (at > 0).then(|| (at - 1) / (4 * vertices.max(1)));
        }
    } else {
        for (kept, &number) in numbers.iter().enumerate() {
            if let Some(mark) = usize::try_from(number).ok().and_then(|n| marks.get_mut(n)) {
                *mark = Some(kept);
            }
        }
    }
    if vertices == 0 || entries != numbers.len() * vertices || !(0..16).contains(&halvings) {
        return Some(None);
    }
    let scale = 1.0 / f32::from(1u16 << halvings);
    let kept = data
        .chunks_exact(8 * vertices)
        .map(|frame| {
            frame
                .chunks_exact(8)
                .map(|entry| {
                    std::array::from_fn(|i| {
                        f32::from(i16::from_le_bytes([entry[2 * i], entry[2 * i + 1]])) * scale
                    })
                })
                .collect()
        })
        .collect::<Vec<Vec<_>>>();
    let last = kept.len().checked_sub(1)?;
    let mark = |n: usize| marks[n].filter(|&k| k <= last);
    let frames = (0..length)
        .map(|n| {
            let before = (0..=n).rev().find_map(|b| Some((b, mark(b)?)));
            let after = (n..length).find_map(|a| Some((a, mark(a)?)));
            let frame = match (before, after) {
                (Some((b, from)), Some((a, to))) if a > b => {
                    (from, to, (n - b) as f32 / (a - b) as f32)
                }
                (Some((_, from)), _) => (from, from, 0.0),
                (None, Some((_, to))) => (to, to, 0.0),
                (None, None) => (0, 0, 0.0),
            };
            (frame.0 as u32, frame.1 as u32, frame.2)
        })
        .collect();
    Some(Some(Bending { kept, frames }))
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

/// The wash laid over a map's ground, as its `test.CPT` keeps it, over as many cells each way as
/// given: after its width and height, squares of `WASH_TILE` pixels, `WASH_TILES` to a row,
/// each in blocks of four by four pixels compressed as DXT3 is. Only the squares the map
/// reaches into are taken, as the original does.
pub fn wash(data: &[u8], (columns, rows): (usize, usize)) -> Option<Image> {
    /// How many pixels each way a square of it has, and how many cells it covers.
    const WASH_TILE: usize = 256;
    const CELLS_PER_TILE: usize = 32;
    const WASH_TILES: usize = 8;
    const BLOCK: usize = 16;
    let tiles = |cells: usize| cells.div_ceil(CELLS_PER_TILE).min(WASH_TILES);
    let (across, down) = (tiles(columns), tiles(rows));
    let (width, height) = (across * WASH_TILE, down * WASH_TILE);
    let blocks = data.get(8..)?;
    if width == 0 || height == 0 || blocks.len() < WASH_TILES * down * WASH_TILE * WASH_TILE {
        return None;
    }
    let mut pixels = vec![[0; 4]; width * height];
    let in_a_row = WASH_TILE / 4;
    for by in 0..height / 4 {
        for bx in 0..width / 4 {
            let tile = by / in_a_row * WASH_TILES + bx / in_a_row;
            let at =
                (tile * in_a_row * in_a_row + by % in_a_row * in_a_row + bx % in_a_row) * BLOCK;
            for (n, pixel) in dxt3(blocks.get(at..at + BLOCK)?).into_iter().enumerate() {
                pixels[(by * 4 + n / 4) * width + bx * 4 + n % 4] = pixel;
            }
        }
    }
    Some(Image {
        width,
        height,
        pixels,
    })
}

/// The sixteen pixels of a block compressed as DXT3 is, row by row: their alphas, four bits
/// each, then two colors as 5, 6 and 5 bits and which of four steps from one to the other each
/// pixel takes.
fn dxt3(block: &[u8]) -> [[u8; 4]; 16] {
    let alphas = u64::from_le_bytes(block[..8].try_into().unwrap_or_default());
    let color = |at: usize| {
        let value = u16::from_le_bytes([block[at], block[at + 1]]);
        let spread = |shift: u16, most: u16| {
            (u32::from(value >> shift & most) * 255 / u32::from(most)) as u8
        };
        [spread(11, 31), spread(5, 63), spread(0, 31)]
    };
    let (first, last) = (color(8), color(10));
    let between = |share: u32| -> [u8; 3] {
        std::array::from_fn(|n| {
            ((u32::from(first[n]) * (3 - share) + u32::from(last[n]) * share) / 3) as u8
        })
    };
    let steps = [first, last, between(1), between(2)];
    let which = u32::from_le_bytes([block[12], block[13], block[14], block[15]]);
    std::array::from_fn(|n| {
        let [r, g, b] = steps[(which >> (2 * n) & 3) as usize];
        [r, g, b, (alphas >> (4 * n) & 15) as u8 * 17]
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
    fn a_part_bends_through_the_frames_kept_and_between_them() {
        let mut bytes = object_bytes("body", &[[0.0; 3]], &[], VERTEX_ANIMATION);
        // Three frames, the first and the last kept, each with its one vertex halved twice.
        for value in [3, -2, 2, 0, 1] {
            bytes.extend(i32::to_le_bytes(value));
        }
        bytes.extend(2i16.to_le_bytes());
        bytes.extend(MAPPED.to_le_bytes());
        bytes.extend(2i32.to_le_bytes());
        for [x, y, z] in [[4i16, 0, -8], [8, 4, -8]] {
            for value in [x, y, z, 0] {
                bytes.extend(value.to_le_bytes());
            }
        }
        for at in [1i32, 0, 5] {
            bytes.extend(at.to_le_bytes());
        }
        let body = object(
            &mut Reader {
                data: &bytes,
                at: 0,
            },
            0,
        )
        .expect("a part");
        let bending = body.bending.expect("it bends");
        assert_eq!(bending.at(0, 0), Some([1.0, 0.0, -2.0]));
        assert_eq!(bending.at(1, 0), Some([1.5, 0.5, -2.0]));
        assert_eq!(bending.at(2, 0), Some([2.0, 1.0, -2.0]));
        assert_eq!(bending.at(9, 0), Some([2.0, 1.0, -2.0]));
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
    fn a_part_slides_its_picture_by_the_first_step_its_effects_say() {
        let slide = |name: &str, u: f32, v: f32| {
            let mut bytes = [SLIDES, 100, SLIDE_FILED].map(i32::to_le_bytes).concat();
            bytes.extend(name.as_bytes());
            bytes.push(0);
            bytes.extend(u.to_le_bytes());
            bytes.extend(v.to_le_bytes());
            bytes
        };
        let mut effects = b"FXDataHeader\0\x08\0\0\0d\0\0\0\x04\0\0\0conbelt01\0".to_vec();
        effects.extend(slide("conbelt01", 0.03, 0.0));
        effects.extend(slide("lefttrack01", 0.0, -0.1));
        effects.extend(slide("conbelt01", 0.5, 0.5));
        effects.extend(slide("far", 9.0, 0.0));
        let found = slides(&effects);
        let names: Vec<&str> = found.iter().map(|slide| slide.part.as_str()).collect();
        assert_eq!(names, ["conbelt01", "lefttrack01"]);
        assert_eq!(found[0].step, [0.03, 0.0]);
        assert_eq!(found[1].step, [0.0, -0.1]);
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

    #[test]
    fn a_wash_is_read_in_squares_of_its_blocks_as_far_as_the_map_reaches() {
        // One square, its first block white and opaque, the rest black and clear.
        let mut data = vec![0; 8 + 8 * 256 * 256];
        data[8..16].fill(0xff);
        data[16..18].copy_from_slice(&0xffffu16.to_le_bytes());
        data[18..20].copy_from_slice(&0xffffu16.to_le_bytes());
        let wash = wash(&data, (20, 30)).unwrap();
        assert_eq!((wash.width, wash.height), (256, 256));
        assert_eq!(wash.pixels[0], [255; 4]);
        assert_eq!(wash.pixels[3 * 256 + 3], [255; 4]);
        assert_eq!(wash.pixels[4], [0; 4]);
        assert_eq!(wash.pixels[4 * 256], [0; 4]);
        assert_eq!(super::wash(&data, (20, 40)), None);
    }
}
