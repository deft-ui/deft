use crate::renderer::Renderer;
use crate::surface::RenderBackend;
use skia_safe::gpu::gl::FramebufferInfo;
use skia_safe::gpu::{BackendTexture, DirectContext, Mipmapped, Protected, Renderable, SurfaceOrigin};
use skia_safe::{gpu, AlphaType, Canvas, ColorType, Image, Surface};
use skia_safe::gpu::surfaces::wrap_backend_texture;
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;
use crate::context::{IRenderContext, RenderContext, UserContext};
use crate::layer::ILayer;

pub struct WebGLRenderer {
    window: Window,
    state: State,
}

extern "C" {
    pub fn emscripten_GetProcAddress(
        name: *const ::std::os::raw::c_char,
    ) -> *const ::std::os::raw::c_void;
}

struct GpuState {
    context: DirectContext,
    framebuffer_info: FramebufferInfo,
}

/// This struct holds the state of the Rust application between JS calls.
///
/// It is created by [init] and passed to the other exported functions. Note that rust-skia data
/// structures are not thread safe, so a state must not be shared between different Web Workers.
pub struct State {
    gpu_state: GpuState,
    surface: Surface,
}

impl State {
    fn new(gpu_state: GpuState, surface: Surface) -> Self {
        State { gpu_state, surface }
    }

    fn set_surface(&mut self, surface: Surface) {
        self.surface = surface;
    }
}

/// Load GL functions pointers from JavaScript so we can call OpenGL functions from Rust.
///
/// This only needs to be done once.
fn init_gl() {
    unsafe {
        gl::load_with(|addr| {
            let addr = std::ffi::CString::new(addr).unwrap();
            emscripten_GetProcAddress(addr.into_raw() as *const _) as *const _
        });
    }
}

/// Create the GPU state from the JavaScript WebGL context.
///
/// This needs to be done once per WebGL context.
fn create_gpu_state() -> GpuState {
    let interface = skia_safe::gpu::gl::Interface::new_native();
    let interface = interface.unwrap();
    let context = skia_safe::gpu::direct_contexts::make_gl(interface, None).unwrap();
    let framebuffer_info = {
        let mut fboid: gl::types::GLint = 0;
        unsafe { gl::GetIntegerv(gl::FRAMEBUFFER_BINDING, &mut fboid) };

        FramebufferInfo {
            fboid: fboid.try_into().unwrap(),
            format: skia_safe::gpu::gl::Format::RGBA8.into(),
            protected: skia_safe::gpu::Protected::No,
        }
    };

    GpuState {
        context,
        framebuffer_info,
    }
}

/// Create the Skia surface that will be used for rendering.
fn create_surface(gpu_state: &mut GpuState, width: i32, height: i32) -> Surface {
    let backend_render_target =
        gpu::backend_render_targets::make_gl((width, height), 1, 8, gpu_state.framebuffer_info);

    gpu::surfaces::wrap_backend_render_target(
        &mut gpu_state.context,
        &backend_render_target,
        skia_safe::gpu::SurfaceOrigin::BottomLeft,
        skia_safe::ColorType::RGBA8888,
        None,
        None,
    )
    .unwrap()
}

/// Initialize the renderer.
///
/// This is called from JS after the WebGL context has been created.
fn init(width: i32, height: i32) -> State {
    let mut gpu_state = create_gpu_state();
    let surface = create_surface(&mut gpu_state, width, height);
    let state = State::new(gpu_state, surface);
    state
}

/// Resize the Skia surface
///
/// This is called from JS when the window is resized.
/// # Safety
fn resize_surface(state: &mut State, width: i32, height: i32) {
    // let state = unsafe { state.as_mut() }.expect("got an invalid state pointer");
    let surface = create_surface(&mut state.gpu_state, width, height);
    state.set_surface(surface);
}

impl WebGLRenderer {
    pub fn new(_event_loop: &ActiveEventLoop, window: Window) -> Option<Self> {
        init_gl();
        let size = window.inner_size();
        let state = init(size.width as i32, size.height as i32);
        Some(WebGLRenderer { window, state })
    }
}

#[derive(Clone)]
pub struct WebGLRenderContext {
    pub gr_context: gpu::DirectContext,
}

impl WebGLRenderContext {
    pub fn new(gr_context: DirectContext) -> Self {
        Self {
            gr_context
        }
    }
}

impl IRenderContext for WebGLRenderContext {
    fn create_layer(
        &mut self,
        width: usize,
        height: usize,
    ) -> Option<Box<dyn ILayer>> {
        let backend_texture = self
            .gr_context
            .create_backend_texture(
                width as i32,
                height as i32,
                ColorType::RGBA8888,
                Mipmapped::No,
                Renderable::Yes,
                Protected::No,
                "layer",
            )
            .unwrap();
        // println!("texture created: {:?}", bt.gl_texture_info());
        let img = Image::from_texture(
            &mut self.gr_context,
            &backend_texture,
            SurfaceOrigin::BottomLeft,
            ColorType::RGBA8888,
            AlphaType::Premul,
            None,
        )?;
        let surface = wrap_backend_texture(
            &mut self.gr_context,
            &backend_texture,
            SurfaceOrigin::BottomLeft,
            None,
            ColorType::RGBA8888,
            None,
            None,
        )?;
        let layer = GlLayer::new(self.gr_context.clone(), backend_texture, img, surface);
        Some(Box::new(layer))
    }

    fn flush(&mut self) {
        self.gr_context.flush_and_submit();
    }
}

pub struct GlLayer {
    context: DirectContext,
    image: Image,
    surface: Surface,
    backend_texture: BackendTexture,
}

impl GlLayer {
    pub fn new(
        context: DirectContext,
        backend_texture: BackendTexture,
        image: Image,
        surface: Surface,
    ) -> Self {
        Self {
            context,
            image,
            surface,
            backend_texture,
        }
    }
}

impl Drop for GlLayer {
    fn drop(&mut self) {
        self.context.delete_backend_texture(&self.backend_texture);
    }
}

impl ILayer for GlLayer {
    fn canvas(&mut self) -> &Canvas {
        &mut self.surface.canvas()
    }

    fn as_image(&mut self) -> Image {
        self.image.clone()
    }
}


impl RenderBackend for WebGLRenderer {
    fn window(&self) -> &Window {
        &self.window
    }

    fn render(&mut self, renderer: Renderer, callback: Box<dyn FnOnce(bool) + Send + 'static>) {
        let gr_context = self.state.gpu_state.context.clone();
        let state = &mut self.state;
        let mut user_context = UserContext::new();
        let mut rc = WebGLRenderContext::new(gr_context.clone());
        let mut ctx = RenderContext::new(&mut rc, &mut user_context);
        renderer.render(&state.surface.canvas(), &mut ctx);
        state
            .gpu_state
            .context
            .flush_and_submit_surface(&mut state.surface, None);
        callback(true);
    }

    fn resize(&mut self, width: u32, height: u32) {
        resize_surface(&mut self.state, width as i32, height as i32)
    }
}
