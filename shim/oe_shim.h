/* The whole OS boundary. Every platform shim implements exactly these functions.
 * Rust mirrors this file in src/platform.rs: keep them in step. */
#ifndef OE_SHIM_H
#define OE_SHIM_H

#include <stdint.h>

typedef struct {
    float x, y; /* pixels, origin top-left, y down */
    float r, g, b, a;
} OeVertex;

enum {
    OE_EVENT_NONE = 0,
    OE_EVENT_QUIT = 1,
    OE_EVENT_RESIZE = 2,     /* a, b = new size in pixels */
    OE_EVENT_MOUSE_MOVE = 3, /* x, y = cursor in pixels, clamped to the screen */
    OE_EVENT_MOUSE_DOWN = 4, /* left button */
    OE_EVENT_MOUSE_UP = 5,   /* left button; a = 1 if Shift was held, otherwise 0 */
    OE_EVENT_KEY_DOWN = 6,   /* a = OE_KEY_*; b = printable ASCII typed, or 0 */
    OE_EVENT_FOCUS = 7,      /* a = 1 when the game gains focus, 0 when it loses it */
    OE_EVENT_WHEEL = 8,      /* y = notches turned, positive away from the player */
    OE_EVENT_RIGHT_DOWN = 9, /* right button; on a Mac trackpad, a two-finger click or tap */
    OE_EVENT_CONTROL = 10,   /* a = 1 when a Control key is pressed, 0 when it is let go */
    OE_EVENT_NUMBER_KEY = 11 /* a = 1 to 9, by place on the number row; b = OE_HELD_*.
                              * Comes as well as the OE_EVENT_KEY_DOWN for that key. */
};

enum {
    OE_HELD_NONE = 0,
    OE_HELD_SHIFT = 1,
    OE_HELD_CONTROL = 2 /* whether or not Shift is held too */
};

enum {
    OE_KEY_OTHER = 0,
    OE_KEY_ESCAPE = 1,
    OE_KEY_BACKSPACE = 2,
    OE_KEY_ENTER = 3,
    OE_KEY_TAB = 4,
    OE_KEY_PAUSE = 5 /* Pause/Break; Mac keyboards have no such key */
};

typedef struct {
    int32_t kind;
    int32_t a, b;
    float x, y;
} OeEvent;

/* Opens a fullscreen window and confines the cursor. Returns 0 on failure. */
int32_t oe_init(const char *title, int32_t *width, int32_t *height);

/* Returns 1 and fills `event` while events are pending, otherwise 0. */
int32_t oe_poll_event(OeEvent *event);

/* Draws a triangle list and presents. Blocks for vsync. */
void oe_draw(const OeVertex *vertices, uint32_t count, float r, float g, float b);

/* A corner of something solid in the world, placed and lit by the backend. */
typedef struct {
    float x, y, z;
    float normal[3];
    float r, g, b;
    float seen;
    float level;
    float stain[3];   /* where on its unit it is, and how black it goes */
    float picture[3]; /* where on a picture of oe_set_pictures it is, and which; below 0, none */
    float team[3];    /* the house color its picture's house parts take */
} OeWorldVertex;

/* A corner of something see-through: on the screen, with a depth to test. Premultiplied. */
typedef struct {
    float x, y, depth;
    float r, g, b, a;
} OeEffectVertex;

typedef struct {
    float place[3];
    float radius;
    float color[3];
    float unused;
} OePointLight;

enum { OE_MAX_LIGHTS = 16 };

/* A frame. Backends without a world pipeline get oe_draw instead. */
typedef struct {
    const OeWorldVertex *ground;
    uint32_t ground_count;
    const OeWorldVertex *objects;
    uint32_t objects_count;
    const OeWorldVertex *casters;
    uint32_t casters_count;
    const OeEffectVertex *effects;
    uint32_t effects_count;
    const OeVertex *over;
    uint32_t over_count;
    const OePointLight *lights;
    uint32_t lights_count;
    int32_t material; /* 0 plain, 1 water, 2 sand, 3 lava, 4 ice, 5 stone, 6 moss; 8 for objects */
    float time;
    float view[16];   /* by columns */
    float mirror[16]; /* the view as seen in a mirror at height 0 */
    float sun_view[16];
    float camera[3];
    float sun[3];
} OeWorld;

/* Draws a frame and presents: the sun's shadow map, the reflection if there is water, then the
 * ground and objects lit per pixel with a depth buffer, the effects tested against it, and what
 * goes over it all. Blocks for vsync. macOS only so far. */
void oe_draw_world(const OeWorld *world, float r, float g, float b);

/* A picture shown under all that each frame draws, as large as fits the screen without
 * changing its shape: rows of RGBA bytes, top row first. It stays until replaced; NULL takes
 * it away. Only the macOS backend shows it so far. */
void oe_set_backdrop(const uint8_t *rgba, int32_t width, int32_t height);

/* The pictures objects are painted with, `count` squares `side` pixels wide, one after
 * another, each of rows of RGBA bytes, top row first. Alpha 0 is a hole; from 128 up, it says
 * how much of the house color shows there: none at 255, all at 128, over the color given.
 * They stay until replaced. Only the macOS backend paints with them so far. */
void oe_set_pictures(const uint8_t *rgba, int32_t side, int32_t count);

/* Plays interleaved float samples, once or over and over if `loop` is 1, replacing anything
 * playing. Only the macOS backend makes a sound so far. */
void oe_play_sound(const float *samples, uint32_t frames, int32_t channels, int32_t rate,
                   int32_t loop);

/* Stops the sound, if any is playing. */
void oe_stop_sound(void);

void oe_shutdown(void);

#endif
