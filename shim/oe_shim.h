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
    OE_EVENT_MOUSE_UP = 5,   /* left button */
    OE_EVENT_KEY_DOWN = 6,   /* a = OE_KEY_*; b = printable ASCII typed, or 0 */
    OE_EVENT_FOCUS = 7,      /* a = 1 when the game gains focus, 0 when it loses it */
    OE_EVENT_WHEEL = 8       /* y = notches turned, positive away from the player */
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

void oe_shutdown(void);

#endif
