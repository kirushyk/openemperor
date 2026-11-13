//! DEFLATE, the packing the game's disc uses: in the zlib streams of its data files, and in the
//! MSZIP blocks of its install cabinet.

/// How long a copy is, by its code past 256, and how many more bits add to it.
const LENGTH_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LENGTH_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
/// How far back a copy starts, by its code, and how many more bits add to it.
const DISTANCE_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DISTANCE_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];
/// The order a dynamic block gives the lengths of the codes its code lengths are written in.
const LENGTH_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];
const LONGEST_CODE: usize = 15;

/// Reads bits from the lowest of each byte up, as DEFLATE writes them.
struct Bits<'a> {
    data: &'a [u8],
    at: usize,
}

impl Bits<'_> {
    fn bit(&mut self) -> Option<u32> {
        let byte = *self.data.get(self.at / 8)?;
        let bit = u32::from(byte >> (self.at % 8)) & 1;
        self.at += 1;
        Some(bit)
    }

    fn bits(&mut self, count: u32) -> Option<u32> {
        (0..count).try_fold(0, |value, n| Some(value | self.bit()? << n))
    }

    /// Skips to the start of the next whole byte.
    fn align(&mut self) {
        self.at = self.at.next_multiple_of(8);
    }
}

/// A canonical Huffman code: how many codes there are of each length, and the symbols in the
/// order of their codes.
struct Huffman {
    counts: [u16; LONGEST_CODE + 1],
    symbols: Vec<u16>,
}

impl Huffman {
    fn new(lengths: &[u8]) -> Self {
        let mut counts = [0; LONGEST_CODE + 1];
        for &length in lengths {
            counts[usize::from(length)] += 1;
        }
        counts[0] = 0;
        let mut symbols: Vec<u16> = (0..lengths.len() as u16)
            .filter(|&symbol| lengths[usize::from(symbol)] != 0)
            .collect();
        symbols.sort_by_key(|&symbol| lengths[usize::from(symbol)]);
        Huffman { counts, symbols }
    }

    fn decode(&self, bits: &mut Bits) -> Option<u16> {
        let (mut code, mut first, mut index) = (0, 0, 0);
        for &count in &self.counts[1..] {
            code |= bits.bit()? as usize;
            let count = usize::from(count);
            if code < first + count {
                return self.symbols.get(index + code - first).copied();
            }
            index += count;
            first = (first + count) << 1;
            code <<= 1;
        }
        None
    }
}

/// Unpacks one DEFLATE stream onto the end of `out`, whose earlier bytes copies may reach back
/// into, and says how many bytes of `data` it took; `None` if it is broken.
pub fn inflate(data: &[u8], out: &mut Vec<u8>) -> Option<usize> {
    let bits = &mut Bits { data, at: 0 };
    loop {
        let last = bits.bit()? == 1;
        match bits.bits(2)? {
            0 => {
                bits.align();
                let start = bits.at / 8;
                let head = data.get(start..start + 4)?;
                let length = usize::from(u16::from_le_bytes([head[0], head[1]]));
                if length != usize::from(!u16::from_le_bytes([head[2], head[3]])) {
                    return None;
                }
                out.extend_from_slice(data.get(start + 4..start + 4 + length)?);
                bits.at = (start + 4 + length) * 8;
            }
            1 => {
                let mut lengths = [8; 288];
                lengths[144..256].fill(9);
                lengths[256..280].fill(7);
                block(bits, out, &Huffman::new(&lengths), &Huffman::new(&[5; 30]))?;
            }
            2 => {
                let (literals, distances) = dynamic_codes(bits)?;
                block(bits, out, &literals, &distances)?;
            }
            _ => return None,
        }
        if last {
            return Some(bits.at.div_ceil(8));
        }
    }
}

/// Unpacks a zlib stream: a two-byte head, then DEFLATE.
pub fn unzlib(data: &[u8]) -> Option<Vec<u8>> {
    let deflated = *data.first()? & 0x0f == 8;
    let mut out = Vec::new();
    inflate(data.get(2..).filter(|_| deflated)?, &mut out)?;
    Some(out)
}

/// The codes a dynamic block gives for its literals and lengths, and for its distances.
fn dynamic_codes(bits: &mut Bits) -> Option<(Huffman, Huffman)> {
    let literals = bits.bits(5)? as usize + 257;
    let distances = bits.bits(5)? as usize + 1;
    let length_codes = bits.bits(4)? as usize + 4;
    let mut code_lengths = [0; 19];
    for &symbol in &LENGTH_ORDER[..length_codes] {
        code_lengths[symbol] = bits.bits(3)? as u8;
    }
    let lengths_code = Huffman::new(&code_lengths);
    let mut lengths = Vec::with_capacity(literals + distances);
    while lengths.len() < literals + distances {
        let (value, repeat) = match lengths_code.decode(bits)? {
            symbol @ 0..=15 => (symbol as u8, 1),
            16 => (*lengths.last()?, 3 + bits.bits(2)?),
            17 => (0, 3 + bits.bits(3)?),
            _ => (0, 11 + bits.bits(7)?),
        };
        lengths.extend((0..repeat).map(|_| value));
    }
    if lengths.len() != literals + distances {
        return None;
    }
    let (literal_lengths, distance_lengths) = lengths.split_at(literals);
    Some((
        Huffman::new(literal_lengths),
        Huffman::new(distance_lengths),
    ))
}

/// Unpacks a block's literals and copies until its end code.
fn block(
    bits: &mut Bits,
    out: &mut Vec<u8>,
    literals: &Huffman,
    distances: &Huffman,
) -> Option<()> {
    loop {
        let symbol = usize::from(literals.decode(bits)?);
        if symbol < 256 {
            out.push(symbol as u8);
            continue;
        }
        if symbol == 256 {
            return Some(());
        }
        let code = symbol - 257;
        let length = usize::from(*LENGTH_BASE.get(code)?)
            + bits.bits(u32::from(LENGTH_EXTRA[code]))? as usize;
        let code = usize::from(distances.decode(bits)?);
        let distance = usize::from(*DISTANCE_BASE.get(code)?)
            + bits.bits(u32::from(DISTANCE_EXTRA[code]))? as usize;
        let start = out.len().checked_sub(distance)?;
        for n in 0..length {
            out.push(out[start + n]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stored_block_comes_out_as_it_went_in() {
        let mut out = Vec::new();
        let taken = inflate(&[1, 3, 0, !3, !0, b'a', b'b', b'c'], &mut out);
        assert_eq!(out, b"abc");
        assert_eq!(taken, Some(8));
    }

    #[test]
    fn a_zlib_stream_with_fixed_codes_and_a_copy_unpacks() {
        // zlib.compress(b"hello hello hello")
        let packed = [
            0x78, 0x9c, 0xcb, 0x48, 0xcd, 0xc9, 0xc9, 0x57, 0xc8, 0x40, 0x90, 0x00, 0x3a, 0x2e,
            0x06, 0x7d,
        ];
        assert_eq!(unzlib(&packed).as_deref(), Some(&b"hello hello hello"[..]));
    }

    #[test]
    fn a_zlib_stream_with_dynamic_codes_unpacks() {
        // Four hundred letters picked by a simple generator, packed by zlib at level 9.
        let packed = [
            0x78, 0xda, 0x3d, 0x50, 0x81, 0x0d, 0x80, 0x30, 0x0c, 0xba, 0x15, 0x08, 0xff, 0xbf,
            0x60, 0x81, 0x45, 0x35, 0x91, 0xb2, 0xae, 0x50, 0x24, 0x10, 0x82, 0x41, 0x8b, 0x07,
            0x0d, 0x91, 0x87, 0x5a, 0xf0, 0x1e, 0x04, 0x82, 0x2a, 0x17, 0xd4, 0x4f, 0x38, 0xc2,
            0x81, 0x96, 0xcc, 0x9d, 0xe5, 0x43, 0xb9, 0xb5, 0x85, 0xbf, 0xb1, 0xa1, 0x84, 0x9d,
            0x6b, 0x97, 0x61, 0x86, 0xcb, 0x3f, 0x33, 0xdb, 0x1e, 0x98, 0xea, 0x78, 0xdd, 0x25,
            0xf7, 0x20, 0x48, 0x35, 0x52, 0x03, 0x7d, 0x33, 0x2e, 0x1a, 0xec, 0x54, 0xa7, 0x6e,
            0xaf, 0x1d, 0x53, 0x9a, 0x87, 0x2c, 0xa1, 0xd9, 0xdf, 0xfc, 0xab, 0x4e, 0x74, 0x7a,
            0xbb, 0x94, 0xad, 0xab, 0x7c, 0x2a, 0x0d, 0xa1, 0x1e, 0x32, 0x24, 0x2b, 0x04, 0xb4,
            0xb3, 0x99, 0xa0, 0x4a, 0x76, 0x23, 0xea, 0x26, 0xd9, 0x3b, 0x6a, 0x47, 0xae, 0xc7,
            0x9c, 0x92, 0x97, 0x93, 0x5f, 0x86, 0x58, 0x90, 0x35, 0xf6, 0xaf, 0xd4, 0x14, 0x6a,
            0x4f, 0xed, 0x8b, 0x62, 0x4d, 0xe9, 0x03, 0x0d, 0x17, 0x99, 0x4d,
        ];
        let mut seed: u32 = 1;
        let unpacked: Vec<u8> = (0..400)
            .map(|_| {
                seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345) & 0x7fff_ffff;
                b"aaaabbbcce"[(seed >> 16) as usize % 10]
            })
            .collect();
        assert_eq!(unzlib(&packed), Some(unpacked));
    }
}
