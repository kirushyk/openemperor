/* Movies, decoded by FFmpeg's libraries. Rust mirrors this file in src/video.rs: keep them in
 * step. */
#ifndef OE_MOVIE_H
#define OE_MOVIE_H

#include <stdint.h>

/* The rate and the channels its sound comes in, whatever the movie holds. */
enum { OE_MOVIE_SOUND_RATE = 44100, OE_MOVIE_SOUND_CHANNELS = 2 };

typedef struct OeMovie OeMovie;

/* Opens a movie and decodes all of its sound. Returns NULL if it cannot be played. */
OeMovie *oe_movie_open(const char *path, int32_t *width, int32_t *height, float *fps);

/* Decodes its next picture into `rgba`, width * height * 4 bytes, top row first. Returns 1, or
 * 0 once there are no more. */
int32_t oe_movie_next_frame(OeMovie *movie, uint8_t *rgba);

/* All of its sound, interleaved, and how many frames of it; NULL if it has none. It lasts as
 * long as the movie does. */
const float *oe_movie_sound(const OeMovie *movie, uint32_t *frames);

void oe_movie_close(OeMovie *movie);

/* Decodes all the sound of anything FFmpeg can open, such as part of a file given as
 * "subfile,,start,FROM,end,TO,,:PATH", interleaved as the movies' sound is. Returns NULL if
 * there is none; what it returns goes back to oe_sound_free. */
float *oe_sound_decode(const char *path, uint32_t *frames);

void oe_sound_free(float *samples);

#endif
