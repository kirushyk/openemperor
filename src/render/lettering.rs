//! The original's lettering: a font on its disc is a picture of its letters, `Standard.tga`,
//! with a list of which letters they are, `FONT\STANDARD.MAP`, one after another in the order
//! the picture has them. The picture's first column starts with a line as tall as the font.
//! Its letters stand in rows, each letter set apart from the next by an empty column, and a
//! row from the next by empty lines, an accent above its letter counting with the row it is
//! over. Read once with the disc's models, the letters are packed into pictures of their own
//! and text is drawn with them in place of our built-in font.

use super::original::PICTURE_SIDE;
use crate::xbf::Image;
use std::collections::HashMap;

/// The room left around each letter where it is packed, so none bleeds into the next.
const PADDING: usize = 1;
/// How far apart the original sets letters, in the picture's pixels: its `FONT\SCRIPT.TXT`
/// gives a spacing of 1.
const SPACING: f32 = 1.0;
/// How wide a space is, as a share of the font's capital height.
const SPACE: f32 = 0.35;
/// The letter whose height is the height of capitals.
const CAPITAL: char = 'H';

/// A letter where it is packed: which picture, where on it, how large in the original's
/// pixels, and how far above and below the line it reaches.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Letter {
    pub layer: usize,
    pub uv: [f32; 4],
    pub width: f32,
    pub above: f32,
    pub below: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Lettering {
    letters: HashMap<char, Letter>,
    /// How tall its capitals are, in the picture's pixels.
    pub capital: f32,
}

impl Lettering {
    pub fn letter(&self, ch: char) -> Option<&Letter> {
        self.letters.get(&ch)
    }

    /// How far on from one letter to the next, for each letter of a text, in the picture's
    /// pixels: its width and the spacing, or a space's for one the font has not.
    pub fn advances<'a>(&'a self, text: &'a str) -> impl Iterator<Item = f32> + 'a {
        text.chars().map(|ch| {
            self.letters
                .get(&ch)
                .map_or(self.capital * SPACE, |letter| letter.width)
                + SPACING
        })
    }
}

/// A letter as the font's picture has it: its box, and its row's line.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Cut {
    left: usize,
    right: usize,
    top: usize,
    bottom: usize,
    line: usize,
}

/// Reads a font out of its picture and its list of letters (UTF-16, as the disc writes it),
/// adding its letters' pictures to those given to the backend: `pictures` their bytes so far
/// and `layers` how many they are.
pub(super) fn read(
    image: &Image,
    listed: &[u8],
    pictures: &mut Vec<u8>,
    layers: &mut usize,
) -> Option<Lettering> {
    let units: Vec<u16> = listed
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    let chars: Vec<char> = char::decode_utf16(units)
        .filter_map(Result::ok)
        .filter(|&ch| ch != '\u{feff}')
        .collect();
    let lit = |x: usize, y: usize| {
        image.pixels[y * image.width + x][..3]
            .iter()
            .any(|&c| c > 0)
    };
    let tall = (0..image.height).take_while(|&y| lit(0, y)).count();
    let on = |x: usize, y: usize| lit(x, y) && !(x == 0 && y < tall);
    let cuts = cut(image.width, image.height, tall, on);
    if tall == 0 || cuts.len() != chars.len() {
        return None;
    }
    let capital = chars
        .iter()
        .position(|&ch| ch == CAPITAL)
        .map(|at| cuts[at])
        .and_then(|c| {
            let rows: Vec<usize> = (c.top..c.bottom)
                .filter(|&y| (c.left..c.right).any(|x| on(x, y)))
                .collect();
            Some((rows.last()? + 1 - rows.first()?) as f32)
        })?;
    let mut lettering = Lettering {
        letters: HashMap::new(),
        capital,
    };
    // Packs the letters into pictures of their own, row by row.
    let mut picture = vec![[0u8; 4]; PICTURE_SIDE * PICTURE_SIDE];
    let (mut x, mut y, mut row) = (0, 0, 0);
    let flush = |picture: &mut Vec<[u8; 4]>, pictures: &mut Vec<u8>, layers: &mut usize| {
        pictures.extend(picture.iter().flatten());
        picture.fill([0; 4]);
        *layers += 1;
    };
    for (&ch, c) in chars.iter().zip(&cuts) {
        let (w, h) = (c.right - c.left + PADDING, c.bottom - c.top + PADDING);
        if w > PICTURE_SIDE || h > PICTURE_SIDE {
            continue;
        }
        if x + w > PICTURE_SIDE {
            (x, y, row) = (0, y + row, 0);
        }
        if y + h > PICTURE_SIDE {
            flush(&mut picture, pictures, layers);
            (x, y, row) = (0, 0, 0);
        }
        for dy in 0..c.bottom - c.top {
            for dx in 0..c.right - c.left {
                let [r, g, b, _] = image.pixels[(c.top + dy) * image.width + c.left + dx];
                let shade = if on(c.left + dx, c.top + dy) {
                    r.max(g).max(b)
                } else {
                    0
                };
                picture[(y + dy) * PICTURE_SIDE + x + dx] = [255, 255, 255, shade];
            }
        }
        let side = PICTURE_SIDE as f32;
        lettering.letters.insert(
            ch,
            Letter {
                layer: *layers,
                uv: [
                    x as f32 / side,
                    y as f32 / side,
                    (x + c.right - c.left) as f32 / side,
                    (y + c.bottom - c.top) as f32 / side,
                ],
                width: (c.right - c.left) as f32,
                above: (c.line - c.top) as f32,
                below: (c.bottom - c.line) as f32,
            },
        );
        x += w;
        row = row.max(h);
    }
    flush(&mut picture, pictures, layers);
    Some(lettering)
}

/// The letters of a font's picture `width` by `height`, `tall` the font's height, where `on`
/// says which pixels are drawn: in rows of drawn lines, a row too short to be one with the
/// row before it; each letter the drawn columns between empty ones; and each row's line
/// where most of its letters end.
fn cut(width: usize, height: usize, tall: usize, on: impl Fn(usize, usize) -> bool) -> Vec<Cut> {
    let mut rows: Vec<(usize, usize)> = Vec::new();
    let mut start = None;
    for y in 0..=height {
        let drawn = y < height && (0..width).any(|x| on(x, y));
        match (drawn, start) {
            (true, None) => start = Some(y),
            (false, Some(top)) => {
                match rows.last_mut() {
                    Some(last) if y - last.0 <= tall => last.1 = y,
                    _ => rows.push((top, y)),
                }
                start = None;
            }
            _ => {}
        }
    }
    let mut cuts = Vec::new();
    for (top, bottom) in rows {
        let mut letters = Vec::new();
        let mut from = None;
        for x in 0..=width {
            let drawn = x < width && (top..bottom).any(|y| on(x, y));
            match (drawn, from) {
                (true, None) => from = Some(x),
                (false, Some(left)) => {
                    letters.push((left, x));
                    from = None;
                }
                _ => {}
            }
        }
        // The line is where most of the row's letters end.
        let mut ends: HashMap<usize, usize> = HashMap::new();
        for &(left, right) in &letters {
            let end = (top..bottom)
                .rev()
                .find(|&y| (left..right).any(|x| on(x, y)))
                .map_or(bottom, |y| y + 1);
            *ends.entry(end).or_default() += 1;
        }
        let line = ends
            .into_iter()
            .max_by_key(|&(end, count)| (count, end))
            .map_or(bottom, |(end, _)| end);
        cuts.extend(letters.into_iter().map(|(left, right)| Cut {
            left,
            right,
            top,
            bottom,
            line,
        }));
    }
    cuts
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A picture drawn from rows of text, `#` a drawn pixel.
    fn picture(rows: &[&str]) -> Image {
        Image {
            width: rows[0].len(),
            height: rows.len(),
            pixels: rows
                .iter()
                .flat_map(|row| {
                    row.chars()
                        .map(|c| if c == '#' { [255; 4] } else { [0, 0, 0, 255] })
                })
                .collect(),
        }
    }

    fn listed(text: &str) -> Vec<u8> {
        [0xfeff]
            .into_iter()
            .chain(text.encode_utf16())
            .flat_map(u16::to_le_bytes)
            .collect()
    }

    #[test]
    fn letters_are_cut_between_empty_columns_and_rows_with_accents_kept_on_their_letters() {
        let image = picture(&[
            "#.............",
            "#.##..#.#..#..",
            "#.##..###..#..",
            "#.##..#.#..#..",
            "#..........#..",
            "..............",
            "...#......#...",
            "..............",
            "..##.....##...",
            "..##.....##...",
        ]);
        let (mut pictures, mut layers) = (Vec::new(), 0);
        let lettering = read(&image, &listed("IHjAB"), &mut pictures, &mut layers).unwrap();
        assert_eq!(layers, 1);
        assert_eq!(pictures.len(), PICTURE_SIDE * PICTURE_SIDE * 4);
        let h = lettering.letter('H').unwrap();
        assert_eq!(h.width, 3.0);
        assert_eq!(lettering.capital, 3.0);
        let i = lettering.letter('I').unwrap();
        assert_eq!((i.above, i.below), (3.0, 1.0), "the row's line under it");
        let j = lettering.letter('j').unwrap();
        assert_eq!((j.above, j.below), (3.0, 1.0), "reaching below it");
        let a = lettering.letter('A').unwrap();
        assert_eq!(a.above, 4.0, "with its accent");
        let advances: Vec<f32> = lettering.advances("H x").collect();
        assert_eq!(advances, [4.0, 3.0 * SPACE + 1.0, 3.0 * SPACE + 1.0]);
    }

    #[test]
    fn a_list_that_does_not_match_the_picture_reads_as_no_font() {
        let image = picture(&["#.#.", "#.#.", "#..."]);
        let (mut pictures, mut layers) = (Vec::new(), 0);
        assert_eq!(
            read(&image, &listed("HH"), &mut pictures, &mut layers),
            None
        );
    }
}
