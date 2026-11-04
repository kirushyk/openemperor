/* macOS shim: borderless fullscreen NSWindow + CAMetalLayer. */
#import <Cocoa/Cocoa.h>
#import <Metal/Metal.h>
#import <QuartzCore/CAMetalLayer.h>
#include "oe_shim.h"

/* Shaders compile at startup: the Command Line Tools ship no offline `metal` compiler. */
static NSString *const kShader = @
    "#include <metal_stdlib>\n"
    "using namespace metal;\n"
    "struct In { packed_float2 pos; packed_float4 color; };\n"
    "struct Out { float4 pos [[position]]; float4 color; };\n"
    "vertex Out vs(uint id [[vertex_id]], const device In *v [[buffer(0)]],\n"
    "              constant float2 &size [[buffer(1)]]) {\n"
    "  float2 p = float2(v[id].pos) / size * 2.0 - 1.0;\n"
    "  Out o; o.pos = float4(p.x, -p.y, 0.0, 1.0); o.color = float4(v[id].color); return o;\n"
    "}\n"
    "fragment float4 fs(Out in [[stage_in]]) { return in.color; }\n";

enum { FRAMES_IN_FLIGHT = 3 };

@interface OeWindow : NSWindow
@end
@implementation OeWindow
- (BOOL)canBecomeKeyWindow { return YES; }
- (BOOL)canBecomeMainWindow { return YES; }
@end

static OeWindow *g_window;
static CAMetalLayer *g_layer;
static id<MTLDevice> g_device;
static id<MTLCommandQueue> g_queue;
static id<MTLRenderPipelineState> g_pipeline;
static id<MTLBuffer> g_buffers[FRAMES_IN_FLIGHT];
static dispatch_semaphore_t g_inflight;
static uint32_t g_frame;
static float g_mouse_x, g_mouse_y;
static float g_scale;
static int32_t g_width, g_height;
static BOOL g_captured;
static NSCursor *g_invisible_cursor;

/* Detaching the OS cursor gives raw deltas, so the game cursor stops exactly at the
 * screen edge even when another monitor sits beyond it. */
static void set_captured(BOOL captured) {
    if (captured == g_captured) return;
    g_captured = captured;
    CGAssociateMouseAndMouseCursorPosition(!captured);
    if (captured) {
        CGDisplayHideCursor(kCGDirectMainDisplay);
        [NSCursor hide];
        /* Presentation options only apply to the active app, so they follow activation.
         * The raised level must not persist while inactive: it would cover system dialogs. */
        NSApp.presentationOptions =
            NSApplicationPresentationHideDock | NSApplicationPresentationHideMenuBar;
        g_window.level = NSMainMenuWindowLevel + 1;
    } else {
        CGDisplayShowCursor(kCGDirectMainDisplay);
        [NSCursor unhide];
        [[NSCursor arrowCursor] set];
        g_window.level = NSNormalWindowLevel;
        NSApp.presentationOptions = NSApplicationPresentationDefault;
    }
}

static void update_size(void) {
    NSRect frame = g_window.screen ? g_window.screen.frame : NSScreen.mainScreen.frame;
    g_scale = (float)(g_window.screen ? g_window.screen : NSScreen.mainScreen).backingScaleFactor;
    g_width = (int32_t)(frame.size.width * g_scale);
    g_height = (int32_t)(frame.size.height * g_scale);
    g_layer.contentsScale = g_scale;
    g_layer.drawableSize = CGSizeMake(g_width, g_height);
}

int32_t oe_init(const char *title, int32_t *width, int32_t *height) {
    @autoreleasepool {
        g_device = MTLCreateSystemDefaultDevice();
        if (!g_device) return 0;
        g_queue = [g_device newCommandQueue];

        NSError *error = nil;
        id<MTLLibrary> library = [g_device newLibraryWithSource:kShader options:nil error:&error];
        if (!library) {
            NSLog(@"shader compile failed: %@", error);
            return 0;
        }
        MTLRenderPipelineDescriptor *desc = [MTLRenderPipelineDescriptor new];
        desc.vertexFunction = [library newFunctionWithName:@"vs"];
        desc.fragmentFunction = [library newFunctionWithName:@"fs"];
        MTLRenderPipelineColorAttachmentDescriptor *color = desc.colorAttachments[0];
        color.pixelFormat = MTLPixelFormatBGRA8Unorm;
        color.blendingEnabled = YES;
        color.sourceRGBBlendFactor = MTLBlendFactorSourceAlpha;
        color.destinationRGBBlendFactor = MTLBlendFactorOneMinusSourceAlpha;
        color.sourceAlphaBlendFactor = MTLBlendFactorOne;
        color.destinationAlphaBlendFactor = MTLBlendFactorOneMinusSourceAlpha;
        g_pipeline = [g_device newRenderPipelineStateWithDescriptor:desc error:&error];
        if (!g_pipeline) {
            NSLog(@"pipeline failed: %@", error);
            return 0;
        }
        g_inflight = dispatch_semaphore_create(FRAMES_IN_FLIGHT);

        g_invisible_cursor = [[NSCursor alloc]
            initWithImage:[[NSImage alloc] initWithSize:NSMakeSize(16, 16)]
                  hotSpot:NSZeroPoint];

        [NSApplication sharedApplication];
        NSApp.activationPolicy = NSApplicationActivationPolicyRegular;
        [NSApp finishLaunching];

        NSScreen *screen = NSScreen.mainScreen;
        g_window = [[OeWindow alloc] initWithContentRect:screen.frame
                                               styleMask:NSWindowStyleMaskBorderless
                                                 backing:NSBackingStoreBuffered
                                                   defer:NO
                                                  screen:screen];
        g_window.title = [NSString stringWithUTF8String:title];
        g_window.releasedWhenClosed = NO;
        g_window.acceptsMouseMovedEvents = YES;
        g_window.collectionBehavior = NSWindowCollectionBehaviorFullScreenPrimary;

        g_layer = [CAMetalLayer layer];
        g_layer.device = g_device;
        g_layer.pixelFormat = MTLPixelFormatBGRA8Unorm;
        g_layer.opaque = YES;
        g_window.contentView.wantsLayer = YES;
        g_window.contentView.layer = g_layer;

        [g_window setFrame:screen.frame display:NO];
        [g_window makeKeyAndOrderFront:nil];
        [NSApp activateIgnoringOtherApps:YES];

        update_size();
        g_mouse_x = g_width / 2.0f;
        g_mouse_y = g_height / 2.0f;
        *width = g_width;
        *height = g_height;
        return 1;
    }
}

static int32_t translate(NSEvent *e, OeEvent *out) {
    switch (e.type) {
        case NSEventTypeMouseMoved:
        case NSEventTypeLeftMouseDragged:
        case NSEventTypeRightMouseDragged:
        case NSEventTypeOtherMouseDragged:
            g_mouse_x = fminf(fmaxf(g_mouse_x + (float)e.deltaX * g_scale, 0), g_width - 1);
            g_mouse_y = fminf(fmaxf(g_mouse_y + (float)e.deltaY * g_scale, 0), g_height - 1);
            out->kind = OE_EVENT_MOUSE_MOVE;
            out->x = g_mouse_x;
            out->y = g_mouse_y;
            return 1;
        case NSEventTypeScrollWheel: {
            /* Trackpads report fine pixel deltas; about 40 of them feel like one notch. */
            float notches = (float)e.scrollingDeltaY;
            if (e.hasPreciseScrollingDeltas) notches /= 40.0f;
            if (notches == 0.0f) return 0;
            out->kind = OE_EVENT_WHEEL;
            out->y = notches;
            return 1;
        }
        case NSEventTypeLeftMouseDown:
            out->kind = OE_EVENT_MOUSE_DOWN;
            return 1;
        case NSEventTypeLeftMouseUp:
            out->kind = OE_EVENT_MOUSE_UP;
            return 1;
        case NSEventTypeKeyDown:
            if ((e.modifierFlags & NSEventModifierFlagCommand) &&
                [e.charactersIgnoringModifiers isEqualToString:@"q"]) {
                out->kind = OE_EVENT_QUIT;
                return 1;
            }
            out->kind = OE_EVENT_KEY_DOWN;
            switch (e.keyCode) {
                case 53:
                    if (e.ARepeat) return 0;
                    out->a = OE_KEY_ESCAPE;
                    return 1;
                case 36:
                case 76:
                    if (e.ARepeat) return 0;
                    out->a = OE_KEY_ENTER;
                    return 1;
                case 51:
                    out->a = OE_KEY_BACKSPACE;
                    return 1;
                default:
                    break;
            }
            out->a = OE_KEY_OTHER;
            if (!(e.modifierFlags & (NSEventModifierFlagCommand | NSEventModifierFlagControl)) &&
                e.characters.length == 1) {
                unichar c = [e.characters characterAtIndex:0];
                if (c >= 0x20 && c <= 0x7E) out->b = c;
            }
            return 1;
        default:
            return 0;
    }
}

int32_t oe_poll_event(OeEvent *event) {
    @autoreleasepool {
        *event = (OeEvent){0};

        /* Release the cursor while another app is in front (Cmd-Tab). */
        BOOL active = NSApp.active;
        if (active != g_captured) {
            set_captured(active);
            event->kind = OE_EVENT_FOCUS;
            event->a = active ? 1 : 0;
            return 1;
        }
        /* AppKit can restore the arrow on its own, for example when the Dock or menu bar
         * hides, and hiding alone does not undo that. Reasserting each frame does. */
        if (g_captured) [g_invisible_cursor set];

        int32_t old_w = g_width, old_h = g_height;
        update_size();
        if (old_w != g_width || old_h != g_height) {
            [g_window setFrame:g_window.screen.frame display:NO];
            g_mouse_x = fminf(g_mouse_x, g_width - 1);
            g_mouse_y = fminf(g_mouse_y, g_height - 1);
            event->kind = OE_EVENT_RESIZE;
            event->a = g_width;
            event->b = g_height;
            return 1;
        }

        for (;;) {
            NSEvent *e = [NSApp nextEventMatchingMask:NSEventMaskAny
                                            untilDate:nil
                                               inMode:NSDefaultRunLoopMode
                                              dequeue:YES];
            if (!e) return 0;
            BOOL handled = translate(e, event);
            /* Keys stay ours: forwarding them would only trigger the system beep. */
            if (e.type != NSEventTypeKeyDown && e.type != NSEventTypeKeyUp) {
                [NSApp sendEvent:e];
            }
            if (handled) return 1;
        }
    }
}

void oe_draw(const OeVertex *vertices, uint32_t count, float r, float g, float b) {
    @autoreleasepool {
        id<CAMetalDrawable> drawable = [g_layer nextDrawable];
        if (!drawable) return;

        dispatch_semaphore_wait(g_inflight, DISPATCH_TIME_FOREVER);
        uint32_t slot = g_frame++ % FRAMES_IN_FLIGHT;
        NSUInteger bytes = count * sizeof(OeVertex);
        if (bytes && (!g_buffers[slot] || g_buffers[slot].length < bytes)) {
            g_buffers[slot] = [g_device newBufferWithLength:bytes * 2
                                                    options:MTLResourceStorageModeShared];
        }
        if (bytes) memcpy(g_buffers[slot].contents, vertices, bytes);

        MTLRenderPassDescriptor *pass = [MTLRenderPassDescriptor renderPassDescriptor];
        pass.colorAttachments[0].texture = drawable.texture;
        pass.colorAttachments[0].loadAction = MTLLoadActionClear;
        pass.colorAttachments[0].storeAction = MTLStoreActionStore;
        pass.colorAttachments[0].clearColor = MTLClearColorMake(r, g, b, 1.0);

        id<MTLCommandBuffer> commands = [g_queue commandBuffer];
        id<MTLRenderCommandEncoder> encoder =
            [commands renderCommandEncoderWithDescriptor:pass];
        if (count) {
            float size[2] = {(float)g_width, (float)g_height};
            [encoder setRenderPipelineState:g_pipeline];
            [encoder setVertexBuffer:g_buffers[slot] offset:0 atIndex:0];
            [encoder setVertexBytes:size length:sizeof(size) atIndex:1];
            [encoder drawPrimitives:MTLPrimitiveTypeTriangle vertexStart:0 vertexCount:count];
        }
        [encoder endEncoding];
        dispatch_semaphore_t inflight = g_inflight;
        [commands addCompletedHandler:^(id<MTLCommandBuffer> done) {
            (void)done;
            dispatch_semaphore_signal(inflight);
        }];
        [commands presentDrawable:drawable];
        [commands commit];
    }
}

void oe_shutdown(void) {
    set_captured(NO);
    [g_window close];
}
