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
    "fragment float4 fs(Out in [[stage_in]]) { return in.color; }\n"
    /* The ground: each pixel knows where in the world it is. */
    "struct GroundIn { packed_float2 pos; packed_float4 color; packed_float3 world; };\n"
    "struct GroundOut { float4 pos [[position]]; float4 color; float3 world; };\n"
    "struct Sky { float2 size; float time; int material; float3 camera; float3 sun; };\n"
    "vertex GroundOut vs_ground(uint id [[vertex_id]], const device GroundIn *v [[buffer(0)]],\n"
    "                           constant float2 &size [[buffer(1)]]) {\n"
    "  float2 p = float2(v[id].pos) / size * 2.0 - 1.0;\n"
    "  GroundOut o; o.pos = float4(p.x, -p.y, 0.0, 1.0);\n"
    "  o.color = float4(v[id].color); o.world = float3(v[id].world); return o;\n"
    "}\n"
    "static float hash(float2 p) {\n"
    "  p = fract(p * float2(123.34, 456.21)); p += dot(p, p + 45.32);\n"
    "  return fract(p.x * p.y);\n"
    "}\n"
    "static float noise(float2 p) {\n"
    "  float2 i = floor(p), f = fract(p), u = f * f * (3.0 - 2.0 * f);\n"
    "  return mix(mix(hash(i), hash(i + float2(1, 0)), u.x),\n"
    "             mix(hash(i + float2(0, 1)), hash(i + float2(1, 1)), u.x), u.y);\n"
    "}\n"
    "static float fbm(float2 p) {\n"
    "  float sum = 0.0, part = 0.5;\n"
    "  for (int i = 0; i < 4; i++) {\n"
    "    sum += noise(p) * part; part *= 0.5;\n"
    "    p = float2(p.x * 1.6 - p.y * 1.2, p.x * 1.2 + p.y * 1.6);\n"
    "  }\n"
    "  return sum;\n"
    "}\n"
    /* Wind-laid ripples, dunes and grains, faded out before they get too fine to resolve. */
    "static float3 sand_pixel(float3 base, float2 p, float level, float3 sun) {\n"
    "  float pixel = max(fwidth(p.x), fwidth(p.y));\n"
    "  float2 wind = float2(0.86, 0.5);\n"
    "  float dune = fbm(p * 0.09 + 3.0);\n"
    "  float dune_slope = fbm(p * 0.09 + 3.0 + wind * 0.5) - dune;\n"
    "  float bend = fbm(p * 0.22) * 9.0 + fbm(p * 0.9) * 1.3;\n"
    "  float along = dot(p, wind) * 15.0 + bend * 4.0;\n"
    "  float ripple_fade = 1.0 - smoothstep(0.03, 0.11, pixel);\n"
    "  float strength = smoothstep(0.25, 0.65, fbm(p * 0.13 + 11.0));\n"
    "  float ripple = sin(along + 0.6 * sin(along)) * ripple_fade * (0.25 + 0.75 * strength);\n"
    "  float grain_fade = 1.0 - smoothstep(0.012, 0.05, pixel);\n"
    "  float grain = (hash(floor(p * 90.0)) - 0.5) * grain_fade;\n"
    "  float fleck = smoothstep(0.93, 0.98, hash(floor(p * 45.0) + 7.0)) * grain_fade;\n"
    "  float speck_fade = 1.0 - smoothstep(0.05, 0.2, pixel);\n"
    "  float speck = (noise(p * 22.0) - 0.5) * speck_fade;\n"
    "  float light = 1.0 + dune_slope * 1.5 + ripple * 0.075 + grain * 0.13 + speck * 0.12;\n"
    "  float3 warm = base * float3(1.0, 0.95, 0.86), pale = base * float3(1.02, 1.01, 0.97);\n"
    "  float3 color = mix(warm, pale, smoothstep(0.35, 0.65, dune));\n"
    /* The lee of each ripple holds darker heavy grains. */
    "  color *= mix(float3(1.0), float3(0.93, 0.9, 0.84), smoothstep(0.2, 0.9, -ripple));\n"
    "  color = color * light - fleck * 0.11;\n"
    /* What stands above the sand is rock, broken up where they meet. */
    "  float edge = 0.32 + (fbm(p * 2.4) - 0.5) * 0.35;\n"
    "  float rock = smoothstep(edge - 0.06, edge + 0.06, level);\n"
    "  float crack = smoothstep(0.0, 0.025, abs(fbm(p * 1.7) - 0.5));\n"
    "  float mottle = 0.82 + fbm(p * 5.0) * 0.36 + (noise(p * 30.0) - 0.5) * 0.14 * speck_fade;\n"
    "  float3 stone = base * mottle * mix(0.8, 1.0, crack);\n"
    "  return mix(color, stone, rock);\n"
    "}\n"
    /* The surface's height at a point, in small waves running several ways. */
    "static float waves(float2 p, float t) {\n"
    "  return fbm(p * 0.55 + float2(t * 0.09, t * 0.05)) * 1.0\n"
    "       + fbm(p * 1.6 + float2(-t * 0.17, t * 0.12)) * 0.42\n"
    "       + fbm(p * 4.5 + float2(t * 0.28, -t * 0.34)) * 0.14;\n"
    "}\n"
    "static float3 sky_in(float3 ray, float3 sun) {\n"
    "  float3 sky = mix(float3(0.74, 0.84, 0.92), float3(0.26, 0.46, 0.74), pow(max(ray.y, 0.0), 0.55));\n"
    "  float toward = max(dot(ray, sun), 0.0);\n"
    "  return sky + float3(1.0, 0.92, 0.75) * pow(toward, 24.0) * 0.28;\n"
    "}\n"
    /* The sea: what is under it bent by the waves and dimmed by how deep it is, and over that
     * the mirror image of the world and the sky, more so the flatter it is looked across. */
    "static float3 water_pixel(float3 beach, float3 world, float2 uv, constant Sky &sky,\n"
    "                          texture2d<float> mirror, sampler smooth) {\n"
    "  float2 p = world.xz; float depth = -world.y; float t = sky.time;\n"
    "  float e = 0.05;\n"
    "  float h = waves(p, t);\n"
    "  float2 slope = float2(waves(p + float2(e, 0), t) - h, waves(p + float2(0, e), t) - h) / e;\n"
    "  float3 normal = normalize(float3(-slope.x * 0.22, 1.0, -slope.y * 0.22));\n"
    "  float3 to_eye = normalize(sky.camera - float3(p.x, 0.0, p.y));\n"
    "  float facing = max(dot(normal, to_eye), 0.0);\n"
    "  float fresnel = 0.05 + 0.95 * pow(1.0 - facing, 4.0);\n"
    /* Refraction: the bottom shows displaced along the slope, the further the deeper. */
    "  float2 bent = p + normal.xz * (0.9 + depth * 1.4) - to_eye.xz * depth * 0.28;\n"
    "  float bed = fbm(bent * 1.1) * 0.5 + 0.75 + (noise(bent * 16.0) - 0.5) * 0.08;\n"
    "  float c1 = abs(fbm(bent * 1.5 + float2(t * 0.16, t * 0.09)) - 0.5);\n"
    "  float c2 = abs(fbm(bent * 1.9 + float2(-t * 0.12, t * 0.15) + 5.0) - 0.5);\n"
    "  float caustic = pow(max(1.0 - (c1 + c2) * 7.0, 0.0), 3.0);\n"
    "  float shallow = exp(-depth * 0.9);\n"
    "  float3 bottom = beach * bed + float3(0.9, 0.95, 0.8) * caustic * 0.55 * shallow;\n"
    "  float3 passes = exp(-float3(1.5, 0.42, 0.22) * (depth * 1.3 + 0.1));\n"
    "  float3 deep = float3(0.015, 0.11, 0.2) + float3(0.0, 0.07, 0.09) * (1.0 - facing);\n"
    "  float3 below = bottom * passes + deep * (1.0 - passes);\n"
    /* Reflection: the mirror image, wobbled by the waves, with the sky wherever it is empty. */
    "  float3 ray = reflect(-to_eye, normal);\n"
    "  float2 wobble = normal.xz * float2(0.018, 0.045);\n"
    "  float2 blur = float2(0.0016, 0.003);\n"
    "  float4 image = (mirror.sample(smooth, uv + wobble) * 2.0\n"
    "    + mirror.sample(smooth, uv + wobble + blur)\n"
    "    + mirror.sample(smooth, uv + wobble - blur)\n"
    "    + mirror.sample(smooth, uv + wobble + blur * float2(-1, 1))\n"
    "    + mirror.sample(smooth, uv + wobble + blur * float2(1, -1))) / 6.0;\n"
    "  float3 above = image.rgb + sky_in(ray, sky.sun) * (1.0 - image.a);\n"
    /* A mirror image over water that shows it: never less than a hint of it. */
    "  float reflects = max(fresnel, image.a * 0.55);\n"
    "  float3 color = mix(below, above, reflects);\n"
    /* The sun's glitter, and foam along the shore. */
    "  float3 half_way = normalize(sky.sun + to_eye);\n"
    "  float sharp = pow(max(dot(normal, half_way), 0.0), 900.0);\n"
    "  color += float3(1.0, 0.95, 0.82) * sharp * 0.9 * (1.0 - image.a);\n"
    "  float surf = fbm(p * 3.2 + float2(t * 0.2, -t * 0.13));\n"
    "  float line = 0.5 + 0.5 * sin(depth * 26.0 - t * 1.4 + surf * 7.0);\n"
    "  float foam = smoothstep(0.26, 0.0, depth) * smoothstep(0.32, 0.68, surf) * (0.45 + 0.55 * line);\n"
    "  return mix(color, float3(0.95, 0.97, 0.98), foam * 0.85);\n"
    "}\n"
    "fragment float4 fs_ground(GroundOut in [[stage_in]], constant Sky &sky [[buffer(0)]],\n"
    "                          texture2d<float> mirror [[texture(0)]],\n"
    "                          sampler smooth [[sampler(0)]]) {\n"
    "  float3 color = in.color.rgb;\n"
    "  float2 p = in.world.xz; float level = in.world.y;\n"
    "  if (sky.material == 2) {\n"
    "    color = sand_pixel(color, p, level, sky.sun);\n"
    "  } else if (sky.material == 1 && level < 0.0) {\n"
    "    color = water_pixel(color, in.world, in.pos.xy / sky.size, sky, mirror, smooth);\n"
    "  } else if (sky.material == 1) {\n"
    /* Land still wet from the sea, darker just above it. */
    "    color *= mix(0.62, 1.0, smoothstep(0.0, 0.12, level));\n"
    "  }\n"
    "  return float4(color * in.color.a, 1.0);\n"
    "}\n";

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
static id<MTLRenderPipelineState> g_ground_pipeline;
static id<MTLTexture> g_mirror;
static id<MTLSamplerState> g_sampler;
static id<MTLBuffer> g_buffers[FRAMES_IN_FLIGHT];
static dispatch_semaphore_t g_inflight;
static uint32_t g_frame;
static float g_mouse_x, g_mouse_y;
static float g_scale;
static int32_t g_width, g_height;
static BOOL g_captured;
static NSCursor *g_invisible_cursor;

/* A texture for a frame whose shader must have one bound but will not read it. */
static id<MTLTexture> empty_texture(void) {
    static id<MTLTexture> empty;
    if (!empty) {
        MTLTextureDescriptor *desc =
            [MTLTextureDescriptor texture2DDescriptorWithPixelFormat:MTLPixelFormatBGRA8Unorm
                                                               width:1
                                                              height:1
                                                           mipmapped:NO];
        empty = [g_device newTextureWithDescriptor:desc];
    }
    return empty;
}

static BOOL make_ground_pipeline(id<MTLLibrary> library) {
    NSError *error = nil;
    MTLRenderPipelineDescriptor *desc = [MTLRenderPipelineDescriptor new];
    desc.vertexFunction = [library newFunctionWithName:@"vs_ground"];
    desc.fragmentFunction = [library newFunctionWithName:@"fs_ground"];
    desc.colorAttachments[0].pixelFormat = MTLPixelFormatBGRA8Unorm;
    g_ground_pipeline = [g_device newRenderPipelineStateWithDescriptor:desc error:&error];
    if (!g_ground_pipeline) {
        NSLog(@"ground pipeline failed: %@", error);
        return NO;
    }
    MTLSamplerDescriptor *sampler = [MTLSamplerDescriptor new];
    sampler.minFilter = MTLSamplerMinMagFilterLinear;
    sampler.magFilter = MTLSamplerMinMagFilterLinear;
    sampler.sAddressMode = MTLSamplerAddressModeClampToEdge;
    sampler.tAddressMode = MTLSamplerAddressModeClampToEdge;
    g_sampler = [g_device newSamplerStateWithDescriptor:sampler];
    return YES;
}

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
        color.sourceRGBBlendFactor = MTLBlendFactorOne;
        color.destinationRGBBlendFactor = MTLBlendFactorOneMinusSourceAlpha;
        color.sourceAlphaBlendFactor = MTLBlendFactorOne;
        color.destinationAlphaBlendFactor = MTLBlendFactorOneMinusSourceAlpha;
        g_pipeline = [g_device newRenderPipelineStateWithDescriptor:desc error:&error];
        if (!g_pipeline) {
            NSLog(@"pipeline failed: %@", error);
            return 0;
        }
        if (!make_ground_pipeline(library)) return 0;
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
        case NSEventTypeRightMouseDown:
            out->kind = OE_EVENT_RIGHT_DOWN;
            return 1;
        case NSEventTypeLeftMouseDown:
            out->kind = OE_EVENT_MOUSE_DOWN;
            return 1;
        case NSEventTypeLeftMouseUp:
            out->kind = OE_EVENT_MOUSE_UP;
            out->a = (e.modifierFlags & NSEventModifierFlagShift) ? 1 : 0;
            return 1;
        case NSEventTypeFlagsChanged:
            out->kind = OE_EVENT_CONTROL;
            out->a = (e.modifierFlags & NSEventModifierFlagControl) ? 1 : 0;
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
                case 48:
                    if (e.ARepeat) return 0;
                    out->a = OE_KEY_TAB;
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

/* Fills a slot's buffer with these lists, one after another, and returns their offsets. */
static id<MTLBuffer> fill_buffer(uint32_t slot, const void *const lists[3],
                                 const NSUInteger bytes[3], NSUInteger offsets[3]) {
    NSUInteger total = bytes[0] + bytes[1] + bytes[2];
    if (total && (!g_buffers[slot] || g_buffers[slot].length < total)) {
        g_buffers[slot] = [g_device newBufferWithLength:total * 2
                                                options:MTLResourceStorageModeShared];
    }
    NSUInteger at = 0;
    for (int i = 0; i < 3; i++) {
        offsets[i] = at;
        if (bytes[i]) memcpy((char *)g_buffers[slot].contents + at, lists[i], bytes[i]);
        at += bytes[i];
    }
    return g_buffers[slot];
}

/* The water's reflection, at half the size: its waves blur it anyway. */
static void encode_mirror(id<MTLCommandBuffer> commands, id<MTLBuffer> buffer, NSUInteger offset,
                          uint32_t count, const float size[2]) {
    NSUInteger w = (NSUInteger)size[0] / 2, h = (NSUInteger)size[1] / 2;
    if (!g_mirror || g_mirror.width != w || g_mirror.height != h) {
        MTLTextureDescriptor *desc =
            [MTLTextureDescriptor texture2DDescriptorWithPixelFormat:MTLPixelFormatBGRA8Unorm
                                                               width:w
                                                              height:h
                                                           mipmapped:NO];
        desc.usage = MTLTextureUsageRenderTarget | MTLTextureUsageShaderRead;
        desc.storageMode = MTLStorageModePrivate;
        g_mirror = [g_device newTextureWithDescriptor:desc];
    }
    MTLRenderPassDescriptor *pass = [MTLRenderPassDescriptor renderPassDescriptor];
    pass.colorAttachments[0].texture = g_mirror;
    pass.colorAttachments[0].loadAction = MTLLoadActionClear;
    pass.colorAttachments[0].storeAction = MTLStoreActionStore;
    pass.colorAttachments[0].clearColor = MTLClearColorMake(0, 0, 0, 0);
    id<MTLRenderCommandEncoder> encoder = [commands renderCommandEncoderWithDescriptor:pass];
    [encoder setRenderPipelineState:g_pipeline];
    [encoder setVertexBuffer:buffer offset:offset atIndex:0];
    [encoder setVertexBytes:size length:sizeof(float) * 2 atIndex:1];
    [encoder drawPrimitives:MTLPrimitiveTypeTriangle vertexStart:0 vertexCount:count];
    [encoder endEncoding];
}

/* Matches `Sky` in the shader, whose float3s are 16 bytes each. */
typedef struct {
    float size[2];
    float time;
    int32_t material;
    float camera[4];
    float sun[4];
} OeSky;

/* All of a frame, into a texture of the given size. */
static void encode_world(id<MTLCommandBuffer> commands, id<MTLTexture> target, uint32_t slot,
                         const OeWorld *world, float r, float g, float b) {
    const void *const lists[3] = {world->ground, world->mirror, world->over};
    const NSUInteger bytes[3] = {world->ground_count * sizeof(OeGroundVertex),
                                 world->mirror_count * sizeof(OeVertex),
                                 world->over_count * sizeof(OeVertex)};
    NSUInteger offsets[3];
    id<MTLBuffer> buffer = fill_buffer(slot, lists, bytes, offsets);
    float size[2] = {(float)target.width, (float)target.height};
    if (world->mirror_count) {
        encode_mirror(commands, buffer, offsets[1], world->mirror_count, size);
    }

    MTLRenderPassDescriptor *pass = [MTLRenderPassDescriptor renderPassDescriptor];
    pass.colorAttachments[0].texture = target;
    pass.colorAttachments[0].loadAction = MTLLoadActionClear;
    pass.colorAttachments[0].storeAction = MTLStoreActionStore;
    pass.colorAttachments[0].clearColor = MTLClearColorMake(r, g, b, 1.0);
    id<MTLRenderCommandEncoder> encoder = [commands renderCommandEncoderWithDescriptor:pass];
    if (world->ground_count) {
        OeSky sky = {{size[0], size[1]}, world->time, world->material,
                     {world->camera[0], world->camera[1], world->camera[2], 0},
                     {world->sun[0], world->sun[1], world->sun[2], 0}};
        [encoder setRenderPipelineState:g_ground_pipeline];
        [encoder setVertexBuffer:buffer offset:offsets[0] atIndex:0];
        [encoder setVertexBytes:size length:sizeof(size) atIndex:1];
        [encoder setFragmentBytes:&sky length:sizeof(sky) atIndex:0];
        [encoder setFragmentSamplerState:g_sampler atIndex:0];
        /* A frame with no reflection samples nothing, but a texture must still be bound. */
        if (world->mirror_count) {
            [encoder setFragmentTexture:g_mirror atIndex:0];
        } else {
            [encoder setFragmentTexture:empty_texture() atIndex:0];
        }
        [encoder drawPrimitives:MTLPrimitiveTypeTriangle
                    vertexStart:0
                    vertexCount:world->ground_count];
    }
    if (world->over_count) {
        [encoder setRenderPipelineState:g_pipeline];
        [encoder setVertexBuffer:buffer offset:offsets[2] atIndex:0];
        [encoder setVertexBytes:size length:sizeof(size) atIndex:1];
        [encoder drawPrimitives:MTLPrimitiveTypeTriangle
                    vertexStart:0
                    vertexCount:world->over_count];
    }
    [encoder endEncoding];
}

void oe_draw_world(const OeWorld *world, float r, float g, float b) {
    @autoreleasepool {
        id<CAMetalDrawable> drawable = [g_layer nextDrawable];
        if (!drawable) return;

        dispatch_semaphore_wait(g_inflight, DISPATCH_TIME_FOREVER);
        uint32_t slot = g_frame++ % FRAMES_IN_FLIGHT;
        id<MTLCommandBuffer> commands = [g_queue commandBuffer];
        encode_world(commands, drawable.texture, slot, world, r, g, b);
        dispatch_semaphore_t inflight = g_inflight;
        [commands addCompletedHandler:^(id<MTLCommandBuffer> done) {
            (void)done;
            dispatch_semaphore_signal(inflight);
        }];
        [commands presentDrawable:drawable];
        [commands commit];
    }
}

void oe_draw(const OeVertex *vertices, uint32_t count, float r, float g, float b) {
    OeWorld world = {.over = vertices, .over_count = count};
    oe_draw_world(&world, r, g, b);
}

void oe_shutdown(void) {
    set_captured(NO);
    [g_window close];
}
