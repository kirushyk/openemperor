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
    OE_EVENT_CONTROL = 10    /* a = 1 when a Control key is pressed, 0 when it is let go */
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

/* A corner of the ground, which a shader details per pixel. */
typedef struct {
    float x, y;
    float r, g, b;
    float seen;
    float world_x, level, world_z;
} OeGroundVertex;

/* A frame of the battlefield. Backends without a ground shader get oe_draw instead. */
typedef struct {
    const OeGroundVertex *ground;
    uint32_t ground_count;
    const OeVertex *mirror;
    uint32_t mirror_count;
    const OeVertex *over;
    uint32_t over_count;
    int32_t material; /* 0 plain, 1 water, 2 sand */
    float time;
    float camera[3];
    float sun[3];
} OeWorld;

/* Draws the ground, then what is over it, and presents. Blocks for vsync. macOS only so far. */
void oe_draw_world(const OeWorld *world, float r, float g, float b);

void oe_shutdown(void);

#endif
