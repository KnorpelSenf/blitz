use anyrender_vello::{CustomPaintCtx, CustomPaintSource, TextureHandle};
use blitz_dom::BaseDocument;
use blitz_traits::{ColorScheme, Devtools, Viewport};
use color::{OpaqueColor, Srgb};
use log::{info, Level};
use raw_window_handle::{
    DisplayHandle, HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle,
    WindowHandle, XlibDisplayHandle, XlibWindowHandle,
};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Instant;
use std::{panic, ptr::NonNull, sync::Arc};
use wasm_bindgen::prelude::wasm_bindgen;
use wgpu::{Device, Queue};

struct WasmWindowHandle {
    display_handle: u32,
    screen_handle: i32,
    window_handle: u32,
}
impl WasmWindowHandle {
    fn from_window(display_handle: u32, screen_handle: i32, window_handle: u32) -> Self {
        WasmWindowHandle {
            display_handle,
            screen_handle,
            window_handle,
        }
    }
}

impl HasWindowHandle for WasmWindowHandle {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        let raw = RawWindowHandle::Xlib(XlibWindowHandle::new(self.window_handle.into()));
        let handle = unsafe { WindowHandle::borrow_raw(raw) };
        Ok(handle)
    }
}
impl HasDisplayHandle for WasmWindowHandle {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        let pointer = unsafe { NonNull::new_unchecked(self.display_handle as *mut _) };
        let raw = RawDisplayHandle::Xlib(XlibDisplayHandle::new(Some(pointer), self.screen_handle));
        let handle = unsafe { DisplayHandle::borrow_raw(raw) };
        Ok(handle)
    }
}

#[wasm_bindgen]
pub fn create_empty_document(
    display_handle: u64,
    screen_handle: u64,
    window_handle: u64,
) -> String {
    let _ = console_log::init_with_level(Level::Debug);
    panic::set_hook(Box::new(console_error_panic_hook::hook));

    let display_handle = display_handle as u32;
    let screen_handle = screen_handle as i32;
    let window_handle = window_handle as u32;

    info!("init");

    let viewport = Viewport::new(250, 250, 1.0, ColorScheme::Dark);
    let doc = BaseDocument::new(viewport.clone());

    let handle = WasmWindowHandle::from_window(display_handle, screen_handle, window_handle);
    let window = Arc::new(handle);

    let mut renderer = DemoPaintSource::new(window);

    info!("render");
    renderer.render(&doc, 1.0, 250, 250, Devtools::default());
    info!("resume");
    renderer.resume(&viewport);

    format!(
        "nodes: {:#?}\nnodes_to_id: {:#?}\nchanged: {:#?}",
        doc.nodes, doc.nodes_to_id, doc.changed
    )
}

type Color = OpaqueColor<Srgb>;

pub struct DemoPaintSource {
    state: DemoRendererState,
    start_time: std::time::Instant,
    tx: Sender<DemoMessage>,
    rx: Receiver<DemoMessage>,
    color: Color,
}

impl CustomPaintSource for DemoPaintSource {
    fn resume(&mut self, device: &Device, queue: &Queue) {
        // TODO: work out what to do about width/height
        let active_state = ActiveDemoRenderer::new(device, queue);
        self.state = DemoRendererState::Active(Box::new(active_state));
    }

    fn suspend(&mut self) {
        self.state = DemoRendererState::Suspended;
    }

    fn render(
        &mut self,
        ctx: CustomPaintCtx<'_>,
        width: u32,
        height: u32,
        _scale: f64,
    ) -> Option<TextureHandle> {
        self.process_messages();
        self.render(ctx, width, height)
    }
}
pub enum DemoMessage {
    // Color in RGB format
    SetColor(Color),
}

enum DemoRendererState {
    Active(Box<ActiveDemoRenderer>),
    Suspended,
}

#[derive(Clone)]
struct TextureAndHandle {
    texture: wgpu::Texture,
    handle: TextureHandle,
}

struct ActiveDemoRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    displayed_texture: Option<TextureAndHandle>,
    next_texture: Option<TextureAndHandle>,
}

impl DemoPaintSource {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self::with_channel(tx, rx)
    }

    pub fn with_channel(tx: Sender<DemoMessage>, rx: Receiver<DemoMessage>) -> Self {
        Self {
            state: DemoRendererState::Suspended,
            start_time: std::time::Instant::now(),
            tx,
            rx,
            color: Color::WHITE,
        }
    }

    pub fn sender(&self) -> Sender<DemoMessage> {
        self.tx.clone()
    }

    fn process_messages(&mut self) {
        loop {
            match self.rx.try_recv() {
                Err(_) => return,
                Ok(msg) => match msg {
                    DemoMessage::SetColor(color) => self.color = color,
                },
            }
        }
    }

    fn render(
        &mut self,
        ctx: CustomPaintCtx<'_>,
        width: u32,
        height: u32,
    ) -> Option<TextureHandle> {
        if width == 0 || height == 0 {
            return None;
        }
        let DemoRendererState::Active(state) = &mut self.state else {
            return None;
        };

        state.render(ctx, self.color.components, width, height, &self.start_time)
    }
}

impl ActiveDemoRenderer {
    pub(crate) fn new(device: &Device, queue: &Queue) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "shader.wgsl"
            ))),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[wgpu::PushConstantRange {
                stages: wgpu::ShaderStages::FRAGMENT,
                range: 0..16, // full size in bytes, aligned
            }],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::TextureFormat::Rgba8Unorm.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            device: device.clone(),
            queue: queue.clone(),
            pipeline,
            displayed_texture: None,
            next_texture: None,
        }
    }

    pub(crate) fn render(
        &mut self,
        mut ctx: CustomPaintCtx<'_>,
        light: [f32; 3],
        width: u32,
        height: u32,
        start_time: &Instant,
    ) -> Option<TextureHandle> {
        // If "next texture" size doesn't match specified size then unregister and drop texture
        if let Some(next) = &self.next_texture {
            if next.texture.width() != width || next.texture.height() != height {
                ctx.unregister_texture(next.handle);
                self.next_texture = None;
            }
        }

        // If there is no "next texture" then create one and register it.
        let texture_and_handle = match &self.next_texture {
            Some(next) => next,
            None => {
                let texture = create_texture(&self.device, width, height);
                let handle = ctx.register_texture(texture.clone());
                self.next_texture = Some(TextureAndHandle { texture, handle });
                self.next_texture.as_ref().unwrap()
            }
        };

        let next_texture = &texture_and_handle.texture;
        let next_texture_handle = texture_and_handle.handle;

        let elapsed: f32 = start_time.elapsed().as_millis() as f32 / 500.;
        let [light_red, light_green, light_blue] = light;
        let push_constants = PushConstants {
            light_color_and_time: [light_red, light_green, light_blue, elapsed],
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &next_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rpass.set_pipeline(&self.pipeline);
            rpass.set_push_constants(
                wgpu::ShaderStages::FRAGMENT, // Stage (your constants are for fragment shader)
                0,                            // Offset in bytes (start at 0)
                bytemuck::bytes_of(&push_constants),
            );
            rpass.draw(0..3, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));

        std::mem::swap(&mut self.next_texture, &mut self.displayed_texture);
        Some(next_texture_handle)
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct PushConstants {
    light_color_and_time: [f32; 4],
}

fn create_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}
