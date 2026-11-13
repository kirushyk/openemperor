/* PROTOTYPE, NEVER COMPILED: Windows shim using a borderless fullscreen window and Direct3D 11. */
#define COBJMACROS
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <windowsx.h>
#include <d3d11.h>
#include <dxgi.h>
#include <d3dcompiler.h>
#include "oe_shim.h"

static const char kShader[] =
    "cbuffer C : register(b0) { float2 size; float2 pad; };\n"
    "struct In { float2 pos : POSITION; float4 color : COLOR; };\n"
    "struct Out { float4 pos : SV_Position; float4 color : COLOR; };\n"
    "Out vs(In i) { Out o; float2 p = i.pos / size * 2.0 - 1.0;\n"
    "  o.pos = float4(p.x, -p.y, 0.0, 1.0); o.color = i.color; return o; }\n"
    "float4 ps(Out i) : SV_Target { return i.color; }\n";

enum { QUEUE_SIZE = 256 };

static HWND g_hwnd;
static ID3D11Device *g_device;
static ID3D11DeviceContext *g_context;
static IDXGISwapChain *g_swapchain;
static ID3D11RenderTargetView *g_rtv;
static ID3D11VertexShader *g_vs;
static ID3D11PixelShader *g_ps;
static ID3D11InputLayout *g_layout;
static ID3D11BlendState *g_blend;
static ID3D11RasterizerState *g_raster;
static ID3D11Buffer *g_constants;
static ID3D11Buffer *g_vertices;
static UINT g_vertex_capacity;
static int32_t g_width, g_height;
static OeEvent g_queue[QUEUE_SIZE];
static unsigned g_head, g_tail;

static void push_event(OeEvent e) {
    unsigned next = (g_tail + 1) % QUEUE_SIZE;
    if (next != g_head) {
        g_queue[g_tail] = e;
        g_tail = next;
    }
}

static void confine_cursor(BOOL confine) {
    if (confine) {
        RECT rect;
        GetWindowRect(g_hwnd, &rect);
        ClipCursor(&rect);
    } else {
        ClipCursor(NULL);
    }
}

static void create_rtv(void) {
    ID3D11Texture2D *back = NULL;
    IDXGISwapChain_GetBuffer(g_swapchain, 0, &IID_ID3D11Texture2D, (void **)&back);
    ID3D11Device_CreateRenderTargetView(g_device, (ID3D11Resource *)back, NULL, &g_rtv);
    ID3D11Texture2D_Release(back);
}

static LRESULT CALLBACK wndproc(HWND hwnd, UINT msg, WPARAM wp, LPARAM lp) {
    OeEvent e = {0};
    switch (msg) {
        case WM_CLOSE:
        case WM_DESTROY:
            e.kind = OE_EVENT_QUIT;
            push_event(e);
            return 0;
        case WM_MOUSEMOVE:
            e.kind = OE_EVENT_MOUSE_MOVE;
            e.x = (float)GET_X_LPARAM(lp);
            e.y = (float)GET_Y_LPARAM(lp);
            push_event(e);
            return 0;
        case WM_MOUSEWHEEL:
            e.kind = OE_EVENT_WHEEL;
            e.y = (float)GET_WHEEL_DELTA_WPARAM(wp) / WHEEL_DELTA;
            push_event(e);
            return 0;
        case WM_RBUTTONDOWN:
            e.kind = OE_EVENT_RIGHT_DOWN;
            push_event(e);
            return 0;
        case WM_LBUTTONDOWN:
            SetCapture(hwnd);
            e.kind = OE_EVENT_MOUSE_DOWN;
            push_event(e);
            return 0;
        case WM_LBUTTONUP:
            ReleaseCapture();
            e.kind = OE_EVENT_MOUSE_UP;
            e.a = (wp & MK_SHIFT) ? 1 : 0;
            push_event(e);
            return 0;
        case WM_KEYDOWN:
            if ((lp & (1 << 30)) && wp != VK_BACK) return 0; /* auto-repeat */
            if (wp == VK_CONTROL) {
                e.kind = OE_EVENT_CONTROL;
                e.a = 1;
                push_event(e);
                return 0;
            }
            if (wp >= '1' && wp <= '9') {
                e.kind = OE_EVENT_NUMBER_KEY;
                e.a = (int32_t)(wp - '0');
                e.b = GetKeyState(VK_CONTROL) < 0 ? OE_HELD_CONTROL
                      : GetKeyState(VK_SHIFT) < 0 ? OE_HELD_SHIFT
                                                  : OE_HELD_NONE;
                push_event(e);
                e = (OeEvent){0};
            }
            e.kind = OE_EVENT_KEY_DOWN;
            e.a = wp == VK_ESCAPE ? OE_KEY_ESCAPE
                  : wp == VK_BACK ? OE_KEY_BACKSPACE
                  : wp == VK_RETURN ? OE_KEY_ENTER
                  : wp == VK_TAB ? OE_KEY_TAB
                  : wp == VK_PAUSE ? OE_KEY_PAUSE
                                    : OE_KEY_OTHER;
            push_event(e);
            return 0;
        case WM_KEYUP:
            if (wp != VK_CONTROL) return DefWindowProcA(hwnd, msg, wp, lp);
            e.kind = OE_EVENT_CONTROL;
            e.a = 0;
            push_event(e);
            return 0;
        case WM_CHAR:
            if (wp < 0x20 || wp > 0x7E) return 0;
            e.kind = OE_EVENT_KEY_DOWN;
            e.a = OE_KEY_OTHER;
            e.b = (int32_t)wp;
            push_event(e);
            return 0;
        case WM_ACTIVATE:
            confine_cursor(LOWORD(wp) != WA_INACTIVE);
            e.kind = OE_EVENT_FOCUS;
            e.a = LOWORD(wp) != WA_INACTIVE ? 1 : 0;
            push_event(e);
            return 0;
        case WM_SETCURSOR:
            SetCursor(NULL); /* the game draws its own */
            return TRUE;
        case WM_SIZE:
            if (g_swapchain && wp != SIZE_MINIMIZED) {
                g_width = LOWORD(lp);
                g_height = HIWORD(lp);
                ID3D11DeviceContext_OMSetRenderTargets(g_context, 0, NULL, NULL);
                ID3D11RenderTargetView_Release(g_rtv);
                IDXGISwapChain_ResizeBuffers(g_swapchain, 0, g_width, g_height, DXGI_FORMAT_UNKNOWN, 0);
                create_rtv();
                confine_cursor(TRUE);
                e.kind = OE_EVENT_RESIZE;
                e.a = g_width;
                e.b = g_height;
                push_event(e);
            }
            return 0;
        default:
            return DefWindowProcA(hwnd, msg, wp, lp);
    }
}

static ID3DBlob *compile(const char *entry, const char *target) {
    ID3DBlob *code = NULL, *errors = NULL;
    HRESULT hr = D3DCompile(kShader, sizeof(kShader) - 1, "quad", NULL, NULL, entry, target, 0, 0, &code, &errors);
    if (errors) {
        OutputDebugStringA((const char *)ID3D10Blob_GetBufferPointer(errors));
        ID3D10Blob_Release(errors);
    }
    return SUCCEEDED(hr) ? code : NULL;
}

int32_t oe_init(const char *title, int32_t *width, int32_t *height) {
    SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

    HINSTANCE instance = GetModuleHandleA(NULL);
    WNDCLASSA wc = {0};
    wc.lpfnWndProc = wndproc;
    wc.hInstance = instance;
    wc.lpszClassName = "OpenEmperor";
    if (!RegisterClassA(&wc)) return 0;

    MONITORINFO monitor = {sizeof(monitor)};
    POINT origin = {0, 0};
    GetMonitorInfoA(MonitorFromPoint(origin, MONITOR_DEFAULTTOPRIMARY), &monitor);
    RECT r = monitor.rcMonitor;
    g_width = r.right - r.left;
    g_height = r.bottom - r.top;

    g_hwnd = CreateWindowExA(WS_EX_APPWINDOW, wc.lpszClassName, title, WS_POPUP, r.left, r.top, g_width, g_height,
                             NULL, NULL, instance, NULL);
    if (!g_hwnd) return 0;

    DXGI_SWAP_CHAIN_DESC sd = {0};
    sd.BufferDesc.Width = g_width;
    sd.BufferDesc.Height = g_height;
    sd.BufferDesc.Format = DXGI_FORMAT_B8G8R8A8_UNORM;
    sd.SampleDesc.Count = 1;
    sd.BufferUsage = DXGI_USAGE_RENDER_TARGET_OUTPUT;
    sd.BufferCount = 2;
    sd.OutputWindow = g_hwnd;
    sd.Windowed = TRUE;
    sd.SwapEffect = DXGI_SWAP_EFFECT_FLIP_DISCARD;
    HRESULT hr = D3D11CreateDeviceAndSwapChain(NULL, D3D_DRIVER_TYPE_HARDWARE, NULL, 0, NULL, 0, D3D11_SDK_VERSION,
                                               &sd, &g_swapchain, &g_device, NULL, &g_context);
    if (FAILED(hr)) return 0;
    create_rtv();

    ID3DBlob *vs = compile("vs", "vs_5_0");
    ID3DBlob *ps = compile("ps", "ps_5_0");
    if (!vs || !ps) return 0;
    ID3D11Device_CreateVertexShader(g_device, ID3D10Blob_GetBufferPointer(vs), ID3D10Blob_GetBufferSize(vs), NULL, &g_vs);
    ID3D11Device_CreatePixelShader(g_device, ID3D10Blob_GetBufferPointer(ps), ID3D10Blob_GetBufferSize(ps), NULL, &g_ps);

    D3D11_INPUT_ELEMENT_DESC elements[] = {
        {"POSITION", 0, DXGI_FORMAT_R32G32_FLOAT, 0, 0, D3D11_INPUT_PER_VERTEX_DATA, 0},
        {"COLOR", 0, DXGI_FORMAT_R32G32B32A32_FLOAT, 0, 8, D3D11_INPUT_PER_VERTEX_DATA, 0},
    };
    ID3D11Device_CreateInputLayout(g_device, elements, 2, ID3D10Blob_GetBufferPointer(vs),
                                   ID3D10Blob_GetBufferSize(vs), &g_layout);
    ID3D10Blob_Release(vs);
    ID3D10Blob_Release(ps);

    D3D11_BLEND_DESC blend = {0};
    blend.RenderTarget[0].BlendEnable = TRUE;
    blend.RenderTarget[0].SrcBlend = D3D11_BLEND_ONE;
    blend.RenderTarget[0].DestBlend = D3D11_BLEND_INV_SRC_ALPHA;
    blend.RenderTarget[0].BlendOp = D3D11_BLEND_OP_ADD;
    blend.RenderTarget[0].SrcBlendAlpha = D3D11_BLEND_ONE;
    blend.RenderTarget[0].DestBlendAlpha = D3D11_BLEND_INV_SRC_ALPHA;
    blend.RenderTarget[0].BlendOpAlpha = D3D11_BLEND_OP_ADD;
    blend.RenderTarget[0].RenderTargetWriteMask = D3D11_COLOR_WRITE_ENABLE_ALL;
    ID3D11Device_CreateBlendState(g_device, &blend, &g_blend);

    D3D11_RASTERIZER_DESC raster = {0};
    raster.FillMode = D3D11_FILL_SOLID;
    raster.CullMode = D3D11_CULL_NONE;
    ID3D11Device_CreateRasterizerState(g_device, &raster, &g_raster);

    D3D11_BUFFER_DESC cb = {0};
    cb.ByteWidth = 16;
    cb.Usage = D3D11_USAGE_DYNAMIC;
    cb.BindFlags = D3D11_BIND_CONSTANT_BUFFER;
    cb.CPUAccessFlags = D3D11_CPU_ACCESS_WRITE;
    ID3D11Device_CreateBuffer(g_device, &cb, NULL, &g_constants);

    ShowWindow(g_hwnd, SW_SHOW);
    SetForegroundWindow(g_hwnd);
    confine_cursor(TRUE);
    *width = g_width;
    *height = g_height;
    return 1;
}

int32_t oe_poll_event(OeEvent *event) {
    MSG msg;
    while (PeekMessageA(&msg, NULL, 0, 0, PM_REMOVE)) {
        TranslateMessage(&msg);
        DispatchMessageA(&msg);
    }
    if (g_head == g_tail) return 0;
    *event = g_queue[g_head];
    g_head = (g_head + 1) % QUEUE_SIZE;
    return 1;
}

void oe_draw(const OeVertex *vertices, uint32_t count, float r, float g, float b) {
    if (count > g_vertex_capacity) {
        if (g_vertices) ID3D11Buffer_Release(g_vertices);
        g_vertex_capacity = count * 2;
        D3D11_BUFFER_DESC desc = {0};
        desc.ByteWidth = g_vertex_capacity * sizeof(OeVertex);
        desc.Usage = D3D11_USAGE_DYNAMIC;
        desc.BindFlags = D3D11_BIND_VERTEX_BUFFER;
        desc.CPUAccessFlags = D3D11_CPU_ACCESS_WRITE;
        ID3D11Device_CreateBuffer(g_device, &desc, NULL, &g_vertices);
    }

    D3D11_MAPPED_SUBRESOURCE mapped;
    if (count) {
        ID3D11DeviceContext_Map(g_context, (ID3D11Resource *)g_vertices, 0, D3D11_MAP_WRITE_DISCARD, 0, &mapped);
        memcpy(mapped.pData, vertices, count * sizeof(OeVertex));
        ID3D11DeviceContext_Unmap(g_context, (ID3D11Resource *)g_vertices, 0);
    }
    ID3D11DeviceContext_Map(g_context, (ID3D11Resource *)g_constants, 0, D3D11_MAP_WRITE_DISCARD, 0, &mapped);
    ((float *)mapped.pData)[0] = (float)g_width;
    ((float *)mapped.pData)[1] = (float)g_height;
    ID3D11DeviceContext_Unmap(g_context, (ID3D11Resource *)g_constants, 0);

    float clear[4] = {r, g, b, 1.0f};
    D3D11_VIEWPORT viewport = {0, 0, (float)g_width, (float)g_height, 0, 1};
    UINT stride = sizeof(OeVertex), offset = 0;
    ID3D11DeviceContext_ClearRenderTargetView(g_context, g_rtv, clear);
    ID3D11DeviceContext_OMSetRenderTargets(g_context, 1, &g_rtv, NULL);
    ID3D11DeviceContext_OMSetBlendState(g_context, g_blend, NULL, 0xFFFFFFFF);
    ID3D11DeviceContext_RSSetState(g_context, g_raster);
    ID3D11DeviceContext_RSSetViewports(g_context, 1, &viewport);
    if (count) {
        ID3D11DeviceContext_IASetInputLayout(g_context, g_layout);
        ID3D11DeviceContext_IASetPrimitiveTopology(g_context, D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
        ID3D11DeviceContext_IASetVertexBuffers(g_context, 0, 1, &g_vertices, &stride, &offset);
        ID3D11DeviceContext_VSSetShader(g_context, g_vs, NULL, 0);
        ID3D11DeviceContext_VSSetConstantBuffers(g_context, 0, 1, &g_constants);
        ID3D11DeviceContext_PSSetShader(g_context, g_ps, NULL, 0);
        ID3D11DeviceContext_Draw(g_context, count, 0);
    }
    IDXGISwapChain_Present(g_swapchain, 1, 0);
}

void oe_shutdown(void) {
    ClipCursor(NULL);
    if (g_swapchain) IDXGISwapChain_Release(g_swapchain);
    if (g_context) ID3D11DeviceContext_Release(g_context);
    if (g_device) ID3D11Device_Release(g_device);
    if (g_hwnd) DestroyWindow(g_hwnd);
}

/* No backdrop or sound on this backend yet. */
void oe_set_backdrop(const uint8_t *rgba, int32_t width, int32_t height) {
    (void)rgba;
    (void)width;
    (void)height;
}

void oe_set_pictures(const uint8_t *rgba, int32_t side, int32_t count) {
    (void)rgba;
    (void)side;
    (void)count;
}

void oe_play_sound(const float *samples, uint32_t frames, int32_t channels, int32_t rate,
                   int32_t loop) {
    (void)samples;
    (void)loop;
    (void)frames;
    (void)channels;
    (void)rate;
}

void oe_stop_sound(void) {}
