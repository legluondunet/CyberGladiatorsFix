// SPDX-License-Identifier: GPL-3.0-or-later
// Experimental, native-Windows, FR-only DirectDraw v1 device owned by this
// project. Never modifies the system's COM vtables or the physical display.
// Compiled into the existing WinMM bridge; only a CLI-gated IAT entry can
// instantiate it. This is deliberately not enabled by normal launches.
#define WIN32_LEAN_AND_MEAN
#define DIRECTDRAW_VERSION 0x0700
#include <windows.h>
#include <initguid.h>
#include <ddraw.h>
#include <new>
#include <cstdlib>
#include <cstring>
#include <utility>

extern "C" void __cdecl cyber_virtual_ddraw_trace(const char*);
static void trace(const char *text) { cyber_virtual_ddraw_trace(text); }
static void log_mode(const char *event,DWORD w,DWORD h,DWORD bpp,HRESULT hr) {
    char msg[192];
    wsprintfA(msg,"%s size=%lux%lux%lu hr=0x%08lx physical_mode_request=0",
        event,(unsigned long)w,(unsigned long)h,(unsigned long)bpp,(unsigned long)hr);
    // Use a separate compact trace for HRESULT, avoid relying on
    // undocumented C++ stream initialization in DLL startup.
    trace(msg);
}
static bool valid_dimensions(DWORD w,DWORD h) {
    return w>0 && h>0 && w<=4096 && h<=4096
        && (static_cast<ULONGLONG>(w)*h)<=4096ULL*4096ULL;
}
class VirtualDraw;
class VirtualSurface final: public IDirectDrawSurface {
public:
    VirtualDraw *owner;
    LONG refs;
    DWORD width,height,caps;
    WORD *pixels;
    VirtualSurface *back;
    bool locked;
    VirtualSurface(VirtualDraw *o,DWORD w,DWORD h,DWORD c);
    ~VirtualSurface();
    bool valid() const { return pixels!=nullptr; }
    HRESULT fill_desc(LPDDSURFACEDESC out,bool expose_pixels);
    STDMETHOD(QueryInterface)(REFIID,LPVOID*) override;
    STDMETHOD_(ULONG,AddRef)() override { return static_cast<ULONG>(InterlockedIncrement(&refs)); }
    STDMETHOD_(ULONG,Release)() override;
    STDMETHOD(AddAttachedSurface)(LPDIRECTDRAWSURFACE) override { trace("SURFACE AddAttachedSurface unsupported");return DDERR_UNSUPPORTED; }
    STDMETHOD(AddOverlayDirtyRect)(LPRECT) override { return DDERR_UNSUPPORTED; }
    STDMETHOD(Blt)(LPRECT,LPDIRECTDRAWSURFACE,LPRECT,DWORD,LPDDBLTFX) override;
    STDMETHOD(BltBatch)(LPDDBLTBATCH,DWORD,DWORD) override { return DDERR_UNSUPPORTED; }
    STDMETHOD(BltFast)(DWORD,DWORD,LPDIRECTDRAWSURFACE,LPRECT,DWORD) override;
    STDMETHOD(DeleteAttachedSurface)(DWORD,LPDIRECTDRAWSURFACE) override { return DDERR_UNSUPPORTED; }
    STDMETHOD(EnumAttachedSurfaces)(LPVOID,LPDDENUMSURFACESCALLBACK) override;
    STDMETHOD(EnumOverlayZOrders)(DWORD,LPVOID,LPDDENUMSURFACESCALLBACK) override { return DDERR_UNSUPPORTED; }
    STDMETHOD(Flip)(LPDIRECTDRAWSURFACE,DWORD) override;
    STDMETHOD(GetAttachedSurface)(LPDDSCAPS,LPDIRECTDRAWSURFACE*) override;
    STDMETHOD(GetBltStatus)(DWORD) override { return DD_OK; }
    STDMETHOD(GetCaps)(LPDDSCAPS) override;
    STDMETHOD(GetClipper)(LPDIRECTDRAWCLIPPER*) override { return DDERR_UNSUPPORTED; }
    STDMETHOD(GetColorKey)(DWORD,LPDDCOLORKEY) override { return DDERR_UNSUPPORTED; }
    STDMETHOD(GetDC)(HDC*) override { trace("SURFACE GetDC unsupported");return DDERR_UNSUPPORTED; }
    STDMETHOD(GetFlipStatus)(DWORD) override { return DD_OK; }
    STDMETHOD(GetOverlayPosition)(LPLONG,LPLONG) override { return DDERR_UNSUPPORTED; }
    STDMETHOD(GetPalette)(LPDIRECTDRAWPALETTE*) override { return DDERR_UNSUPPORTED; }
    STDMETHOD(GetPixelFormat)(LPDDPIXELFORMAT) override;
    STDMETHOD(GetSurfaceDesc)(LPDDSURFACEDESC) override;
    STDMETHOD(Initialize)(LPDIRECTDRAW,LPDDSURFACEDESC) override { return DDERR_ALREADYINITIALIZED; }
    STDMETHOD(IsLost)() override { return DD_OK; }
    STDMETHOD(Lock)(LPRECT,LPDDSURFACEDESC,DWORD,HANDLE) override;
    STDMETHOD(ReleaseDC)(HDC) override { return DDERR_UNSUPPORTED; }
    STDMETHOD(Restore)() override { return DD_OK; }
    STDMETHOD(SetClipper)(LPDIRECTDRAWCLIPPER) override { return DDERR_UNSUPPORTED; }
    STDMETHOD(SetColorKey)(DWORD,LPDDCOLORKEY) override { return DDERR_UNSUPPORTED; }
    STDMETHOD(SetOverlayPosition)(LONG,LONG) override { return DDERR_UNSUPPORTED; }
    STDMETHOD(SetPalette)(LPDIRECTDRAWPALETTE) override { return DDERR_UNSUPPORTED; }
    STDMETHOD(Unlock)(LPVOID) override;
    STDMETHOD(UpdateOverlay)(LPRECT,LPDIRECTDRAWSURFACE,LPRECT,DWORD,LPDDOVERLAYFX) override { return DDERR_UNSUPPORTED; }
    STDMETHOD(UpdateOverlayDisplay)(DWORD) override { return DDERR_UNSUPPORTED; }
    STDMETHOD(UpdateOverlayZOrder)(DWORD,LPDIRECTDRAWSURFACE) override { return DDERR_UNSUPPORTED; }
};

class VirtualDraw final: public IDirectDraw {
public:
    IDirectDraw *real;
    LONG refs;
    DWORD width,height,bpp;
    bool cooperative,configured;
    VirtualSurface *primary;
    explicit VirtualDraw(IDirectDraw *source)
       :real(source),refs(1),width(640),height(480),bpp(16),
        cooperative(false),configured(false),primary(nullptr) {}
    ~VirtualDraw() { real->Release(); }
    STDMETHOD(QueryInterface)(REFIID,LPVOID*) override;
    STDMETHOD_(ULONG,AddRef)() override { return static_cast<ULONG>(InterlockedIncrement(&refs)); }
    STDMETHOD_(ULONG,Release)() override {
        ULONG n=static_cast<ULONG>(InterlockedDecrement(&refs));
        if(!n)delete this;
        return n;
    }
    STDMETHOD(Compact)() override { return DD_OK; }
    STDMETHOD(CreateClipper)(DWORD f,LPDIRECTDRAWCLIPPER *p,IUnknown *outer) override { return real->CreateClipper(f,p,outer); }
    STDMETHOD(CreatePalette)(DWORD f,LPPALETTEENTRY entries,LPDIRECTDRAWPALETTE *p,IUnknown *outer) override { return real->CreatePalette(f,entries,p,outer); }
    STDMETHOD(CreateSurface)(LPDDSURFACEDESC,LPDIRECTDRAWSURFACE*,IUnknown*) override;
    STDMETHOD(DuplicateSurface)(LPDIRECTDRAWSURFACE,LPDIRECTDRAWSURFACE*) override {return DDERR_UNSUPPORTED;}
    STDMETHOD(EnumDisplayModes)(DWORD f,LPDDSURFACEDESC d,LPVOID ctx,LPDDENUMMODESCALLBACK cb) override {
        return real->EnumDisplayModes(f,d,ctx,cb);
    }
    STDMETHOD(EnumSurfaces)(DWORD f,LPDDSURFACEDESC d,LPVOID ctx,LPDDENUMSURFACESCALLBACK cb) override {
        return real->EnumSurfaces(f,d,ctx,cb);
    }
    STDMETHOD(FlipToGDISurface)() override {return DD_OK;}
    STDMETHOD(GetCaps)(LPDDCAPS a,LPDDCAPS b) override {return real->GetCaps(a,b);}
    STDMETHOD(GetDisplayMode)(LPDDSURFACEDESC) override;
    STDMETHOD(GetFourCCCodes)(LPDWORD n,LPDWORD c) override {return real->GetFourCCCodes(n,c);}
    STDMETHOD(GetGDISurface)(LPDIRECTDRAWSURFACE*) override;
    STDMETHOD(GetMonitorFrequency)(LPDWORD h) override {return real->GetMonitorFrequency(h);}
    STDMETHOD(GetScanLine)(LPDWORD h) override {return real->GetScanLine(h);}
    STDMETHOD(GetVerticalBlankStatus)(LPBOOL h) override {return real->GetVerticalBlankStatus(h);}
    STDMETHOD(Initialize)(GUID*) override {return DDERR_ALREADYINITIALIZED;}
    STDMETHOD(RestoreDisplayMode)() override {trace("DDRAW_VIRTUAL RestoreDisplayMode physical_mode_change=0");return DD_OK;}
    STDMETHOD(SetCooperativeLevel)(HWND,DWORD) override;
    STDMETHOD(SetDisplayMode)(DWORD,DWORD,DWORD) override;
    STDMETHOD(WaitForVerticalBlank)(DWORD a,HANDLE h) override {return real->WaitForVerticalBlank(a,h);}
};

static void pixel_format(LPDDPIXELFORMAT pf) {
    std::memset(pf,0,sizeof(*pf));
    pf->dwSize=sizeof(*pf);
    pf->dwFlags=DDPF_RGB;
    pf->dwRGBBitCount=16;
    pf->dwRBitMask=0xf800;pf->dwGBitMask=0x07e0;pf->dwBBitMask=0x001f;
}
VirtualSurface::VirtualSurface(VirtualDraw *o,DWORD w,DWORD h,DWORD c)
 :owner(o),refs(1),width(w),height(h),caps(c),pixels(nullptr),
  back(nullptr),locked(false) {
    owner->AddRef();
    if(valid_dimensions(w,h)) {
        pixels=static_cast<WORD*>(std::calloc(static_cast<size_t>(w)*h,sizeof(WORD)));
    }
}
VirtualSurface::~VirtualSurface() {
    if(owner->primary==this)owner->primary=nullptr;
    if(back)back->Release();
    std::free(pixels);
    owner->Release();
}
ULONG VirtualSurface::Release() {
    ULONG n=static_cast<ULONG>(InterlockedDecrement(&refs));
    if(!n)delete this;
    return n;
}
HRESULT VirtualSurface::QueryInterface(REFIID iid,LPVOID *out) {
    if(!out)return E_POINTER;*out=nullptr;
    if(IsEqualIID(iid,IID_IUnknown) || IsEqualIID(iid,IID_IDirectDrawSurface)) {
        *out=static_cast<IDirectDrawSurface*>(this);AddRef();return S_OK;
    }
    trace("SURFACE QueryInterface unsupported IID");
    return E_NOINTERFACE;
}
HRESULT VirtualDraw::QueryInterface(REFIID iid,LPVOID *out) {
    if(!out)return E_POINTER;*out=nullptr;
    if(IsEqualIID(iid,IID_IUnknown) || IsEqualIID(iid,IID_IDirectDraw)) {
        *out=static_cast<IDirectDraw*>(this);AddRef();return S_OK;
    }
    trace("DDRAW_VIRTUAL QueryInterface unsupported IID");
    return E_NOINTERFACE;
}
HRESULT VirtualDraw::SetCooperativeLevel(HWND hwnd,DWORD flags) {
    if(flags!=(DDSCL_FULLSCREEN|DDSCL_EXCLUSIVE)
       && flags!=DDSCL_NORMAL) {
        trace("DDRAW_VIRTUAL unsupported cooperative flags");
        return DDERR_UNSUPPORTED;
    }
    if(!configured) {
        // The system device always sees normal, non-exclusive cooperation.
        HRESULT hr=real->SetCooperativeLevel(hwnd,DDSCL_NORMAL);
        if(FAILED(hr)) {trace("DDRAW_VIRTUAL native DDSCL_NORMAL failed");return hr;}
        configured=true;
    }
    cooperative=true;
    trace("DDRAW_VIRTUAL SetCooperativeLevel requested_exclusive=1 physical_exclusive=0");
    return DD_OK;
}
HRESULT VirtualDraw::SetDisplayMode(DWORD w,DWORD h,DWORD depth) {
    if(!cooperative||depth!=16||!valid_dimensions(w,h)) {
        trace("DDRAW_VIRTUAL SetDisplayMode rejected");
        return DDERR_UNSUPPORTED;
    }
    width=w;height=h;bpp=depth;
    log_mode("DDRAW_VIRTUAL SetDisplayMode logical",w,h,depth,DD_OK);
    return DD_OK; // backed by virtual surfaces; no physical mode request
}
HRESULT VirtualDraw::CreateSurface(LPDDSURFACEDESC desc,LPDIRECTDRAWSURFACE *out,IUnknown *outer) {
    if(!out)return DDERR_INVALIDPARAMS;
    *out=nullptr;
    if(outer || !cooperative || !desc || desc->dwSize!=sizeof(DDSURFACEDESC)
       || !(desc->dwFlags&DDSD_CAPS)) {
        trace("DDRAW_VIRTUAL CreateSurface unsupported descriptor");
        return DDERR_UNSUPPORTED;
    }
    const DWORD requested=desc->ddsCaps.dwCaps;
    const bool is_primary=(requested&DDSCAPS_PRIMARYSURFACE)!=0;
    const bool is_flip=(requested&DDSCAPS_FLIP)!=0;
    const DWORD back_count=(desc->dwFlags&DDSD_BACKBUFFERCOUNT)?desc->dwBackBufferCount:0;
    if(is_primary && (is_flip != (back_count==1))
       ||back_count>1) {
        trace("DDRAW_VIRTUAL CreateSurface unsupported primary chain");
        return DDERR_UNSUPPORTED;
    }
    DWORD w=(desc->dwFlags&DDSD_WIDTH)?desc->dwWidth:width;
    DWORD h=(desc->dwFlags&DDSD_HEIGHT)?desc->dwHeight:height;
    if(!valid_dimensions(w,h))return DDERR_INVALIDPARAMS;
    auto *front=new(std::nothrow) VirtualSurface(this,w,h,requested);
    if(!front)return DDERR_OUTOFMEMORY;
    if(!front->valid()) {front->Release();return DDERR_OUTOFMEMORY;}
    if(is_primary && is_flip){
        auto *back=new(std::nothrow) VirtualSurface(this,w,h,DDSCAPS_BACKBUFFER);
        if(!back||!back->valid()){
            if(back)back->Release();
            front->Release();return DDERR_OUTOFMEMORY;
        }
        front->back=back; // initial reference retained by primary
    }
    if(is_primary)primary=front;
    *out=front;
    char msg[144];
    wsprintfA(msg,"DDRAW_VIRTUAL CreateSurface logical=%lux%lu caps=0x%08lx backbuffers=%lu",
        (unsigned long)w,(unsigned long)h,(unsigned long)requested,(unsigned long)back_count);
    trace(msg);
    return DD_OK;
}
HRESULT VirtualSurface::fill_desc(LPDDSURFACEDESC out,bool reveal) {
    if(!out||out->dwSize<sizeof(DDSURFACEDESC))return DDERR_INVALIDPARAMS;
    DDSURFACEDESC result={};
    result.dwSize=sizeof(result);
    result.dwFlags=DDSD_CAPS|DDSD_WIDTH|DDSD_HEIGHT|DDSD_PITCH|DDSD_PIXELFORMAT;
    result.dwHeight=height;result.dwWidth=width;result.lPitch=static_cast<LONG>(width*2);
    result.ddsCaps.dwCaps=caps;pixel_format(&result.ddpfPixelFormat);
    if(reveal){result.dwFlags|=DDSD_LPSURFACE;result.lpSurface=pixels;}
    *out=result;
    return DD_OK;
}
HRESULT VirtualDraw::GetDisplayMode(LPDDSURFACEDESC out) {
    if(!out||out->dwSize<sizeof(DDSURFACEDESC))return DDERR_INVALIDPARAMS;
    DDSURFACEDESC result={};result.dwSize=sizeof(result);
    result.dwFlags=DDSD_WIDTH|DDSD_HEIGHT|DDSD_PIXELFORMAT;
    result.dwWidth=width;result.dwHeight=height;pixel_format(&result.ddpfPixelFormat);
    *out=result;return DD_OK;
}
HRESULT VirtualDraw::GetGDISurface(LPDIRECTDRAWSURFACE *out) {
    if(!out)return DDERR_INVALIDPARAMS;*out=nullptr;
    if(!primary)return DDERR_NOTFOUND;
    primary->AddRef();*out=primary;return DD_OK;
}
HRESULT VirtualSurface::GetSurfaceDesc(LPDDSURFACEDESC out){return fill_desc(out,false);}
HRESULT VirtualSurface::GetCaps(LPDDSCAPS out){
    if(!out)return DDERR_INVALIDPARAMS;out->dwCaps=caps;return DD_OK;
}
HRESULT VirtualSurface::GetPixelFormat(LPDDPIXELFORMAT out){
    if(!out||out->dwSize<sizeof(DDPIXELFORMAT))return DDERR_INVALIDPARAMS;
    pixel_format(out);return DD_OK;
}
HRESULT VirtualSurface::GetAttachedSurface(LPDDSCAPS wanted,LPDIRECTDRAWSURFACE *out){
    if(!out||!wanted)return DDERR_INVALIDPARAMS;
    *out=nullptr;
    if(!back || (wanted->dwCaps&DDSCAPS_BACKBUFFER)==0)return DDERR_NOTFOUND;
    back->AddRef();*out=back;return DD_OK;
}
HRESULT VirtualSurface::EnumAttachedSurfaces(LPVOID ctx,LPDDENUMSURFACESCALLBACK cb){
    if(!cb)return DDERR_INVALIDPARAMS;if(!back)return DD_OK;
    DDSURFACEDESC desc={};desc.dwSize=sizeof(desc);back->fill_desc(&desc,false);
    back->AddRef();HRESULT result=cb(back,&desc,ctx);back->Release();
    return result==DDENUMRET_CANCEL?DD_OK:DD_OK;
}
HRESULT VirtualSurface::Flip(LPDIRECTDRAWSURFACE target,DWORD flags){
    if(!back||locked||back->locked||target && target!=back
       ||flags & ~(DDFLIP_WAIT|DDFLIP_NOVSYNC)){
        trace("DDRAW_VIRTUAL Flip unsupported/busy");return DDERR_UNSUPPORTED;
    }
    std::swap(pixels,back->pixels);
    return DD_OK;
}
HRESULT VirtualSurface::Lock(LPRECT rect,LPDDSURFACEDESC out,DWORD flags,HANDLE){
    if(rect || flags & ~(DDLOCK_WAIT|DDLOCK_READONLY|DDLOCK_WRITEONLY|DDLOCK_NOSYSLOCK))
        return DDERR_UNSUPPORTED;
    if(locked)return DDERR_SURFACEBUSY;
    HRESULT hr=fill_desc(out,true);
    if(SUCCEEDED(hr))locked=true;
    return hr;
}
HRESULT VirtualSurface::Unlock(LPVOID){
    if(!locked)return DDERR_NOTLOCKED;
    locked=false;return DD_OK;
}
HRESULT VirtualSurface::Blt(LPRECT dst,LPDIRECTDRAWSURFACE source,LPRECT src,
                            DWORD flags,LPDDBLTFX fx){
    if(locked || (back&&back->locked))return DDERR_SURFACEBUSY;
    RECT d={0,0,(LONG)width,(LONG)height};
    if(dst)d=*dst;
    if(d.left<0||d.top<0||d.right>(LONG)width||d.bottom>(LONG)height
       ||d.left>=d.right||d.top>=d.bottom)return DDERR_INVALIDRECT;
    if(flags&DDBLT_COLORFILL){
        if(!fx || fx->dwSize<sizeof(DDBLTFX))return DDERR_INVALIDPARAMS;
        WORD color=static_cast<WORD>(fx->dwFillColor);
        for(LONG y=d.top;y<d.bottom;y++)
            for(LONG x=d.left;x<d.right;x++)pixels[static_cast<size_t>(y)*width+x]=color;
        return DD_OK;
    }
    auto *other=source?dynamic_cast<VirtualSurface*>(source):nullptr;
    if(!other || !other->pixels || other->locked)return DDERR_UNSUPPORTED;
    RECT s={0,0,(LONG)other->width,(LONG)other->height};if(src)s=*src;
    if(s.left<0||s.top<0||s.right>(LONG)other->width||s.bottom>(LONG)other->height
       ||s.left>=s.right||s.top>=s.bottom
       ||d.right-d.left!=s.right-s.left||d.bottom-d.top!=s.bottom-s.top)
        return DDERR_UNSUPPORTED; // no stretching: 3D scales in GameFrame
    for(LONG y=0;y<d.bottom-d.top;y++)
        std::memmove(pixels+(d.top+y)*width+d.left,
             other->pixels+(s.top+y)*other->width+s.left,
             static_cast<size_t>(d.right-d.left)*2);
    return DD_OK;
}
HRESULT VirtualSurface::BltFast(DWORD x,DWORD y,LPDIRECTDRAWSURFACE src,LPRECT rect,DWORD flags){
    if(flags & ~(DDBLTFAST_WAIT))return DDERR_UNSUPPORTED;
    if(!src)return DDERR_INVALIDPARAMS;
    auto *other=dynamic_cast<VirtualSurface*>(src);
    if(!other)return DDERR_UNSUPPORTED;
    RECT s={0,0,(LONG)other->width,(LONG)other->height};
    if(rect)s=*rect;
    RECT d={(LONG)x,(LONG)y,(LONG)(x+s.right-s.left),(LONG)(y+s.bottom-s.top)};
    return Blt(&d,src,&s,0,nullptr);
}
extern "C" HRESULT __cdecl cyber_wrap_existing_ddraw(void **out) {
    if(!out||!*out)return E_POINTER;
    auto *real=static_cast<IDirectDraw*>(*out);
    auto *virtual_device=new(std::nothrow) VirtualDraw(real);
    if(!virtual_device) {
        real->Release();*out=nullptr;
        trace("DDRAW_VIRTUAL allocation failed fail_closed=1");
        return E_OUTOFMEMORY;
    }
    *out=static_cast<IDirectDraw*>(virtual_device);
    trace("DDRAW_VIRTUAL object_created=1 physical_display_mode_unchanged=1");
    return S_OK;
}

// The original 1996 UI thread polls "status movie mode" continuously and
// doesn't ordinarily service GetMessage/PeekMessage during the ~40-second AVI.
// Under non-exclusive desktop composition Windows can ghost that HWND as
// "Not responding". Pump bounded messages on the original MCI/game thread;
// never create a competing GUI thread, and never touch the physical display.
extern "C" DWORD __cdecl cyber_virtual_avi_pump_messages() {
    static volatile LONG inside_pump=0;
    if(InterlockedCompareExchange(&inside_pump,1,0)!=0)return 0;
    DWORD processed=0;
    MSG msg={};
    for(DWORD i=0;i<16;i++){
        if(!PeekMessageA(&msg,nullptr,0,0,PM_REMOVE))break;
        if(msg.message==WM_QUIT){
            // Preserve the requested thread quit: never discard it.
            PostQuitMessage(static_cast<int>(msg.wParam));
            break;
        }
        TranslateMessage(&msg);
        DispatchMessageA(&msg);
        processed++;
    }
    InterlockedExchange(&inside_pump,0);
    return processed;
}
