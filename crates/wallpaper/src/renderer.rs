use atelier_common::error::{AtelierError, Result};
use windows::core::Interface;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL, D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_11_1,
};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11RenderTargetView, ID3D11Texture2D,
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_ALPHA_MODE_IGNORE, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    IDXGIDevice, IDXGIFactory2, IDXGISwapChain1, DXGI_PRESENT, DXGI_SWAP_CHAIN_DESC1,
    DXGI_SWAP_EFFECT_FLIP_DISCARD, DXGI_USAGE_RENDER_TARGET_OUTPUT,
};

pub struct D3D11Renderer {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    swap_chain: IDXGISwapChain1,
    render_target_view: Option<ID3D11RenderTargetView>,
    width: u32,
    height: u32,
}

impl D3D11Renderer {
    pub fn new(hwnd: HWND, width: u32, height: u32) -> Result<Self> {
        let feature_levels = [D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0];
        let mut selected_feature_level = D3D_FEATURE_LEVEL::default();
        let mut device_opt: Option<ID3D11Device> = None;
        let mut context_opt: Option<ID3D11DeviceContext> = None;

        unsafe {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                None,
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                Some(&feature_levels),
                D3D11_SDK_VERSION,
                Some(&mut device_opt),
                Some(&mut selected_feature_level),
                Some(&mut context_opt),
            )
        }
        .map_err(|e| AtelierError::Platform(format!("Failed to create D3D11 device: {e}")))?;

        let device =
            device_opt.ok_or_else(|| AtelierError::Platform("D3D11 device is null".to_string()))?;
        let context = context_opt
            .ok_or_else(|| AtelierError::Platform("D3D11 context is null".to_string()))?;

        let dxgi_device: IDXGIDevice = device
            .cast()
            .map_err(|e| AtelierError::Platform(format!("Failed to cast to IDXGIDevice: {e}")))?;

        let dxgi_adapter = unsafe { dxgi_device.GetAdapter() }
            .map_err(|e| AtelierError::Platform(format!("Failed to retrieve DXGI adapter: {e}")))?;

        let dxgi_factory: IDXGIFactory2 = unsafe { dxgi_adapter.GetParent() }
            .map_err(|e| AtelierError::Platform(format!("Failed to retrieve DXGI factory: {e}")))?;

        let swap_chain_desc = DXGI_SWAP_CHAIN_DESC1 {
            Width: width,
            Height: height,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            Stereo: windows::Win32::Foundation::BOOL(0),
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 2,
            Scaling: windows::Win32::Graphics::Dxgi::DXGI_SCALING_STRETCH,
            SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
            AlphaMode: DXGI_ALPHA_MODE_IGNORE,
            Flags: 0,
        };

        let swap_chain = unsafe {
            dxgi_factory.CreateSwapChainForHwnd(&device, hwnd, &swap_chain_desc, None, None)
        }
        .map_err(|e| AtelierError::Platform(format!("Failed to create swap chain: {e}")))?;

        let back_buffer: ID3D11Texture2D = unsafe { swap_chain.GetBuffer(0) }
            .map_err(|e| AtelierError::Platform(format!("Failed to get back buffer: {e}")))?;

        let mut rtv_opt: Option<ID3D11RenderTargetView> = None;
        unsafe { device.CreateRenderTargetView(&back_buffer, None, Some(&mut rtv_opt)) }
            .map_err(|e| AtelierError::Platform(format!("Failed to create RTV: {e}")))?;

        let render_target_view = rtv_opt;

        Ok(Self {
            device,
            context,
            swap_chain,
            render_target_view,
            width,
            height,
        })
    }

    pub fn device(&self) -> &ID3D11Device {
        &self.device
    }

    pub fn context(&self) -> &ID3D11DeviceContext {
        &self.context
    }

    pub fn clear(&self, color: [f32; 4]) {
        if let Some(rtv) = &self.render_target_view {
            unsafe {
                self.context.ClearRenderTargetView(rtv, &color);
            }
        }
    }

    pub fn present(&self) -> Result<()> {
        unsafe { self.swap_chain.Present(1, DXGI_PRESENT(0)) }
            .ok()
            .map_err(|e| AtelierError::Platform(format!("Present failed: {e}")))?;
        Ok(())
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        if width == 0 || height == 0 || (width == self.width && height == self.height) {
            return Ok(());
        }

        unsafe {
            self.context.ClearState();
        }
        self.render_target_view = None;

        unsafe {
            self.swap_chain.ResizeBuffers(
                2,
                width,
                height,
                DXGI_FORMAT_B8G8R8A8_UNORM,
                windows::Win32::Graphics::Dxgi::DXGI_SWAP_CHAIN_FLAG(0),
            )
        }
        .map_err(|e| AtelierError::Platform(format!("Failed to resize buffers: {e}")))?;

        let back_buffer: ID3D11Texture2D = unsafe { self.swap_chain.GetBuffer(0) }
            .map_err(|e| AtelierError::Platform(format!("Failed to get back buffer: {e}")))?;

        let mut rtv_opt: Option<ID3D11RenderTargetView> = None;
        unsafe {
            self.device
                .CreateRenderTargetView(&back_buffer, None, Some(&mut rtv_opt))
        }
        .map_err(|e| AtelierError::Platform(format!("Failed to recreate RTV: {e}")))?;

        self.render_target_view = rtv_opt;
        self.width = width;
        self.height = height;

        Ok(())
    }
}
