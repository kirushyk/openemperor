//! Plays a movie through FFmpeg's libraries, by way of the small C file in `shim/` that uses
//! them: its pictures decoded one at a time as they are wanted, its sound all at once.

use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::ptr::NonNull;

/// How its sound comes, whatever the movie holds. Matches `shim/oe_movie.h`.
pub const SOUND_CHANNELS: u32 = 2;
pub const SOUND_RATE: u32 = 44_100;

#[repr(C)]
struct RawMovie {
    _private: [u8; 0],
}

unsafe extern "C" {
    fn oe_movie_open(
        path: *const std::ffi::c_char,
        width: *mut i32,
        height: *mut i32,
        fps: *mut f32,
    ) -> *mut RawMovie;
    fn oe_movie_next_frame(movie: *mut RawMovie, rgba: *mut u8) -> i32;
    fn oe_movie_sound(movie: *const RawMovie, frames: *mut u32) -> *const f32;
    fn oe_movie_close(movie: *mut RawMovie);
    fn oe_sound_decode(location: *const std::ffi::c_char, frames: *mut u32) -> *mut f32;
    fn oe_sound_free(samples: *mut f32);
}

/// All the sound of anything FFmpeg can open, such as part of a file given as
/// `subfile,,start,FROM,end,TO,,:PATH`, interleaved as a movie's is. Empty if there is none.
pub fn decode_sound(location: &str) -> Vec<f32> {
    let Ok(location) = CString::new(location) else {
        return Vec::new();
    };
    let mut frames = 0;
    // SAFETY: the location is a valid C string and `frames` is valid for the call.
    let samples = unsafe { oe_sound_decode(location.as_ptr(), &mut frames) };
    if samples.is_null() {
        return Vec::new();
    }
    let count = frames as usize * SOUND_CHANNELS as usize;
    // SAFETY: the shim returned `frames` frames of `SOUND_CHANNELS` samples there, which are
    // copied before they go back to it.
    let sound = unsafe { std::slice::from_raw_parts(samples, count) }.to_vec();
    // SAFETY: they came from oe_sound_decode and are not used again.
    unsafe { oe_sound_free(samples) };
    sound
}

pub struct Movie {
    raw: NonNull<RawMovie>,
    pub width: u32,
    pub height: u32,
    fps: f32,
    /// The picture showing now, and how many have been shown so far.
    frame: Vec<u8>,
    shown: usize,
    over: bool,
}

impl Movie {
    /// Opens a movie. Nothing if it is not there or cannot be played.
    pub fn open(path: &Path) -> Option<Self> {
        let path = CString::new(path.as_os_str().as_bytes()).ok()?;
        let (mut width, mut height, mut fps) = (0, 0, 0.0);
        // SAFETY: the path is a valid C string and the out pointers are valid for the call.
        let raw = unsafe { oe_movie_open(path.as_ptr(), &mut width, &mut height, &mut fps) };
        let raw = NonNull::new(raw)?;
        let (width, height) = (width as u32, height as u32);
        Some(Movie {
            raw,
            width,
            height,
            fps,
            frame: vec![0; (width * height * 4) as usize],
            shown: 0,
            over: false,
        })
    }

    /// All of its sound, interleaved, `SOUND_CHANNELS` to a frame.
    pub fn sound(&self) -> &[f32] {
        let mut frames = 0;
        // SAFETY: the movie is open, and the samples live as long as it does.
        let samples = unsafe { oe_movie_sound(self.raw.as_ptr(), &mut frames) };
        if samples.is_null() {
            return &[];
        }
        // SAFETY: the shim holds `frames` frames of `SOUND_CHANNELS` samples there.
        unsafe { std::slice::from_raw_parts(samples, frames as usize * SOUND_CHANNELS as usize) }
    }

    /// Moves on to the picture for this far into the movie. Returns whether it changed.
    pub fn advance(&mut self, seconds: f32) -> bool {
        let wanted = (seconds * self.fps) as usize + 1;
        let mut changed = false;
        while self.shown < wanted && !self.over {
            // SAFETY: the frame holds width * height * 4 bytes, as the shim writes.
            if unsafe { oe_movie_next_frame(self.raw.as_ptr(), self.frame.as_mut_ptr()) } == 1 {
                self.shown += 1;
                changed = true;
            } else {
                self.over = true;
            }
        }
        changed
    }

    /// The picture showing now, black before the first.
    pub fn frame(&self) -> &[u8] {
        &self.frame
    }

    /// Whether every picture has been shown.
    pub fn is_over(&self) -> bool {
        self.over
    }
}

impl Drop for Movie {
    fn drop(&mut self) {
        // SAFETY: the movie is open, and is not used again.
        unsafe { oe_movie_close(self.raw.as_ptr()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_movie_that_is_not_there_does_not_open() {
        assert!(Movie::open(Path::new("/nowhere/at/all.bik")).is_none());
    }
}
