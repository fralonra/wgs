// crates/hanga/src/lib.rs

use wgpu::util::DeviceExt;
use std::sync::Arc;
use hanga_traits::Runtime; // Formerly wgs_runtime_base

// Inject our new modules
pub mod pipeline_2d; // The SpriteBatcher we designed
pub mod loader;      // The .gyo loader

use pipeline_2d::SpriteBatch;

pub struct HangaEngine {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    
    // 🌟 NEW: The 2D Pipeline
    sprite_batch: SpriteBatch,
    
    // 🌟 NEW: The Compute/SDF Pipeline (Placeholder)
    // raymarch_pipeline: RaymarchPipeline,
}

impl HangaEngine {
    pub async fn new(window: Arc<winit::window::Window>) -> Self {
        // ... Standard wgpu setup (Instance, Adapter, Device) ...
        // You can copy most of this from the old wgs_runtime_wgpu::lib.rs
        // BUT ensure you request features: wgpu::Features::POLYGON_MODE_LINE (for debug)
        
        // Initialize our SpriteBatch
        let sprite_batch = SpriteBatch::new(&device, 10_000); // Capacity for 10k sprites

        Self {
            device,
            queue,
            surface,
            config,
            sprite_batch,
        }
    }
    
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Hanga Render Encoder"),
        });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // 1. Draw Raymarched Background (Future)
            // self.raymarch_pipeline.draw(&mut rpass);

            // 2. Draw Sprites (The "Rain" layer)
            self.sprite_batch.draw(&mut rpass);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
