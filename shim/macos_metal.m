/* macOS shim: borderless fullscreen NSWindow + CAMetalLayer. */
#import <Cocoa/Cocoa.h>
#import <Metal/Metal.h>
#import <QuartzCore/CAMetalLayer.h>
#import <AudioToolbox/AudioToolbox.h>
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
    /* The backdrop: one rectangle, given by its corners in clip space, made of six corners. */
    "struct ImageOut { float4 pos [[position]]; float2 uv; };\n"
    "vertex ImageOut vs_image(uint id [[vertex_id]], constant float4 &rect [[buffer(1)]]) {\n"
    "  float2 corners[6] = {float2(0, 0), float2(1, 0), float2(1, 1), float2(0, 0), float2(1, 1), float2(0, 1)};\n"
    "  float2 c = corners[id];\n"
    "  ImageOut o; o.pos = float4(mix(rect.xy, rect.zw, c), 0.0, 1.0); o.uv = c; return o;\n"
    "}\n"
    "fragment float4 fs_image(ImageOut in [[stage_in]], texture2d<float> image [[texture(0)]],\n"
    "                         sampler smooth [[sampler(0)]]) {\n"
    "  return float4(image.sample(smooth, in.uv).rgb, 1.0);\n"
    "}\n"
    "struct EffectIn { packed_float3 pos; packed_float4 color; };\n"
    "vertex Out vs_effect(uint id [[vertex_id]], const device EffectIn *v [[buffer(0)]],\n"
    "                     constant float2 &size [[buffer(1)]]) {\n"
    "  float3 at = float3(v[id].pos); float2 p = at.xy / size * 2.0 - 1.0;\n"
    "  Out o; o.pos = float4(p.x, -p.y, at.z, 1.0); o.color = float4(v[id].color); return o;\n"
    "}\n"
    /* A tile's diagonal, in world units: the original game's coordinates, 32 to a tile's side. */
    "constant float TILE_DIAGONAL = 45.254834;\n"
    "struct WorldIn { packed_float3 pos; packed_float3 normal; packed_float3 color; float seen; float level; packed_float3 stain;\n"
    "                 packed_float3 picture; packed_float3 team; };\n"
    "struct WorldOut { float4 pos [[position]]; float3 world; float3 normal; float3 color; float seen; float level; float3 stain;\n"
    "                  float3 picture; float3 team; };\n"
    "struct PointLight { packed_float3 place; float radius; packed_float3 color; float unused; };\n"
    "struct Sky { float4x4 view; float4x4 sun_view; float2 size; float time; int material;\n"
    "             float3 camera; float3 sun; float3 air; float3 sunlight; int mirrored; int light_count;\n"
    "             float2 wash; PointLight lights[16]; };\n"
    "vertex WorldOut vs_world(uint id [[vertex_id]], const device WorldIn *v [[buffer(0)]],\n"
    "                         constant Sky &sky [[buffer(1)]]) {\n"
    "  WorldOut o; o.world = float3(v[id].pos); o.pos = sky.view * float4(o.world, 1.0);\n"
    "  o.normal = float3(v[id].normal); o.color = float3(v[id].color);\n"
    "  o.seen = v[id].seen; o.level = v[id].level; o.stain = float3(v[id].stain);\n"
    "  o.picture = float3(v[id].picture); o.team = float3(v[id].team); return o;\n"
    "}\n"
    "vertex float4 vs_shadow(uint id [[vertex_id]], const device WorldIn *v [[buffer(0)]],\n"
    "                        constant Sky &sky [[buffer(1)]]) {\n"
    "  return sky.sun_view * float4(float3(v[id].pos), 1.0);\n"
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
    "static float3 sand_pixel(float3 base, float2 p, float pixel) {\n"
    "  float speck_fade = 1.0 - smoothstep(0.05, 0.2, pixel);\n"
    "  float2 wind = float2(0.86, 0.5);\n"
    "  float dune = fbm(p * 0.09 + 3.0);\n"
    "  float dune_slope = fbm(p * 0.09 + 3.0 + wind * 0.5) - dune;\n"
    "  float ripple_fade = 1.0 - smoothstep(0.03, 0.11, pixel);\n"
    "  float ripple = 0.0;\n"
    /* Too far off to show, the ripples are not worth their three layers of noise. */
    "  if (ripple_fade > 0.0) {\n"
    "    float bend = fbm(p * 0.22) * 9.0 + fbm(p * 0.9) * 1.3;\n"
    "    float along = dot(p, wind) * 15.0 + bend * 4.0;\n"
    "    float strength = smoothstep(0.25, 0.65, fbm(p * 0.13 + 11.0));\n"
    "    ripple = sin(along + 0.6 * sin(along)) * ripple_fade * (0.25 + 0.75 * strength);\n"
    "  }\n"
    "  float grain_fade = 1.0 - smoothstep(0.012, 0.05, pixel);\n"
    "  float grain = (hash(floor(p * 90.0)) - 0.5) * grain_fade;\n"
    "  float fleck = smoothstep(0.93, 0.98, hash(floor(p * 45.0) + 7.0)) * grain_fade;\n"
    "  float speck = (noise(p * 22.0) - 0.5) * speck_fade;\n"
    "  float light = 1.0 + dune_slope * 1.5 + ripple * 0.075 + grain * 0.13 + speck * 0.12;\n"
    "  float3 warm = base * float3(1.0, 0.95, 0.86), pale = base * float3(1.02, 1.01, 0.97);\n"
    "  float3 color = mix(warm, pale, smoothstep(0.35, 0.65, dune));\n"
    /* The lee of each ripple holds darker heavy grains. */
    "  color *= mix(float3(1.0), float3(0.93, 0.9, 0.84), smoothstep(0.2, 0.9, -ripple));\n"
    "  color = color * light - fleck * 0.11;\n"
    "  return color;\n"
    "}\n"
    /* Three numbers from a point, at once. */
    "static float3 hash3(float2 p) {\n"
    "  float3 q = fract(float3(p.xyx) * float3(0.1031, 0.1030, 0.0973));\n"
    "  q += dot(q, q.yxz + 33.33);\n"
    "  return fract((q.xxy + q.yzz) * q.zyx);\n"
    "}\n"
    /* Two octaves of noise, for broad patches where more would not show. */
    "static float fbm2(float2 p) {\n"
    "  return noise(p) * 0.67 + noise(p * 2.03 + 1.7) * 0.33;\n"
    "}\n"
    /* The nearest of points scattered one to each cell of a grid: how far it is, how far the
     * edge to the next nearest is, and a number of its own. */
    "static float3 cells(float2 p) {\n"
    "  float2 i = floor(p), f = fract(p);\n"
    "  float near = 8.0, next = 8.0, id = 0.0;\n"
    "  for (int y = -1; y <= 1; y++) for (int x = -1; x <= 1; x++) {\n"
    "    float2 g = float2(x, y);\n"
    "    float3 h = hash3(i + g);\n"
    "    float d = length(g + h.xy - f);\n"
    "    if (d < near) { next = near; near = d; id = h.z; }\n"
    "    else if (d < next) next = d;\n"
    "  }\n"
    "  return float3(near, next - near, id);\n"
    "}\n"
    /* How much of a line `width` wide lies at a distance from it, blurred over a pixel so it
     * does not shimmer, and gone once it is finer than a pixel can show. */
    "static float line_at(float distance, float width, float pixel) {\n"
    "  return (1.0 - smoothstep(width, width + pixel * 1.5, distance)) * (1.0 - smoothstep(width * 2.0, width * 6.0, pixel));\n"
    "}\n"
    /* Dried mud: curled plates of earth, each its own shade and pale with dust at its lifted
     * rim, split by dark cracks, with finer cracks inside the bigger plates. */
    "static float3 cracked_earth(float3 base, float2 p, float pixel) {\n"
    "  float2 q = p * 1.8 + (float2(noise(p * 2.2), noise(p * 2.2 + 5.2)) - 0.5) * 0.35;\n"
    "  float3 plate = cells(q);\n"
    "  float scale = pixel * 2.0;\n"
    "  float crack = line_at(plate.y, 0.06, scale);\n"
    "  float fine = 0.0;\n"
    "  if (pixel < 0.05) fine = line_at(cells(q * 2.6 + 3.1).y, 0.03, scale * 2.6) * 0.5;\n"
    "  float rim = smoothstep(0.3, 0.07, plate.y) * (1.0 - crack);\n"
    /* Each plate's edge breaks down into its crack, shaded on its way down. */
    "  float wall = smoothstep(0.14, 0.06, plate.y) * (1.0 - smoothstep(0.03, 0.15, pixel));\n"
    "  float dust = fbm2(p * 3.5);\n"
    "  float3 color = base * (0.84 + plate.z * 0.24) * (0.88 + dust * 0.26);\n"
    "  color = mix(color, base * float3(1.12, 1.07, 0.98), smoothstep(0.55, 0.8, fbm2(p * 0.6 + 9.0)) * 0.45);\n"
    "  color *= (1.0 + rim * 0.16) * (1.0 - wall * 0.35);\n"
    "  color *= 1.0 - fine * 0.4;\n"
    "  if (pixel < 0.08) color *= 1.0 + (noise(p * 40.0) - 0.5) * 0.12 * (1.0 - smoothstep(0.02, 0.08, pixel));\n"
    /* From far off the cracks are too fine to show, but they still darken it. */
    "  color *= 1.0 - smoothstep(0.03, 0.15, pixel) * 0.1;\n"
    "  return mix(color, base * 0.2, crack * 0.92);\n"
    "}\n"
    /* Grass: clumps and blades, drier yellow patches, darker lush ones, bare earth showing
     * through here and there, and a few flowers. */
    "static float3 grass(float3 base, float2 p, float pixel) {\n"
    "  float fade = 1.0 - smoothstep(0.015, 0.06, pixel);\n"
    "  float clumps = fbm2(p * 0.9);\n"
    "  float patch = fbm2(p * 0.16 + 2.0);\n"
    "  float3 color = base * (0.78 + clumps * 0.44);\n"
    "  color = mix(color, base * float3(1.3, 1.12, 0.55), smoothstep(0.55, 0.78, patch) * 0.55);\n"
    "  color = mix(color, base * float3(0.62, 0.78, 0.6), smoothstep(0.42, 0.22, patch) * 0.5);\n"
    "  float2 r = float2(p.x * 0.8 - p.y * 0.6, p.x * 0.6 + p.y * 0.8);\n"
    "  float blades = (noise(r * float2(70.0, 22.0)) - 0.5) + (noise(r * float2(130.0, 40.0) + 3.0) - 0.5) * 0.7;\n"
    "  color *= 1.0 + blades * 0.38 * fade;\n"
    "  float bare = smoothstep(0.7, 0.82, fbm2(p * 0.45 + 20.0));\n"
    "  color = mix(color, float3(0.34, 0.27, 0.17) * (0.8 + fbm2(p * 6.0) * 0.4), bare * 0.75);\n"
    "  float flower = smoothstep(0.985, 0.995, hash3(floor(p * 28.0) + 3.0).x) * fade * (1.0 - bare);\n"
    "  float3 bloom = hash3(floor(p * 28.0) + 3.0).y > 0.5 ? float3(0.95, 0.9, 0.4) : float3(0.95, 0.95, 0.95);\n"
    "  return mix(color, bloom, flower * 0.8);\n"
    "}\n"
    /* Sea ice: clear deeper patches, white pressure ridges along the floes' edges, fine
     * scratches and frost. */
    "static float3 ice(float3 base, float2 p, float pixel) {\n"
    "  float3 color = base * (0.86 + fbm2(p * 0.3) * 0.28);\n"
    "  color = mix(color, base * float3(0.55, 0.76, 0.98), smoothstep(0.5, 0.75, fbm2(p * 0.45 + 4.0)) * 0.55);\n"
    "  float2 q = p * 0.55 + (float2(fbm2(p * 0.9), fbm(p * 0.9 + 3.3)) - 0.5) * 0.6;\n"
    "  float3 floe = cells(q);\n"
    "  float ridge = line_at(floe.y, 0.03, pixel * 0.55);\n"
    "  color = mix(color, float3(0.93, 0.97, 1.0), ridge * 0.75);\n"
    "  float fade = 1.0 - smoothstep(0.02, 0.08, pixel);\n"
    "  float2 r = float2(p.x * 0.9 + p.y * 0.4, p.y * 0.9 - p.x * 0.4);\n"
    "  float scratch = line_at(abs(noise(r * float2(3.0, 40.0)) - 0.5), 0.02, pixel * 20.0) * 0.25;\n"
    "  float frost = (noise(p * 35.0) - 0.5) * fade;\n"
    "  return color * (1.0 + frost * 0.1) + scratch * float3(0.3, 0.35, 0.4);\n"
    "}\n"
    /* Snow: soft drifts lit along their windward sides, blue in their hollows, with wind
     * streaks and glints of sun on crystals. */
    "static float3 snow(float3 base, float2 p, float pixel, float3 sun) {\n"
    "  float e = 0.3;\n"
    "  float h = fbm(p * 0.35);\n"
    "  float2 slope = float2(fbm((p + float2(e, 0)) * 0.35) - h, fbm((p + float2(0, e)) * 0.35) - h) / e;\n"
    "  float lit = clamp(1.0 + dot(slope, -sun.xz) * 0.7, 0.85, 1.12);\n"
    "  float3 color = mix(base * float3(0.84, 0.9, 1.0), base, smoothstep(0.3, 0.65, h)) * lit;\n"
    "  float streak = sin(dot(p, float2(0.8, 0.6)) * 9.0 + fbm2(p * 0.7) * 8.0);\n"
    "  color *= 1.0 + streak * 0.03 * (1.0 - smoothstep(0.03, 0.1, pixel));\n"
    "  float glint = smoothstep(0.996, 1.0, hash3(floor(p * 110.0)).x) * (1.0 - smoothstep(0.006, 0.02, pixel));\n"
    "  return color + glint * 0.45;\n"
    "}\n"
    /* Gravel: pebbles of many shades packed close, dark between them. */
    "static float3 gravel(float3 base, float2 p, float pixel) {\n"
    "  float3 pebble = cells(p * 6.0);\n"
    "  float3 big = cells(p * 1.7 + 7.0);\n"
    "  float fade = 1.0 - smoothstep(0.012, 0.05, pixel);\n"
    "  float3 stone = base * (0.7 + pebble.z * 0.55 * fade + (1.0 - fade) * 0.27);\n"
    "  stone *= 0.9 + big.z * 0.2;\n"
    "  stone *= 1.0 + (0.5 - smoothstep(0.0, 0.45, pebble.x)) * 0.25 * fade;\n"
    "  float gap = (1.0 - smoothstep(0.02, 0.1, pebble.y)) * fade;\n"
    "  float3 color = mix(stone, base * 0.35, gap * 0.8);\n"
    "  return color * (0.88 + fbm2(p * 0.4) * 0.24);\n"
    "}\n"
    /* Slate: slabs split along straight fractures, each its own shade, laid in faint layers
     * and mottled. */
    "static float3 slate(float3 base, float2 p, float pixel) {\n"
    "  float2 q = float2(p.x * 0.55 + p.y * 0.3, p.y * 0.9 - p.x * 0.2);\n"
    "  float3 slab = cells(q);\n"
    "  float layers = sin(dot(p, float2(0.6, 0.8)) * 9.0 + fbm2(p * 0.5) * 5.0);\n"
    "  float3 color = base * (0.82 + slab.z * 0.3) * (0.92 + layers * 0.05) * (0.86 + fbm2(p * 3.0) * 0.28);\n"
    "  float crack = line_at(slab.y, 0.025, pixel * 0.7);\n"
    "  float flake = (noise(p * 25.0) - 0.5) * (1.0 - smoothstep(0.02, 0.08, pixel));\n"
    "  return color * (1.0 + flake * 0.15) * (1.0 - crack * 0.55);\n"
    "}\n"
    /* Moss and lichen: soft patches of several greens, pale crusts, and rusty spots. */
    "static float3 moss(float3 base, float2 p, float pixel) {\n"
    "  float fade = 1.0 - smoothstep(0.015, 0.06, pixel);\n"
    "  float patch = fbm2(p * 1.1);\n"
    "  float3 color = mix(base * float3(0.7, 0.8, 0.62), base * float3(1.18, 1.22, 0.85), smoothstep(0.35, 0.7, patch));\n"
    "  float crust = smoothstep(0.62, 0.7, fbm2(p * 2.4 + 6.0));\n"
    "  color = mix(color, base * float3(1.5, 1.45, 1.15), crust * 0.5);\n"
    "  float3 spot = cells(p * 3.0);\n"
    "  float rust = (1.0 - smoothstep(0.12, 0.2, spot.x)) * step(0.82, spot.z) * fade;\n"
    "  color = mix(color, float3(0.62, 0.36, 0.14), rust * 0.7);\n"
    "  float tuft = (noise(p * 45.0) - 0.5) * fade;\n"
    "  return color * (1.0 + tuft * 0.3);\n"
    "}\n"
    /* Bare rock on mountainsides and pit walls: layered, mottled and cracked. */
    "static float3 rock(float3 base, float2 p, float level, float pixel) {\n"
    "  float layers = sin(level * 14.0 + fbm2(p * 0.8) * 4.0);\n"
    "  float3 block = cells(p * float2(0.9, 1.3));\n"
    "  float crack = line_at(block.y, 0.03, pixel * 1.1);\n"
    "  float mottle = 0.8 + fbm2(p * 5.0) * 0.38 + (noise(p * 30.0) - 0.5) * 0.14 * (1.0 - smoothstep(0.05, 0.2, pixel));\n"
    "  return base * 0.85 * mottle * (0.88 + block.z * 0.2) * (0.92 + layers * 0.08) * (1.0 - crack * 0.5);\n"
    "}\n"
    /* The gentle side of an island: contours of packed scree running across the slope. */
    "static float3 scree(float3 base, float2 p, float level, float pixel) {\n"
    "  float contour = 0.5 + 0.5 * sin(level * 70.0 + (fbm2(p * 1.5) - 0.5) * 3.0);\n"
    "  float grit = (noise(p * 18.0) - 0.5) * (1.0 - smoothstep(0.05, 0.2, pixel));\n"
    "  float pebbles = smoothstep(0.62, 0.72, fbm2(p * 4.0));\n"
    "  return base * (0.84 + contour * 0.24 + grit * 0.3) * (1.0 - pebbles * 0.25);\n"
    "}\n"
    /* The land of each planet: what lies low, what lies on the raised ground above it, broken
     * up where they meet, and rock wherever it is steep or high. */
    "static float3 ground_pixel(int material, float3 base, float2 p, float level, float3 normal, float3 sun) {\n"
    "  float pixel = max(fwidth(p.x), fwidth(p.y));\n"
    "  float up = 1.0;\n"
    "  if (level < 0.5) {\n"
    "    float edge = 0.32 + (fbm2(p * 2.4) - 0.5) * 0.35;\n"
    "    up = level > 0.1 ? smoothstep(edge - 0.06, edge + 0.06, level) : 0.0;\n"
    "  }\n"
    /* Snow lies on the tops; only the steep shows rock there. */
    "  float steep = smoothstep(0.82, 0.62, normal.y);\n"
    "  float high = material == 4 ? 0.0 : smoothstep(0.95, 1.2, level);\n"
    "  float bare = max(steep, high);\n"
    "  if (bare >= 1.0) return rock(base, p, level, pixel);\n"
    "  float3 color;\n"
    "  switch (material) {\n"
    "    case 1: color = up >= 1.0 ? grass(base, p, pixel) : up <= 0.0 ? sand_pixel(base, p, pixel)\n"
    "                   : mix(sand_pixel(base, p, pixel), grass(base, p, pixel), up); break;\n"
    "    case 3: color = up >= 1.0 ? cracked_earth(base, p, pixel) : up <= 0.0 ? sand_pixel(base, p, pixel)\n"
    "                   : mix(sand_pixel(base, p, pixel), cracked_earth(base, p, pixel), up); break;\n"
    "    case 2: color = up >= 1.0 ? cracked_earth(base, p, pixel) : up <= 0.0 ? sand_pixel(base, p, pixel)\n"
    "                   : mix(sand_pixel(base, p, pixel), cracked_earth(base, p, pixel), up); break;\n"
    "    case 4: color = up >= 1.0 ? snow(base, p, pixel, sun) : up <= 0.0 ? ice(base, p, pixel)\n"
    "                   : mix(ice(base, p, pixel), snow(base, p, pixel, sun), up); break;\n"
    "    case 5: color = up >= 1.0 ? slate(base, p, pixel) : up <= 0.0 ? gravel(base, p, pixel)\n"
    "                   : mix(gravel(base, p, pixel), slate(base, p, pixel), up); break;\n"
    "    case 6: color = up >= 1.0 ? slate(base, p, pixel) : up <= 0.0 ? moss(base, p, pixel)\n"
    "                   : mix(moss(base, p, pixel), slate(base, p, pixel), up); break;\n"
    "    default: return base;\n"
    "  }\n"
    /* Flat land has an upright normal: only a slope leans a little, and rock takes the steep. */
    "  float gentle = smoothstep(0.9985, 0.985, normal.y) * (1.0 - smoothstep(0.75, 0.95, level));\n"
    "  if (gentle > 0.0) color = mix(color, scree(base, p, level, pixel), gentle);\n"
    "  return bare > 0.0 ? mix(color, rock(base, p, level, pixel), bare) : color;\n"
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
    "static float3 water_pixel(float3 beach, float2 p, float depth, float in_sun, float2 uv,\n"
    "                          constant Sky &sky, texture2d<float> mirror, sampler smooth) {\n"
    "  float t = sky.time;\n"
    "  float e = 0.05;\n"
    "  float h = waves(p, t);\n"
    "  float2 slope = float2(waves(p + float2(e, 0), t) - h, waves(p + float2(0, e), t) - h) / e;\n"
    "  float3 normal = normalize(float3(-slope.x * 0.22, 1.0, -slope.y * 0.22));\n"
    "  float3 to_eye = normalize(sky.camera / TILE_DIAGONAL - float3(p.x, 0.0, p.y));\n"
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
    "  float3 color = mix(below, above, reflects) * mix(0.62, 1.0, in_sun);\n"
    /* The sun's glitter, and foam along the shore. */
    "  float3 half_way = normalize(sky.sun + to_eye);\n"
    "  float sharp = pow(max(dot(normal, half_way), 0.0), 900.0);\n"
    "  color += float3(1.0, 0.95, 0.82) * sharp * 0.9 * (1.0 - image.a) * in_sun;\n"
    "  float surf = fbm(p * 3.2 + float2(t * 0.2, -t * 0.13));\n"
    "  float line = 0.5 + 0.5 * sin(depth * 26.0 - t * 1.4 + surf * 7.0);\n"
    "  float foam = smoothstep(0.26, 0.0, depth) * smoothstep(0.32, 0.68, surf) * (0.45 + 0.55 * line);\n"
    "  return mix(color, float3(0.95, 0.97, 0.98), foam * 0.85);\n"
    "}\n"
    "/* Lava: plates of black crust drifting on what burns underneath, showing through their cracks. */\n"
    "static float3 lava_pixel(float2 p, float depth, float t) {\n"
    "  float2 drift = float2(fbm(p * 0.18 + t * 0.015), fbm(p * 0.18 + 7.0 - t * 0.012)) * 2.2;\n"
    "  float2 q = p + drift + float2(t * 0.06, t * 0.035);\n"
    "  float plates = fbm(q * 0.55);\n"
    "  float crack = abs(fbm(q * 1.3 + 3.0) - 0.5);\n"
    "  float pulse = 0.12 * sin(t * 0.7 + fbm(p * 0.3) * 9.0);\n"
    "  float heat = smoothstep(0.035, 0.0, crack) * 0.85 + smoothstep(0.42, 0.26, plates) * 0.75 + pulse * 0.5;\n"
    "  /* It cools and crusts over where it meets the land. */\n"
    "  heat *= 0.55 + 0.45 * smoothstep(0.0, 0.3, depth);\n"
    "  float3 crust = float3(0.035, 0.02, 0.018) * (0.7 + fbm(q * 7.0) * 0.8);\n"
    "  float3 color = mix(crust, float3(0.6, 0.06, 0.01), smoothstep(0.12, 0.5, heat));\n"
    "  color = mix(color, float3(1.0, 0.36, 0.04), smoothstep(0.5, 0.85, heat));\n"
    "  return mix(color, float3(1.0, 0.86, 0.38), smoothstep(0.85, 1.2, heat));\n"
    "}\n"
    "static float sun_on(float3 world, float3 normal, constant Sky &sky, depth2d<float> map, sampler compare) {\n"
    "  float4 seen = sky.sun_view * float4(world + normal * 1.81, 1.0);\n"
    "  float2 uv = float2(seen.x * 0.5 + 0.5, 0.5 - seen.y * 0.5);\n"
    "  if (uv.x <= 0.0 || uv.x >= 1.0 || uv.y <= 0.0 || uv.y >= 1.0) return 1.0;\n"
    "  float z = seen.z - 0.0004; float texel = 1.0 / 4096.0; float sum = 0.0;\n"
    "  for (int i = -1; i <= 1; i++) for (int j = -1; j <= 1; j++)\n"
    "    sum += map.sample_compare(compare, uv + float2(i, j) * texel * 1.5, z);\n"
    "  return sum / 9.0;\n"
    "}\n"
    "static float3 light_on(float3 color, float3 world, float3 normal, float in_sun, constant Sky &sky) {\n"
    "  float3 light = sky.air + sky.sunlight * max(dot(normal, sky.sun), 0.0) * in_sun;\n"
    "  for (int i = 0; i < sky.light_count; i++) {\n"
    "    float3 to = float3(sky.lights[i].place) - world;\n"
    "    float reach = length(to) / sky.lights[i].radius;\n"
    "    float fall = max(1.0 - reach * reach, 0.0);\n"
    "    float side = max(dot(normal, normalize(to)) * 0.75 + 0.25, 0.0);\n"
    "    light += float3(sky.lights[i].color) * fall * fall * side;\n"
    "  }\n"
    "  return color * light;\n"
    "}\n"
    /* How a machine shines: paint with a soft wide sheen, bare metal with a sharp highlight
     * from the sun and the sky and ground seen in it, stronger at a glancing angle. */
    "static float3 shine_on(float3 lit, float3 color, float3 world, float3 normal, float in_sun,\n"
    "                       float shine, constant Sky &sky) {\n"
    "  float metal = smoothstep(0.5, 0.9, shine);\n"
    "  float3 to_eye = normalize(sky.camera - world);\n"
    "  float3 halfway = normalize(to_eye + normalize(sky.sun));\n"
    "  float power = mix(12.0, 70.0, metal);\n"
    "  float spec = pow(max(dot(normal, halfway), 0.0), power) * in_sun * shine * mix(0.3, 1.4, metal);\n"
    "  float3 bounce = reflect(-to_eye, normal);\n"
    "  float3 seen = mix(float3(0.2, 0.19, 0.17), float3(0.62, 0.68, 0.78), bounce.y * 0.5 + 0.5);\n"
    "  float glance = pow(1.0 - max(dot(normal, to_eye), 0.0), 3.0);\n"
    "  float3 mirrored = color * seen * 1.1 + seen * glance * 0.35;\n"
    "  return mix(lit, lit * 0.6 + mirrored * 0.7, metal * 0.55) + float3(1.0, 0.96, 0.88) * spec;\n"
    "}\n"
    "fragment float4 fs_world(WorldOut in [[stage_in]], constant Sky &sky [[buffer(0)]],\n"
    "                         texture2d<float> mirror [[texture(0)]], depth2d<float> map [[texture(1)]],\n"
    "                         sampler smooth [[sampler(0)]], sampler compare [[sampler(1)]],\n"
    "                         texture2d_array<float> pictures [[texture(2)]], sampler tiled [[sampler(2)]],\n"
    "                         texture2d<float> wash [[texture(3)]]) {\n"
    "  /* The water's surface and what lies under it show nothing in the mirror. */\n"
    "  if (sky.mirrored != 0 && (in.level < 0.0 || in.world.y < 0.0)) discard_fragment();\n"
    "  float3 normal = normalize(in.normal);\n"
    "  float in_sun = sun_on(in.world, normal, sky, map, compare);\n"
    /* The ground's patterns and how high and deep it lies are laid out in tile diagonals. */
    "  float2 p = in.world.xz / TILE_DIAGONAL; float level = in.level / TILE_DIAGONAL;\n"
    "  float3 color = in.color;\n"
    /* A painted object: its picture's color, the house color over its house parts, and no
     * surface where the picture has a hole. */
    "  if (in.picture.z >= 0.0) {\n"
    "    float4 texel = pictures.sample(tiled, in.picture.xy, uint(in.picture.z + 0.5));\n"
    /* A decal, drawn as material 9, unlit, shows as much of its picture as its level says;
     * one whose level is below 0, a light's glow, adds its picture to what is behind it. */
    "    if (sky.material == 9) {\n"
    "      float cover = texel.a * abs(in.level);\n"
    "      if (cover <= 0.0) discard_fragment();\n"
    "      return float4(texel.rgb * in.seen * cover, in.level < 0.0 ? 0.0 : cover);\n"
    "    }\n"
    "    if (texel.a < 0.25) discard_fragment();\n"
    "    color = texel.rgb + saturate((1.0 - texel.a) * 2.0) * in.team;\n"
    "  }\n"
    /* Black stains of what is damaged, spreading the worse it is, with soot around them. */
    "  float clean = 1.0;\n"
    "  if (in.stain.z > 0.0) {\n"
    "    float blot = fbm(in.stain.xy * 3.4);\n"
    "    float edge = 0.7 - in.stain.z * 0.24;\n"
    "    clean = mix(1.0, 0.72, smoothstep(edge - 0.1, edge, blot));\n"
    "    clean *= mix(1.0, 0.1, smoothstep(edge, edge + 0.05, blot));\n"
    "    color *= clean;\n"
    "  }\n"
    "  /* An invisible unit, as its own side sees it: each pixel grey, blue or pink, changing fast. */\n"
    "  if (in.stain.z < 0.0) {\n"
    "    float frame = fmod(floor(sky.time * 24.0), 509.0);\n"
    "    float pick = hash3(floor(in.pos.xy) + frame * float2(5.0, 11.0)).x;\n"
    "    float3 shimmer = pick < 0.34 ? float3(0.62, 0.62, 0.64)\n"
    "                   : pick < 0.67 ? float3(0.3, 0.5, 1.0) : float3(1.0, 0.45, 0.75);\n"
    "    return float4(shimmer * in.seen, 1.0);\n"
    "  }\n"
    "  if (sky.material == 1 && in.level < 0.0) {\n"
    "    color = water_pixel(color, p, -level, in_sun, in.pos.xy / sky.size, sky, mirror, smooth);\n"
    "    return float4(color * in.seen, 1.0);\n"
    "  }\n"
    "  if (sky.material == 3 && in.level < 0.0) {\n"
    "    return float4(lava_pixel(p, -level, sky.time) * in.seen, 1.0);\n"
    "  }\n"
    "  if (sky.material >= 1 && sky.material <= 6) color = ground_pixel(sky.material, color, p, level, normal, sky.sun);\n"
    "  /* Land still wet from the sea, darker just above it. */\n"
    "  if (sky.material == 1) color *= mix(0.62, 1.0, smoothstep(0.0, 0.12, level));\n"
    "  float3 lit = light_on(color, in.world, normal, in_sun, sky);\n"
    /* The ground lit, as much under its wash as the wash's alpha says. */
    "  if (sky.material == 0 && sky.wash.x > 0.0) {\n"
    "    float4 over = wash.sample(smooth, in.world.xz / sky.wash);\n"
    "    lit = mix(lit, over.rgb, over.a);\n"
    "  }\n"
    /* Objects, drawn as material 8, say how much they shine in their level; soot takes it away. */
    "  if (sky.material == 8 && in.level > 0.0)\n"
    "    lit = shine_on(lit, color, in.world, normal, in_sun, in.level * clean, sky);\n"
    /* Land beside lava glows from below: its shore and its bed, not the low land above. */
    "  if (sky.material == 3) {\n"
    "    float near = smoothstep(0.6, 0.0, in.world.y / TILE_DIAGONAL);\n"
    "    lit += float3(1.0, 0.3, 0.05) * near * near * (0.55 + 0.1 * sin(sky.time * 1.3 + p.x));\n"
    "  }\n"
    "  return float4(lit * in.seen, 1.0);\n"
    "}\n"
    ;

enum { FRAMES_IN_FLIGHT = 3, SHADOW_MAP = 4096 }; /* The game steps its sun view by this map. */
/* What the world's fragments are told they are drawing when it is the objects on the map. */
enum { MATERIAL_OBJECTS = 8 };
/* And when it is the decals on the ground. */
enum { MATERIAL_DECALS = 9 };

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
static id<MTLRenderPipelineState> g_effect_pipeline;
static id<MTLRenderPipelineState> g_world_pipeline;
static id<MTLRenderPipelineState> g_decal_pipeline;
static id<MTLRenderPipelineState> g_shadow_pipeline;
static id<MTLRenderPipelineState> g_image_pipeline;
static id<MTLTexture> g_backdrop;
static id<MTLTexture> g_wash;
static id<MTLDepthStencilState> g_depth_write;
static id<MTLDepthStencilState> g_depth_read;
static id<MTLDepthStencilState> g_depth_ignored;
static id<MTLTexture> g_depth;
static id<MTLTexture> g_mirror;
static id<MTLTexture> g_mirror_depth;
static id<MTLTexture> g_shadow_map;
static id<MTLSamplerState> g_sampler;
static id<MTLSamplerState> g_compare_sampler;
static id<MTLSamplerState> g_tiled_sampler;
static id<MTLTexture> g_pictures;
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

static id<MTLDepthStencilState> depth_state(MTLCompareFunction compare, BOOL write) {
    MTLDepthStencilDescriptor *desc = [MTLDepthStencilDescriptor new];
    desc.depthCompareFunction = compare;
    desc.depthWriteEnabled = write;
    return [g_device newDepthStencilStateWithDescriptor:desc];
}

static id<MTLTexture> depth_texture(NSUInteger w, NSUInteger h, MTLTextureUsage usage) {
    MTLTextureDescriptor *desc =
        [MTLTextureDescriptor texture2DDescriptorWithPixelFormat:MTLPixelFormatDepth32Float
                                                           width:w
                                                          height:h
                                                       mipmapped:NO];
    desc.usage = usage;
    desc.storageMode = MTLStorageModePrivate;
    return [g_device newTextureWithDescriptor:desc];
}

static void blend_premultiplied(MTLRenderPipelineColorAttachmentDescriptor *color) {
    color.pixelFormat = MTLPixelFormatBGRA8Unorm;
    color.blendingEnabled = YES;
    color.sourceRGBBlendFactor = MTLBlendFactorOne;
    color.destinationRGBBlendFactor = MTLBlendFactorOneMinusSourceAlpha;
    color.sourceAlphaBlendFactor = MTLBlendFactorOne;
    color.destinationAlphaBlendFactor = MTLBlendFactorOneMinusSourceAlpha;
}

/* All the pipelines and states the frame goes through. */
static BOOL make_pipelines(id<MTLLibrary> library) {
    NSError *error = nil;
    MTLRenderPipelineDescriptor *desc = [MTLRenderPipelineDescriptor new];
    desc.depthAttachmentPixelFormat = MTLPixelFormatDepth32Float;
    desc.vertexFunction = [library newFunctionWithName:@"vs"];
    desc.fragmentFunction = [library newFunctionWithName:@"fs"];
    blend_premultiplied(desc.colorAttachments[0]);
    g_pipeline = [g_device newRenderPipelineStateWithDescriptor:desc error:&error];
    desc.vertexFunction = [library newFunctionWithName:@"vs_effect"];
    if (g_pipeline) g_effect_pipeline = [g_device newRenderPipelineStateWithDescriptor:desc error:&error];

    desc = [MTLRenderPipelineDescriptor new];
    desc.depthAttachmentPixelFormat = MTLPixelFormatDepth32Float;
    desc.vertexFunction = [library newFunctionWithName:@"vs_world"];
    desc.fragmentFunction = [library newFunctionWithName:@"fs_world"];
    desc.colorAttachments[0].pixelFormat = MTLPixelFormatBGRA8Unorm;
    if (g_effect_pipeline) g_world_pipeline = [g_device newRenderPipelineStateWithDescriptor:desc error:&error];
    blend_premultiplied(desc.colorAttachments[0]);
    if (g_world_pipeline) g_decal_pipeline = [g_device newRenderPipelineStateWithDescriptor:desc error:&error];

    desc = [MTLRenderPipelineDescriptor new];
    desc.depthAttachmentPixelFormat = MTLPixelFormatDepth32Float;
    desc.vertexFunction = [library newFunctionWithName:@"vs_shadow"];
    if (g_decal_pipeline) g_shadow_pipeline = [g_device newRenderPipelineStateWithDescriptor:desc error:&error];

    desc = [MTLRenderPipelineDescriptor new];
    desc.depthAttachmentPixelFormat = MTLPixelFormatDepth32Float;
    desc.vertexFunction = [library newFunctionWithName:@"vs_image"];
    desc.fragmentFunction = [library newFunctionWithName:@"fs_image"];
    desc.colorAttachments[0].pixelFormat = MTLPixelFormatBGRA8Unorm;
    if (g_shadow_pipeline) g_image_pipeline = [g_device newRenderPipelineStateWithDescriptor:desc error:&error];
    if (!g_image_pipeline) {
        NSLog(@"pipeline failed: %@", error);
        return NO;
    }
    g_depth_write = depth_state(MTLCompareFunctionLess, YES);
    g_depth_read = depth_state(MTLCompareFunctionLessEqual, NO);
    g_depth_ignored = depth_state(MTLCompareFunctionAlways, NO);

    MTLSamplerDescriptor *sampler = [MTLSamplerDescriptor new];
    sampler.minFilter = MTLSamplerMinMagFilterLinear;
    sampler.magFilter = MTLSamplerMinMagFilterLinear;
    sampler.sAddressMode = MTLSamplerAddressModeClampToEdge;
    sampler.tAddressMode = MTLSamplerAddressModeClampToEdge;
    g_sampler = [g_device newSamplerStateWithDescriptor:sampler];
    sampler.compareFunction = MTLCompareFunctionLessEqual;
    g_compare_sampler = [g_device newSamplerStateWithDescriptor:sampler];
    MTLSamplerDescriptor *tiled = [MTLSamplerDescriptor new];
    tiled.minFilter = MTLSamplerMinMagFilterLinear;
    tiled.magFilter = MTLSamplerMinMagFilterLinear;
    tiled.mipFilter = MTLSamplerMipFilterLinear;
    tiled.sAddressMode = MTLSamplerAddressModeRepeat;
    tiled.tAddressMode = MTLSamplerAddressModeRepeat;
    g_tiled_sampler = [g_device newSamplerStateWithDescriptor:tiled];
    g_shadow_map = depth_texture(SHADOW_MAP, SHADOW_MAP,
                                 MTLTextureUsageRenderTarget | MTLTextureUsageShaderRead);
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
        if (!make_pipelines(library)) return 0;
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

/* A number key's event, waiting behind the key-down that the same press also makes. */
static OeEvent g_number_key;

/* The 1 to 9 at this place on the number row, whatever the layout prints, or 0. */
static int32_t number_row_digit(unsigned short key_code) {
    switch (key_code) {
        case 18: return 1;
        case 19: return 2;
        case 20: return 3;
        case 21: return 4;
        case 23: return 5;
        case 22: return 6;
        case 26: return 7;
        case 28: return 8;
        case 25: return 9;
        default: return 0;
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
            int32_t digit = number_row_digit(e.keyCode);
            if (digit && !e.ARepeat && !(e.modifierFlags & NSEventModifierFlagCommand)) {
                g_number_key = (OeEvent){.kind = OE_EVENT_NUMBER_KEY, .a = digit};
                g_number_key.b = (e.modifierFlags & NSEventModifierFlagControl) ? OE_HELD_CONTROL
                                 : (e.modifierFlags & NSEventModifierFlagShift) ? OE_HELD_SHIFT
                                                                                : OE_HELD_NONE;
            }
            return 1;
        default:
            return 0;
    }
}

int32_t oe_poll_event(OeEvent *event) {
    @autoreleasepool {
        if (g_number_key.kind != OE_EVENT_NONE) {
            *event = g_number_key;
            g_number_key.kind = OE_EVENT_NONE;
            return 1;
        }
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

/* Matches `Sky` in the shader, whose float3s are 16 bytes each. */
typedef struct {
    float view[16];
    float sun_view[16];
    float size[2];
    float time;
    int32_t material;
    float camera[4];
    float sun[4];
    float air[4];
    float sunlight[4];
    int32_t mirrored;
    int32_t light_count;
    float wash[2];
    OePointLight lights[OE_MAX_LIGHTS];
} OeSky;

enum { LISTS = 6 };

/* Fills a slot's buffer with these lists, one after another, and notes their offsets. */
static id<MTLBuffer> fill_buffer(uint32_t slot, const void *const lists[LISTS],
                                 const NSUInteger bytes[LISTS], NSUInteger offsets[LISTS]) {
    NSUInteger total = 0;
    for (int i = 0; i < LISTS; i++) total += bytes[i];
    if (total && (!g_buffers[slot] || g_buffers[slot].length < total)) {
        g_buffers[slot] = [g_device newBufferWithLength:total * 2
                                                options:MTLResourceStorageModeShared];
    }
    NSUInteger at = 0;
    for (int i = 0; i < LISTS; i++) {
        offsets[i] = at;
        if (bytes[i]) memcpy((char *)g_buffers[slot].contents + at, lists[i], bytes[i]);
        at += bytes[i];
    }
    return g_buffers[slot];
}

static void draw_list(id<MTLRenderCommandEncoder> encoder, id<MTLBuffer> buffer, NSUInteger offset,
                      uint32_t count) {
    if (!count) return;
    [encoder setVertexBuffer:buffer offset:offset atIndex:0];
    [encoder drawPrimitives:MTLPrimitiveTypeTriangle vertexStart:0 vertexCount:count];
}

/* The pictures objects are painted with, or a blank one while there are none. */
static id<MTLTexture> pictures(void) {
    if (!g_pictures) {
        MTLTextureDescriptor *desc = [MTLTextureDescriptor new];
        desc.textureType = MTLTextureType2DArray;
        desc.pixelFormat = MTLPixelFormatRGBA8Unorm;
        desc.width = desc.height = 1;
        desc.arrayLength = 1;
        desc.usage = MTLTextureUsageShaderRead;
        g_pictures = [g_device newTextureWithDescriptor:desc];
    }
    return g_pictures;
}

/* What the world's fragments read, in any pass that draws them. */
static void bind_world(id<MTLRenderCommandEncoder> encoder, id<MTLTexture> mirror, const OeSky *sky,
                       int32_t material) {
    OeSky mine = *sky;
    mine.material = material;
    [encoder setVertexBytes:&mine length:sizeof(mine) atIndex:1];
    [encoder setFragmentBytes:&mine length:sizeof(mine) atIndex:0];
    [encoder setFragmentTexture:mirror atIndex:0];
    [encoder setFragmentTexture:g_shadow_map atIndex:1];
    [encoder setFragmentSamplerState:g_sampler atIndex:0];
    [encoder setFragmentSamplerState:g_compare_sampler atIndex:1];
    [encoder setFragmentTexture:pictures() atIndex:2];
    [encoder setFragmentSamplerState:g_tiled_sampler atIndex:2];
    [encoder setFragmentTexture:g_wash ? g_wash : empty_texture() atIndex:3];
}

/* The depth of all that is solid, as the sun sees it. */
static void encode_shadows(id<MTLCommandBuffer> commands, id<MTLBuffer> buffer,
                           const NSUInteger offsets[LISTS], const OeWorld *world, const OeSky *sky) {
    MTLRenderPassDescriptor *pass = [MTLRenderPassDescriptor renderPassDescriptor];
    pass.depthAttachment.texture = g_shadow_map;
    pass.depthAttachment.loadAction = MTLLoadActionClear;
    pass.depthAttachment.storeAction = MTLStoreActionStore;
    pass.depthAttachment.clearDepth = 1.0;
    id<MTLRenderCommandEncoder> encoder = [commands renderCommandEncoderWithDescriptor:pass];
    [encoder setRenderPipelineState:g_shadow_pipeline];
    [encoder setDepthStencilState:g_depth_write];
    [encoder setDepthBias:1.0 slopeScale:2.0 clamp:0.01];
    [encoder setVertexBytes:sky length:sizeof(*sky) atIndex:1];
    draw_list(encoder, buffer, offsets[0], world->ground_count);
    draw_list(encoder, buffer, offsets[1], world->objects_count);
    draw_list(encoder, buffer, offsets[2], world->casters_count);
    [encoder endEncoding];
}

/* The world seen in the water's mirror, at half the size: its waves blur it anyway. */
static void encode_mirror(id<MTLCommandBuffer> commands, id<MTLBuffer> buffer,
                          const NSUInteger offsets[LISTS], const OeWorld *world, const OeSky *sky,
                          const float size[2]) {
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
        g_mirror_depth = depth_texture(w, h, MTLTextureUsageRenderTarget);
    }
    MTLRenderPassDescriptor *pass = [MTLRenderPassDescriptor renderPassDescriptor];
    pass.colorAttachments[0].texture = g_mirror;
    pass.colorAttachments[0].loadAction = MTLLoadActionClear;
    pass.colorAttachments[0].storeAction = MTLStoreActionStore;
    pass.colorAttachments[0].clearColor = MTLClearColorMake(0, 0, 0, 0);
    pass.depthAttachment.texture = g_mirror_depth;
    pass.depthAttachment.loadAction = MTLLoadActionClear;
    pass.depthAttachment.storeAction = MTLStoreActionDontCare;
    pass.depthAttachment.clearDepth = 1.0;
    OeSky mirrored = *sky;
    memcpy(mirrored.view, world->mirror, sizeof(mirrored.view));
    mirrored.mirrored = 1;
    id<MTLRenderCommandEncoder> encoder = [commands renderCommandEncoderWithDescriptor:pass];
    [encoder setRenderPipelineState:g_world_pipeline];
    [encoder setDepthStencilState:g_depth_write];
    bind_world(encoder, empty_texture(), &mirrored, 0);
    draw_list(encoder, buffer, offsets[0], world->ground_count);
    bind_world(encoder, empty_texture(), &mirrored, MATERIAL_OBJECTS);
    draw_list(encoder, buffer, offsets[1], world->objects_count);
    [encoder endEncoding];
}

/* All of a frame, into a texture. */
static void encode_world(id<MTLCommandBuffer> commands, id<MTLTexture> target, uint32_t slot,
                         const OeWorld *world, float r, float g, float b) {
    const void *const lists[LISTS] = {world->ground, world->objects, world->casters, world->effects,
                                      world->over,   world->decals};
    const NSUInteger bytes[LISTS] = {world->ground_count * sizeof(OeWorldVertex),
                                     world->objects_count * sizeof(OeWorldVertex),
                                     world->casters_count * sizeof(OeWorldVertex),
                                     world->effects_count * sizeof(OeEffectVertex),
                                     world->over_count * sizeof(OeVertex),
                                     world->decals_count * sizeof(OeWorldVertex)};
    NSUInteger offsets[LISTS];
    id<MTLBuffer> buffer = fill_buffer(slot, lists, bytes, offsets);
    float size[2] = {(float)target.width, (float)target.height};

    OeSky sky = {0};
    memcpy(sky.view, world->view, sizeof(sky.view));
    memcpy(sky.sun_view, world->sun_view, sizeof(sky.sun_view));
    sky.size[0] = size[0];
    sky.size[1] = size[1];
    sky.time = world->time;
    memcpy(sky.camera, world->camera, sizeof(world->camera));
    memcpy(sky.sun, world->sun, sizeof(world->sun));
    memcpy(sky.air, world->ambient, sizeof(world->ambient));
    memcpy(sky.sunlight, world->sunlight, sizeof(world->sunlight));
    memcpy(sky.wash, world->wash, sizeof(world->wash));
    if (!g_wash) sky.wash[0] = sky.wash[1] = 0;
    sky.light_count = (int32_t)MIN(world->lights_count, OE_MAX_LIGHTS);
    if (sky.light_count) {
        memcpy(sky.lights, world->lights, sizeof(OePointLight) * (size_t)sky.light_count);
    }

    BOOL has_world = world->ground_count > 0;
    BOOL has_water = has_world && world->material == 1;
    if (has_world) encode_shadows(commands, buffer, offsets, world, &sky);
    if (has_water) encode_mirror(commands, buffer, offsets, world, &sky, size);

    if (!g_depth || g_depth.width != target.width || g_depth.height != target.height) {
        g_depth = depth_texture(target.width, target.height, MTLTextureUsageRenderTarget);
    }
    MTLRenderPassDescriptor *pass = [MTLRenderPassDescriptor renderPassDescriptor];
    pass.colorAttachments[0].texture = target;
    pass.colorAttachments[0].loadAction = MTLLoadActionClear;
    pass.colorAttachments[0].storeAction = MTLStoreActionStore;
    pass.colorAttachments[0].clearColor = MTLClearColorMake(r, g, b, 1.0);
    pass.depthAttachment.texture = g_depth;
    pass.depthAttachment.loadAction = MTLLoadActionClear;
    pass.depthAttachment.storeAction = MTLStoreActionDontCare;
    pass.depthAttachment.clearDepth = 1.0;
    id<MTLRenderCommandEncoder> encoder = [commands renderCommandEncoderWithDescriptor:pass];
    if (has_world) {
        [encoder setRenderPipelineState:g_world_pipeline];
        [encoder setDepthStencilState:g_depth_write];
        bind_world(encoder, has_water ? g_mirror : empty_texture(), &sky, world->material);
        draw_list(encoder, buffer, offsets[0], world->ground_count);
        bind_world(encoder, empty_texture(), &sky, MATERIAL_OBJECTS);
        draw_list(encoder, buffer, offsets[1], world->objects_count);
        /* After all that is solid, which hides what lies behind it, and writing no depth. */
        if (world->decals_count) {
            [encoder setRenderPipelineState:g_decal_pipeline];
            [encoder setDepthStencilState:g_depth_read];
            bind_world(encoder, empty_texture(), &sky, MATERIAL_DECALS);
            draw_list(encoder, buffer, offsets[5], world->decals_count);
        }
    }
    if (g_backdrop) {
        /* As large as fits, keeping its shape, in the middle. */
        float fit = fminf(size[0] / (float)g_backdrop.width, size[1] / (float)g_backdrop.height);
        float half_w = (float)g_backdrop.width * fit / size[0];
        float half_h = (float)g_backdrop.height * fit / size[1];
        float rect[4] = {-half_w, half_h, half_w, -half_h};
        [encoder setRenderPipelineState:g_image_pipeline];
        [encoder setDepthStencilState:g_depth_ignored];
        [encoder setVertexBytes:rect length:sizeof(rect) atIndex:1];
        [encoder setFragmentTexture:g_backdrop atIndex:0];
        [encoder setFragmentSamplerState:g_sampler atIndex:0];
        [encoder drawPrimitives:MTLPrimitiveTypeTriangle vertexStart:0 vertexCount:6];
    }
    [encoder setVertexBytes:size length:sizeof(size) atIndex:1];
    if (world->effects_count) {
        [encoder setRenderPipelineState:g_effect_pipeline];
        [encoder setDepthStencilState:g_depth_read];
        draw_list(encoder, buffer, offsets[3], world->effects_count);
    }
    if (world->over_count) {
        [encoder setRenderPipelineState:g_pipeline];
        [encoder setDepthStencilState:g_depth_ignored];
        draw_list(encoder, buffer, offsets[4], world->over_count);
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

void oe_set_backdrop(const uint8_t *rgba, int32_t width, int32_t height) {
    if (!rgba || width <= 0 || height <= 0) {
        g_backdrop = nil;
        return;
    }
    /* A new texture each time, so that none still being drawn from is written over. */
    MTLTextureDescriptor *desc =
        [MTLTextureDescriptor texture2DDescriptorWithPixelFormat:MTLPixelFormatRGBA8Unorm
                                                           width:(NSUInteger)width
                                                          height:(NSUInteger)height
                                                       mipmapped:NO];
    desc.usage = MTLTextureUsageShaderRead;
    id<MTLTexture> texture = [g_device newTextureWithDescriptor:desc];
    [texture replaceRegion:MTLRegionMake2D(0, 0, (NSUInteger)width, (NSUInteger)height)
               mipmapLevel:0
                 withBytes:rgba
               bytesPerRow:(NSUInteger)width * 4];
    g_backdrop = texture;
}

void oe_set_pictures(const uint8_t *rgba, int32_t side, int32_t count) {
    if (!rgba || side <= 0 || count <= 0) {
        g_pictures = nil;
        return;
    }
    MTLTextureDescriptor *desc = [MTLTextureDescriptor new];
    desc.textureType = MTLTextureType2DArray;
    desc.pixelFormat = MTLPixelFormatRGBA8Unorm;
    desc.width = desc.height = (NSUInteger)side;
    desc.arrayLength = (NSUInteger)count;
    desc.mipmapLevelCount = (NSUInteger)floor(log2((double)side)) + 1;
    desc.usage = MTLTextureUsageShaderRead;
    /* A new texture, so that none still being drawn from is written over. */
    id<MTLTexture> texture = [g_device newTextureWithDescriptor:desc];
    NSUInteger bytes = (NSUInteger)side * (NSUInteger)side * 4;
    for (NSUInteger layer = 0; layer < (NSUInteger)count; layer++) {
        [texture replaceRegion:MTLRegionMake2D(0, 0, (NSUInteger)side, (NSUInteger)side)
                   mipmapLevel:0
                         slice:layer
                     withBytes:rgba + layer * bytes
                   bytesPerRow:(NSUInteger)side * 4
                 bytesPerImage:bytes];
    }
    id<MTLCommandBuffer> commands = [g_queue commandBuffer];
    id<MTLBlitCommandEncoder> blit = [commands blitCommandEncoder];
    [blit generateMipmapsForTexture:texture];
    [blit endEncoding];
    [commands commit];
    g_pictures = texture;
}

void oe_set_wash(const uint8_t *rgba, int32_t width, int32_t height) {
    if (!rgba || width <= 0 || height <= 0) {
        g_wash = nil;
        return;
    }
    MTLTextureDescriptor *desc =
        [MTLTextureDescriptor texture2DDescriptorWithPixelFormat:MTLPixelFormatRGBA8Unorm
                                                           width:(NSUInteger)width
                                                          height:(NSUInteger)height
                                                       mipmapped:NO];
    desc.usage = MTLTextureUsageShaderRead;
    /* A new texture, so that none still being drawn from is written over. */
    id<MTLTexture> texture = [g_device newTextureWithDescriptor:desc];
    [texture replaceRegion:MTLRegionMake2D(0, 0, (NSUInteger)width, (NSUInteger)height)
               mipmapLevel:0
                 withBytes:rgba
               bytesPerRow:(NSUInteger)width * 4];
    g_wash = texture;
}

/* The sound playing: its samples, how many, and how far the queue has been given them. */
static AudioQueueRef g_sound;
static float *g_samples;
static uint32_t g_sample_count, g_samples_queued;
static int32_t g_sound_loops;

/* Gives the queue the next stretch of samples in this buffer, or lets it run dry at the end. */
static void queue_samples(void *user, AudioQueueRef queue, AudioQueueBufferRef buffer) {
    (void)user;
    uint32_t room = buffer->mAudioDataBytesCapacity / sizeof(float);
    if (g_sound_loops && g_samples_queued >= g_sample_count) g_samples_queued = 0;
    uint32_t count = MIN(room, g_sample_count - g_samples_queued);
    if (!count) return;
    memcpy(buffer->mAudioData, g_samples + g_samples_queued, count * sizeof(float));
    g_samples_queued += count;
    buffer->mAudioDataByteSize = count * (uint32_t)sizeof(float);
    AudioQueueEnqueueBuffer(queue, buffer, 0, NULL);
}

void oe_stop_sound(void) {
    if (g_sound) {
        AudioQueueStop(g_sound, true);
        AudioQueueDispose(g_sound, true);
        g_sound = NULL;
    }
    free(g_samples);
    g_samples = NULL;
    g_sample_count = g_samples_queued = 0;
}

void oe_play_sound(const float *samples, uint32_t frames, int32_t channels, int32_t rate,
                   int32_t loop) {
    enum { BUFFERS = 3 };
    oe_stop_sound();
    g_sound_loops = loop;
    if (!samples || !frames || channels <= 0 || rate <= 0) return;
    g_sample_count = frames * (uint32_t)channels;
    g_samples = malloc(g_sample_count * sizeof(float));
    if (!g_samples) return;
    memcpy(g_samples, samples, g_sample_count * sizeof(float));
    uint32_t frame_bytes = (uint32_t)channels * (uint32_t)sizeof(float);
    AudioStreamBasicDescription format = {
        .mSampleRate = rate,
        .mFormatID = kAudioFormatLinearPCM,
        .mFormatFlags = kAudioFormatFlagIsFloat | kAudioFormatFlagIsPacked,
        .mBytesPerPacket = frame_bytes,
        .mFramesPerPacket = 1,
        .mBytesPerFrame = frame_bytes,
        .mChannelsPerFrame = (uint32_t)channels,
        .mBitsPerChannel = 32,
    };
    if (AudioQueueNewOutput(&format, queue_samples, NULL, NULL, NULL, 0, &g_sound) != noErr) {
        g_sound = NULL;
        oe_stop_sound();
        return;
    }
    /* A quarter of a second in each buffer. */
    for (int i = 0; i < BUFFERS; i++) {
        AudioQueueBufferRef buffer;
        if (AudioQueueAllocateBuffer(g_sound, (uint32_t)rate / 4 * frame_bytes, &buffer) == noErr) {
            queue_samples(NULL, g_sound, buffer);
        }
    }
    AudioQueueStart(g_sound, NULL);
}

void oe_shutdown(void) {
    oe_stop_sound();
    set_captured(NO);
    [g_window close];
}
