use crate::{pausable_instant::PausableInstant, uniform::Uniform, viewport::Viewport};
use anyhow::{bail, Result};
use std::borrow::Cow;
use wgpu::util::DeviceExt;
use wgs_core::{concat_shader_frag, WgsData, VERT_DEFAULT};
use wgs_runtime_base::RuntimeExt;

pub const UNIFORM_GROUP_ID: u32 = 0;

const DATA_PER_PIXEL: u32 = 4;
const U8_SIZE: u32 = std::mem::size_of::<u8>() as u32;

pub struct Runtime {
    captured_callback: Option<(Viewport, Box<dyn FnOnce(&mut Self, u32, u32, Vec<u8>)>)>,
    device: wgpu::Device,
    format: wgpu::TextureFormat,
    is_paused: bool,
    pipeline: wgpu::RenderPipeline,
    queue: wgpu::Queue,
    sampler: wgpu::Sampler,
    shader_vert: String,
    surface: wgpu::Surface,
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
}

impl RuntimeExt for Runtime {
    fn add_texture(&mut self, width: u32, height: u32, buffer: Vec<u8>) {
        self.texture_bind_groups.push(create_texture(
            &self.device,
            &self.queue,
            &self.sampler,
            self.format,
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
            self.format,
            width,
            height,
            &buffer,
        );

        self.wgs.change_texture(index, width, height, buffer);
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
        self.surface.configure(
            &self.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                format: self.format,
                width: width as u32,
                height: height as u32,
                present_mode: wgpu::PresentMode::AutoVsync,
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                view_formats: vec![self.format],
            },
        );

        self.uniform.resolution = [width / 2.0, height];
    }

    fn restart(&mut self) {
        self.is_paused = false;

        self.time_instant = PausableInstant::now();

        let resolution = self.uniform.resolution;

        self.uniform = Uniform::default();
        self.uniform.resolution = resolution;
    }

    fn resume(&mut self) {
        self.is_paused = false;

        self.time_instant.resume();
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

    fn update_frag(&mut self, shader_frag: &str) -> Result<()> {
        let mut bind_group_layouts = vec![&self.uniform_bind_group_layout];
        for (layout, _) in &self.texture_bind_groups {
            bind_group_layouts.push(layout);
        }

        self.pipeline = build_pipeline(
            shader_frag,
            &self.shader_vert,
            &bind_group_layouts,
            &self.device,
            self.format,
        )?;

        Ok(())
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
    pub fn new<W>(w: &W, wgs: WgsData, viewport: Option<Viewport>) -> Result<Self>
    where
        W: raw_window_handle::HasRawWindowHandle + raw_window_handle::HasRawDisplayHandle,
    {
        let (surface, format, device, queue) = futures::executor::block_on(init_device(w))?;

        device.push_error_scope(wgpu::ErrorFilter::Validation);

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
            captured_callback: None,
            device,
            format,
            is_paused: false,
            pipeline,
            queue,
            sampler,
            shader_vert,
            surface,
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
        })
    }

    pub fn device_ref(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    pub fn frame_finish(&mut self) -> Result<()> {
        if self.surface_texture.is_none() {
            bail!("No actived wgpu::SurfaceTexture found.")
        }

        if let Some(surface_texture) = self.surface_texture.take() {
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

    pub fn frame_start(&mut self) -> Result<()> {
        if self.surface_texture.is_some() {
            bail!("Non-finished wgpu::SurfaceTexture found.")
        }

        let surface_texture = self.surface.get_current_texture()?;

        self.surface_texture = Some(surface_texture);

        if let Some(surface_texture) = &self.surface_texture {
            self.texture_view = Some(
                surface_texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default()),
            );
        }

        Ok(())
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused
    }

    pub fn max_texture_count(&self) -> u32 {
        self.device.limits().max_bind_groups
    }

    pub fn pop_error_scope(&mut self) -> Option<wgpu::Error> {
        let error_scope = futures::executor::block_on(self.device.pop_error_scope());

        self.device.push_error_scope(wgpu::ErrorFilter::Validation);

        error_scope
    }

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

    pub fn request_capture_image<F>(&mut self, viewport: &Viewport, f: F)
    where
        F: FnOnce(&mut Self, u32, u32, Vec<u8>) + 'static,
    {
        self.captured_callback = Some((viewport.clone(), Box::new(f)));
    }

    pub fn set_viewport(&mut self, viewport: Option<Viewport>) {
        self.viewport = viewport;
    }

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
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            ..Default::default()
        },
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
    format: wgpu::TextureFormat,
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
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        label: Some("Diffuse Texture"),
        view_formats: &[format],
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

async fn init_device<W>(
    w: &W,
) -> Result<(
    wgpu::Surface,
    wgpu::TextureFormat,
    wgpu::Device,
    wgpu::Queue,
)>
where
    W: raw_window_handle::HasRawWindowHandle + raw_window_handle::HasRawDisplayHandle,
{
    let backends = wgpu::util::backend_bits_from_env().unwrap_or_else(wgpu::Backends::all);
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends,
        dx12_shader_compiler: wgpu::Dx12Compiler::default(),
    });

    let surface = unsafe { instance.create_surface(w)? };

    if let Some(adapter) = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
    {
        let format = surface
            .get_capabilities(&adapter)
            .formats
            .first()
            .copied()
            .expect("Get preferred format.");

        let adapter_features = adapter.features();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Device Descriptor"),
                    features: adapter_features & wgpu::Features::default(),
                    limits: wgpu::Limits::downlevel_defaults(),
                },
                None,
            )
            .await?;

        Ok((surface, format, device, queue))
    } else {
        bail!("No adapters are found that suffice all the 'hard' options.")
    }
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
    let textures = wgs
        .textures_ref()
        .iter()
        .map(|texture| (texture.width, texture.height, &texture.data))
        .collect::<Vec<(u32, u32, &Vec<u8>)>>();

    let texture_bind_groups = textures
        .iter()
        .map(|(width, height, data)| {
            create_texture(device, queue, sampler, format, *width, *height, &data)
        })
        .collect::<Vec<(wgpu::BindGroupLayout, wgpu::BindGroup)>>();

    let mut bind_group_layouts = vec![uniform_bind_group_layout];
    for (layout, _) in &texture_bind_groups {
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

    Ok((texture_bind_groups, pipeline))
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

fn trim_image_buffer(viewport: &Viewport, align_width: usize, buffer: &[u8]) -> Vec<u8> {
    let x = viewport.x as usize;
    let width = viewport.width as usize;
    let height = viewport.height as usize;

    let mut output = Vec::with_capacity(width * height);

    let pad_before_per_row = x * DATA_PER_PIXEL as usize * U8_SIZE as usize;
    let len_per_row = width * DATA_PER_PIXEL as usize * U8_SIZE as usize;

    for chunk in buffer.chunks(align_width) {
        output.append(&mut chunk[pad_before_per_row..pad_before_per_row + len_per_row].to_owned());
    }

    output
}

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
