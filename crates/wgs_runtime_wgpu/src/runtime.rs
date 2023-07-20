use crate::{pausable_instant::PausableInstant, uniform::Uniform, viewport::Viewport};
use anyhow::{bail, Result};
use std::borrow::Cow;
use wgpu::util::DeviceExt;
use wgs_core::{concat_shader_frag, WgsData, VERT_DEFAULT};
use wgs_runtime_base::RuntimeExt;

#[cfg(not(target_arch = "wasm32"))]
const DATA_PER_PIXEL: u32 = 4;
#[cfg(not(target_arch = "wasm32"))]
const U8_SIZE: u32 = std::mem::size_of::<u8>() as u32;
const UNIFORM_GROUP_ID: u32 = 0;

/// The wgpu wgs runtime.
pub struct Runtime {
    #[cfg(not(target_arch = "wasm32"))]
    captured_callback: Option<(Viewport, Box<dyn FnOnce(&mut Self, u32, u32, Vec<u8>)>)>,
    device: wgpu::Device,
    format: wgpu::TextureFormat,
    height: f32,
    is_paused: bool,
    pipeline: wgpu::RenderPipeline,
    queue: wgpu::Queue,
    sampler: wgpu::Sampler,
    shader_vert: String,
    surface: wgpu::Surface,
    surface_configuration: wgpu::SurfaceConfiguration,
    surface_texture: Option<wgpu::SurfaceTexture>,
    texture_bind_groups: Vec<(wgpu::BindGroupLayout, wgpu::BindGroup)>,
    texture_view: Option<wgpu::TextureView>,
    time_instant: PausableInstant,
    uniform: Uniform,
    uniform_bind_group: wgpu::BindGroup,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
    uniform_buffer: wgpu::Buffer,
    viewport: Option<Viewport>,
    wgs: WgsData,
    width: f32,
}

impl RuntimeExt for Runtime {
    fn add_texture(&mut self, width: u32, height: u32, buffer: Vec<u8>) {
        self.texture_bind_groups.push(create_texture(
            &self.device,
            &self.queue,
            &self.sampler,
            width,
            height,
            &buffer,
        ));

        self.wgs.add_texture(width, height, buffer);
    }

    fn change_texture(&mut self, index: usize, width: u32, height: u32, buffer: Vec<u8>) {
        self.texture_bind_groups[index] = create_texture(
            &self.device,
            &self.queue,
            &self.sampler,
            width,
            height,
            &buffer,
        );

        self.wgs.change_texture(index, width, height, buffer);
    }

    fn compile(&mut self) -> Result<()> {
        self.pipeline = prepare_wgs_pipeline(
            &self.wgs,
            &self.device,
            self.format,
            &self.shader_vert,
            &self.texture_bind_groups,
            &self.uniform_bind_group_layout,
        )?;

        self.restart();

        Ok(())
    }

    fn load(&mut self, wgs: wgs_core::WgsData) -> Result<()> {
        let (texture_bind_groups, pipeline) = prepare_wgs(
            &wgs,
            &self.device,
            &self.queue,
            &self.sampler,
            self.format,
            &self.shader_vert,
            &self.uniform_bind_group_layout,
        )?;

        self.texture_bind_groups = texture_bind_groups;
        self.pipeline = pipeline;
        self.wgs = wgs;

        self.restart();

        Ok(())
    }

    fn pause(&mut self) {
        if self.is_paused {
            return;
        }

        self.is_paused = true;

        self.time_instant.pause();
    }

    fn remove_texture(&mut self, index: usize) {
        self.texture_bind_groups.remove(index);

        self.wgs.remove_texture(index);
    }

    fn render(&mut self) -> Result<()> {
        if self.texture_view.is_none() {
            bail!("No actived wgpu::TextureView found.")
        }

        let view = self.texture_view.as_ref().unwrap();

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        if let Some(viewport) = &self.viewport {
            self.uniform.resolution = [viewport.width, viewport.height];
        } else {
            self.uniform.resolution = [self.width, self.height]
        }

        self.uniform.time = self.time_instant.elapsed().as_secs_f32();

        self.queue
            .write_buffer(&self.uniform_buffer, 0, self.uniform.as_bytes());

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            if let Some(viewport) = &self.viewport {
                render_pass.set_viewport(
                    viewport.x,
                    viewport.y,
                    viewport.width,
                    viewport.height,
                    viewport.min_depth,
                    viewport.max_depth,
                );
            }

            render_pass.set_pipeline(&self.pipeline);

            render_pass.set_bind_group(UNIFORM_GROUP_ID, &self.uniform_bind_group, &[]);

            let mut index = 1;
            for (_, bind_group) in &self.texture_bind_groups {
                render_pass.set_bind_group(index, bind_group, &[]);
                index += 1;
            }

            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));

        Ok(())
    }

    fn resize(&mut self, width: f32, height: f32) {
        self.surface_configuration.width = width as u32;
        self.surface_configuration.height = height as u32;

        self.surface
            .configure(&self.device, &self.surface_configuration);

        self.width = width;
        self.height = height;
    }

    fn restart(&mut self) {
        self.is_paused = false;

        self.time_instant = PausableInstant::now();

        let resolution = self.uniform.resolution;

        self.uniform = Uniform::default();
        self.uniform.resolution = resolution;
    }

    fn resume(&mut self) {
        if !self.is_paused {
            return;
        }

        self.is_paused = false;

        self.time_instant.resume();
    }

    fn set_wgs_frag(&mut self, shader_frag: &str) {
        self.wgs.set_frag(shader_frag)
    }

    fn set_wgs_name(&mut self, name: &str) {
        self.wgs.set_name(name);
    }

    fn update_cursor(&mut self, cursor: [f32; 2]) {
        if self.is_paused {
            return;
        }

        self.uniform.cursor = [cursor[0], self.uniform.resolution[1] - cursor[1]];
    }

    fn update_mouse_press(&mut self) {
        if self.is_paused {
            return;
        }

        self.uniform.mouse_down = 1;
        self.uniform.mouse_press = self.uniform.cursor;
    }

    fn update_mouse_release(&mut self) {
        if self.is_paused {
            return;
        }

        self.uniform.mouse_down = 0;
        self.uniform.mouse_release = self.uniform.cursor;
    }

    fn wgs(&self) -> &WgsData {
        &self.wgs
    }
}

impl Runtime {
    #[cfg(target_arch = "wasm32")]
    pub async fn new(
        canvas: web_sys::HtmlCanvasElement,
        wgs: WgsData,
        viewport: Option<Viewport>,
    ) -> Result<Self> {
        let instance = init_instance();

        let surface = init_surface(&instance, canvas)?;

        Self::with_instance_and_surface(wgs, viewport, instance, surface).await
    }

    /// Creates a new runtime instance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wgs_core::WgsData;
    /// use wgs_runtime_wgpu::{Runtime, RuntimeExt};
    /// use winit::{event::WindowEvent, event_loop::EventLoop, window::Window};
    ///
    /// fn main() {
    ///     let event_loop = EventLoop::new();
    ///
    ///     let window = Window::new(&event_loop).unwrap();
    ///
    ///     let mut runtime =
    ///         futures::executor::block_on(Runtime::new(&window, WgsData::default(), None)).unwrap();
    ///
    ///     let size = window.inner_size();
    ///
    ///     runtime.resize(size.width as f32, size.height as f32);
    ///
    ///     // Dealing with events.
    /// }
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn new<W>(w: &W, wgs: WgsData, viewport: Option<Viewport>) -> Result<Self>
    where
        W: raw_window_handle::HasRawWindowHandle + raw_window_handle::HasRawDisplayHandle,
    {
        let instance = init_instance();

        let surface = init_surface(&instance, w)?;

        Self::with_instance_and_surface(wgs, viewport, instance, surface).await
    }

    /// Creates a new runtime with given [`wgpu::Instance`] and [`wgpu::Surface`].
    pub async fn with_instance_and_surface(
        wgs: WgsData,
        viewport: Option<Viewport>,
        instance: wgpu::Instance,
        surface: wgpu::Surface,
    ) -> Result<Self> {
        let (format, surface_configuration, device, queue) =
            init_adapter(&instance, &surface).await?;

        #[cfg(not(target_arch = "wasm32"))]
        {
            device.push_error_scope(wgpu::ErrorFilter::Validation);
        }

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

        let (uniform, uniform_buffer, uniform_bind_group_layout, uniform_bind_group) =
            setup_uniform(&device);

        let shader_vert = VERT_DEFAULT.to_owned();

        let (texture_bind_groups, pipeline) = prepare_wgs(
            &wgs,
            &device,
            &queue,
            &sampler,
            format,
            &shader_vert,
            &uniform_bind_group_layout,
        )?;

        Ok(Self {
            #[cfg(not(target_arch = "wasm32"))]
            captured_callback: None,
            device,
            format,
            height: 0.0,
            is_paused: false,
            pipeline,
            queue,
            sampler,
            shader_vert,
            surface,
            surface_configuration,
            surface_texture: None,
            texture_bind_groups,
            texture_view: None,
            time_instant: PausableInstant::now(),
            uniform,
            uniform_bind_group,
            uniform_bind_group_layout,
            uniform_buffer,
            viewport,
            wgs,
            width: 0.0,
        })
    }

    /// Returns the [`wgpu::Device`].
    pub fn device_ref(&self) -> &wgpu::Device {
        &self.device
    }

    /// Returns the [`wgpu::TextureFormat`] used in the program.
    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// Finishes the current working frame and presents it.
    ///
    /// Needs to be called after [`Self::frame_start`] and at the end of each frame.
    ///
    /// # Errors
    ///
    /// - Will return an error if [`Self::frame_start`] haven't been called first.
    pub fn frame_finish(&mut self) -> Result<()> {
        if self.surface_texture.is_none() {
            bail!("No actived wgpu::SurfaceTexture found.")
        }

        if let Some(surface_texture) = self.surface_texture.take() {
            #[cfg(not(target_arch = "wasm32"))]
            if let Some((viewport, callback)) = self.captured_callback.take() {
                let texture = &surface_texture.texture;

                let size = texture.size();

                let (width, height, buffer) = futures::executor::block_on(self.capture_image(
                    &viewport,
                    size.width,
                    size.height,
                    texture.as_image_copy(),
                ))?;

                callback(self, width, height, buffer);
            }

            surface_texture.present();
        }

        Ok(())
    }

    /// Starts a new frame.
    ///
    /// Needs to be called before [`Self::frame_finish`] and at the begining of each frame.
    ///
    /// # Errors
    ///
    /// - Will return an error if [`Self::frame_finish()`] haven't been called at the end of the last frame.
    pub fn frame_start(&mut self) -> Result<()> {
        if self.surface_texture.is_some() {
            bail!("Non-finished wgpu::SurfaceTexture found.")
        }

        let surface_texture = self.surface.get_current_texture()?;

        self.surface_texture = Some(surface_texture);

        if let Some(surface_texture) = &self.surface_texture {
            self.texture_view = Some(surface_texture.texture.create_view(
                &wgpu::TextureViewDescriptor {
                    format: Some(self.format),
                    ..wgpu::TextureViewDescriptor::default()
                },
            ));
        }

        Ok(())
    }

    /// Returns whether the wgs rendering is currently paused.
    pub fn is_paused(&self) -> bool {
        self.is_paused
    }

    /// The maximum number of textures that can be used.
    ///
    /// Depends on the [`wgpu::Limits::max_bind_groups`] of [`wgpu::Device`].
    pub fn max_texture_count(&self) -> u32 {
        self.device.limits().max_bind_groups
    }

    /// Pops an error scope from [`wgpu::Device`]. [Read more](wgpu::Device::pop_error_scope).
    pub fn pop_error_scope(&mut self) -> Option<wgpu::Error> {
        let error_scope = futures::executor::block_on(self.device.pop_error_scope());

        self.device.push_error_scope(wgpu::ErrorFilter::Validation);

        error_scope
    }

    /// Renders other stuff on the target surface besides the wgs content.
    ///
    /// # Examples
    ///
    /// ```rust
    /// runtime.render_with(|_device, _queue, _view| {
    ///    // Other rendering like ui etc.
    /// }).unwrap();
    /// ```
    pub fn render_with<F>(&mut self, mut f: F) -> Result<()>
    where
        F: FnMut(&wgpu::Device, &wgpu::Queue, &wgpu::TextureView) -> Result<()>,
    {
        if self.texture_view.is_none() {
            bail!("No actived wgpu::TextureView found.")
        }

        let view = self.texture_view.as_ref().unwrap();

        f(&self.device, &self.queue, view)?;

        Ok(())
    }

    /// Request a capture on the given [`Viewport`] asynchronously.
    ///
    /// # Examples
    ///
    /// ```rust
    /// runtime.request_capture_image(
    ///    &viewport,
    ///    |runtime, width, height, buffer| {
    ///        /// Doing something with the buffer.
    ///    },
    ///);
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    pub fn request_capture_image<F>(&mut self, viewport: &Viewport, f: F)
    where
        F: FnOnce(&mut Self, u32, u32, Vec<u8>) + 'static,
    {
        self.captured_callback = Some((viewport.clone(), Box::new(f)));
    }

    /// Sets the [`Viewport`] for render wgs.
    pub fn set_viewport(&mut self, viewport: Option<Viewport>) {
        self.viewport = viewport;
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn capture_image(
        &mut self,
        viewport: &Viewport,
        raw_width: u32,
        raw_height: u32,
        image_texture: wgpu::ImageCopyTexture<'_>,
    ) -> Result<(u32, u32, Vec<u8>)> {
        let align_width = align_up(
            raw_width * DATA_PER_PIXEL * U8_SIZE,
            wgpu::COPY_BYTES_PER_ROW_ALIGNMENT,
        ) / U8_SIZE;

        let texture_size = wgpu::Extent3d {
            width: raw_width,
            height: raw_height,
            depth_or_array_layers: 1,
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Capture Encoder"),
            });

        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Buffer"),
            size: (align_width * raw_height) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let image_buffer = wgpu::ImageCopyBuffer {
            buffer: &output_buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(align_width),
                rows_per_image: None,
            },
        };

        encoder.copy_texture_to_buffer(image_texture, image_buffer, texture_size);

        self.queue.submit(Some(encoder.finish()));

        let buffer = view_into_buffer(
            &self.device,
            viewport,
            raw_width,
            raw_height,
            &output_buffer,
        )
        .await?;

        let width = viewport.width as u32;
        let height = viewport.height as u32;

        Ok((width, height, buffer))
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn align_up(num: u32, align: u32) -> u32 {
    (num + align - 1) & !(align - 1)
}

fn build_pipeline(
    shader_frag: &str,
    shader_vert: &str,
    bind_group_layouts: &[&wgpu::BindGroupLayout],
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
) -> Result<wgpu::RenderPipeline> {
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Pipeline Layout"),
        bind_group_layouts,
        push_constant_ranges: &[],
    });

    let fs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::from(shader_frag)),
    });

    let vs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::from(shader_vert)),
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &vs_module,
            entry_point: "main",
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &fs_module,
            entry_point: "main",
            targets: &[Some(format.into())],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    Ok(pipeline)
}

fn create_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    sampler: &wgpu::Sampler,
    width: u32,
    height: u32,
    buffer: &[u8],
) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
    let texture_size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        size: texture_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        label: Some("Diffuse Texture"),
        view_formats: &[wgpu::TextureFormat::Rgba8UnormSrgb],
    });

    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        buffer,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * width),
            rows_per_image: Some(height),
        },
        texture_size,
    );

    let texture_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("Texture Bind Group Layout"),
        });

    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &texture_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
        label: Some("Bind Group"),
    });

    (texture_bind_group_layout, texture_bind_group)
}

async fn init_adapter(
    instance: &wgpu::Instance,
    surface: &wgpu::Surface,
) -> Result<(
    wgpu::TextureFormat,
    wgpu::SurfaceConfiguration,
    wgpu::Device,
    wgpu::Queue,
)> {
    if let Some(adapter) = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(surface),
            ..wgpu::RequestAdapterOptions::default()
        })
        .await
    {
        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let format = swapchain_capabilities.formats[0];

        let adapter_features = adapter.features();

        let surface_configuration = init_surface_configuration(&surface, &adapter, format);

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Device Descriptor"),
                    features: adapter_features & wgpu::Features::default(),
                    #[cfg(target_arch = "wasm32")]
                    limits: wgpu::Limits::downlevel_webgl2_defaults(),
                    #[cfg(not(target_arch = "wasm32"))]
                    limits: wgpu::Limits::downlevel_defaults(),
                },
                None,
            )
            .await?;

        Ok((format, surface_configuration, device, queue))
    } else {
        bail!("No adapters are found that suffice all the 'hard' options.")
    }
}

fn init_instance() -> wgpu::Instance {
    let backends = wgpu::util::backend_bits_from_env().unwrap_or_else(wgpu::Backends::all);

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends,
        dx12_shader_compiler: wgpu::Dx12Compiler::default(),
    });

    instance
}

#[cfg(target_arch = "wasm32")]
fn init_surface(
    instance: &wgpu::Instance,
    canvas: web_sys::HtmlCanvasElement,
) -> Result<wgpu::Surface> {
    let surface = instance.create_surface_from_canvas(canvas)?;

    Ok(surface)
}

#[cfg(not(target_arch = "wasm32"))]
fn init_surface<W>(instance: &wgpu::Instance, w: &W) -> Result<wgpu::Surface>
where
    W: raw_window_handle::HasRawWindowHandle + raw_window_handle::HasRawDisplayHandle,
{
    let surface = unsafe { instance.create_surface(w)? };

    Ok(surface)
}

fn init_surface_configuration(
    surface: &wgpu::Surface,
    adapter: &wgpu::Adapter,
    format: wgpu::TextureFormat,
) -> wgpu::SurfaceConfiguration {
    let config = if let Some(mut config) = surface.get_default_config(adapter, 0, 0) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            config.usage |= wgpu::TextureUsages::COPY_SRC;
        }

        config.view_formats.push(format);

        config
    } else {
        wgpu::SurfaceConfiguration {
            #[cfg(not(target_arch = "wasm32"))]
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            #[cfg(target_arch = "wasm32")]
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: 0,
            height: 0,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![format],
        }
    };

    config
}

fn prepare_wgs(
    wgs: &WgsData,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    sampler: &wgpu::Sampler,
    format: wgpu::TextureFormat,
    shader_vert: &str,
    uniform_bind_group_layout: &wgpu::BindGroupLayout,
) -> Result<(
    Vec<(wgpu::BindGroupLayout, wgpu::BindGroup)>,
    wgpu::RenderPipeline,
)> {
    let texture_bind_groups = prepare_wgs_textures(wgs, device, queue, sampler);

    let pipeline = prepare_wgs_pipeline(
        wgs,
        device,
        format,
        shader_vert,
        &texture_bind_groups,
        uniform_bind_group_layout,
    )?;

    Ok((texture_bind_groups, pipeline))
}

fn prepare_wgs_pipeline(
    wgs: &WgsData,
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    shader_vert: &str,
    texture_bind_groups: &Vec<(wgpu::BindGroupLayout, wgpu::BindGroup)>,
    uniform_bind_group_layout: &wgpu::BindGroupLayout,
) -> Result<wgpu::RenderPipeline> {
    let mut bind_group_layouts = vec![uniform_bind_group_layout];
    for (layout, _) in texture_bind_groups {
        bind_group_layouts.push(layout);
    }

    let shader_frag = concat_shader_frag(&wgs.frag(), wgs.textures_ref().len());

    let pipeline = build_pipeline(
        &shader_frag,
        &shader_vert,
        &bind_group_layouts,
        device,
        format,
    )?;

    Ok(pipeline)
}

fn prepare_wgs_textures(
    wgs: &WgsData,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    sampler: &wgpu::Sampler,
) -> Vec<(wgpu::BindGroupLayout, wgpu::BindGroup)> {
    let textures = wgs
        .textures_ref()
        .iter()
        .map(|texture| (texture.width, texture.height, &texture.data))
        .collect::<Vec<(u32, u32, &Vec<u8>)>>();

    textures
        .iter()
        .map(|(width, height, data)| create_texture(device, queue, sampler, *width, *height, &data))
        .collect()
}

fn setup_uniform(
    device: &wgpu::Device,
) -> (
    Uniform,
    wgpu::Buffer,
    wgpu::BindGroupLayout,
    wgpu::BindGroup,
) {
    let uniform = Uniform::default();

    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Uniform Buffer"),
        contents: uniform.as_bytes(),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let uniform_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

    let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Uniform Bind Group"),
        layout: &uniform_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });

    (
        uniform,
        uniform_buffer,
        uniform_bind_group_layout,
        uniform_bind_group,
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn trim_image_buffer(viewport: &Viewport, align_width: usize, buffer: &[u8]) -> Vec<u8> {
    let x = viewport.x as usize;
    let width = viewport.width as usize;
    let height = viewport.height as usize;

    let mut output = Vec::with_capacity(width * height);

    let pad_before_per_row = x * DATA_PER_PIXEL as usize * U8_SIZE as usize;
    let len_per_row = width * DATA_PER_PIXEL as usize * U8_SIZE as usize;

    for chunk in buffer.chunks(align_width) {
        for chunk in chunk[pad_before_per_row..pad_before_per_row + len_per_row].chunks(4) {
            // Convert BGRA8 to RGBA8
            output.push(chunk[2]);
            output.push(chunk[1]);
            output.push(chunk[0]);
            output.push(chunk[3]);
        }
    }

    output
}

#[cfg(not(target_arch = "wasm32"))]
async fn view_into_buffer(
    device: &wgpu::Device,
    viewport: &Viewport,
    raw_width: u32,
    raw_height: u32,
    raw_buffer: &wgpu::Buffer,
) -> Result<Vec<u8>> {
    let slice = raw_buffer.slice(..);

    let (sender, receiver) = futures::channel::oneshot::channel();

    slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());

    device.poll(wgpu::Maintain::Wait);

    if let std::result::Result::Ok(_) = receiver.await {
        let buffer_view = slice.get_mapped_range();

        let buffer = trim_image_buffer(
            viewport,
            buffer_view.len() / raw_height as usize,
            &buffer_view,
        );

        drop(buffer_view);
        raw_buffer.unmap();

        Ok(buffer)
    } else {
        bail!("Failed to map the buffer.")
    }
}
