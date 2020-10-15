use crate::{Backend, Color, Error, Renderer, Settings, Viewport};

use iced_native::{futures, mouse};

/// A fake frame sink for iced powered by `raqote`.
#[allow(missing_debug_implementations)]
pub struct Compositor {
    settings: Settings,
    surfaces: Vec<SurfaceInfo>,
    local_pool: futures::executor::LocalPool,
}

impl Compositor {
    /// Requests a new [`Compositor`] with the given [`Settings`].
    ///
    /// [`Compositor`]: struct.Compositor.html
    /// [`Settings`]: struct.Settings.html
    pub async fn request(settings: Settings) -> Option<Self> {
        let surfaces = vec![];
        let local_pool = futures::executor::LocalPool::new();

        Some(Compositor {
            settings,
            surfaces,
            local_pool,
        })
    }

    /// Creates a new rendering [`Backend`] for this [`Compositor`].
    ///
    /// [`Compositor`]: struct.Compositor.html
    /// [`Backend`]: struct.Backend.html
    pub fn create_backend(&self) -> Backend {
        Backend::new(self.settings)
    }

    fn commit_frame(&self, swap_chain: &SwapChain, frame: raqote::DrawTarget) {
        if let Some(path) = self.settings.output {
            #[allow(unused_results)]
            frame.write_png(path);
        }
    }
}

/// A provider of drawing targets on a backing surface, at a specific
/// width and height.
pub struct SwapChain {
    surface: SurfaceHandle,
    width: u32,
    height: u32,
}

impl SwapChain {
    fn get_current_frame(&mut self) -> Result<raqote::DrawTarget, Error> {
        Ok(raqote::DrawTarget::new(self.width as i32, self.height as i32))
    }
}

impl std::fmt::Debug for SwapChain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SwapChain")
        .field("surface", &self.surface)
        .field("width", &self.width)
        .field("height", &self.height)
        .finish()
    }
}


/// Describes a surface which was issued.
struct SurfaceInfo {
    handle: SurfaceHandle,
    created_at: std::time::Instant,
}

/// Newtype for referring to a specific surface.
#[derive(Debug,Default,Clone,Copy)]
struct SurfaceHandle(usize);

/// Represents a renderable area. Typically, this maps 1:1 with windows.
#[derive(Debug)]
pub struct Surface {
    handle: SurfaceHandle,
}

impl iced_graphics::window::Compositor for Compositor {
    type Settings = Settings;
    type Renderer = Renderer;
    type Surface = Surface;
    type SwapChain = SwapChain;

    fn new(settings: Self::Settings) -> Result<(Self, Renderer), Error> {
        let compositor = futures::executor::block_on(Self::request(settings))
            .ok_or(Error::AdapterNotFound)?;

        let backend = compositor.create_backend();

        Ok((compositor, Renderer::new(backend)))
    }

    fn create_surface<W>(
        &mut self,
        window: &W,
    ) -> Self::Surface {
        let idx = self.surfaces.len();
        self.surfaces.push(SurfaceInfo{
            handle: SurfaceHandle(idx),
            created_at: std::time::Instant::now(),
        });
        Surface{ handle: SurfaceHandle(idx) }
    }

    fn create_swap_chain(
        &mut self,
        surface: &Self::Surface,
        width: u32,
        height: u32,
    ) -> Self::SwapChain {
        let surface = surface.handle;

        let sc = SwapChain{
            surface,
            width,
            height,
        };
        println!("swapchain: {:?}", sc);
        sc
    }

    fn draw<T: AsRef<str>>(
        &mut self,
        renderer: &mut Self::Renderer,
        swap_chain: &mut Self::SwapChain,
        viewport: &Viewport,
        background_color: Color,
        output: &<Self::Renderer as iced_native::Renderer>::Output,
        overlay: &[T],
    ) -> mouse::Interaction {
        let mut frame = swap_chain.get_current_frame().expect("Next frame");

        frame.clear(raqote::SolidSource::from_unpremultiplied_argb(
            (background_color.a * 255.0) as u8,
            (background_color.r * 255.0) as u8,
            (background_color.g * 255.0) as u8,
            (background_color.b * 255.0) as u8,
        ));

        let mouse_interaction = renderer.backend_mut().draw(
            &mut frame,
            viewport,
            output,
            overlay,
        );

        self.commit_frame(&swap_chain, frame);

        self.local_pool.run_until_stalled();
        mouse_interaction
    }
}
