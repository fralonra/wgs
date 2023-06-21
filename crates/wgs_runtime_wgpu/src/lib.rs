//! A wgs runtime powered by [wgpu](https://wgpu.rs/).
//! 
//! Can be run on both native and Web.
//! 
//! ## Examples
//! 
//! ### Integrate with Winit
//! 
//! ```rust
//! use wgs_core::WgsData;
//! use wgs_runtime_wgpu::{Runtime, RuntimeExt};
//! use winit::{
//!     event::{Event, WindowEvent},
//!     event_loop::{ControlFlow, EventLoop},
//!     window::Window,
//! };
//!
//! fn main() {
//!     let event_loop = EventLoop::new();
//!
//!     let window = Window::new(&event_loop).unwrap();
//!
//!     let mut runtime =
//!         futures::executor::block_on(Runtime::new(&window, WgsData::default(), None)).unwrap();
//!
//!     let size = window.inner_size();
//!
//!     /// Needs to set the width and height of the runtime before rendering.
//!     runtime.resize(size.width as f32, size.height as f32);
//!
//!     event_loop.run(move |event, _, control_flow| {
//!         *control_flow = ControlFlow::Wait;
//!
//!         match event {
//!             Event::RedrawEventsCleared => window.request_redraw(),
//!             Event::WindowEvent {
//!                 event: WindowEvent::Resized(size),
//!                 ..
//!             } => {
//!                 runtime.resize(size.width as f32, size.height as f32);
//!
//!                 window.request_redraw();
//!             }
//!             Event::RedrawRequested(_) => {
//!                 /// Starts a new frame before doing the actual rendering.
//!                 runtime.frame_start().unwrap();
//! 
//!                 /// The actual rendering for wgs.
//!                 runtime.render().unwrap();
//! 
//!                 /// To render other stuff on the target surface besides the wgs content.
//!                 // runtime.render_with(|_device, _queue, _view| {
//!                 //     // Other rendering like ui etc.
//!                 // }).unwrap();
//!
//!                 /// Remember to finish the current working frame.
//!                 runtime.frame_finish().unwrap();
//!
//!                 window.request_redraw();
//!             }
//!             Event::WindowEvent {
//!                 event: WindowEvent::CloseRequested,
//!                 ..
//!             } => *control_flow = ControlFlow::Exit,
//!             _ => {}
//!         }
//!     });
//! }
//! ```
//! 
//! ### Integrate with Web
//! 
//! `wgs_runtime_wgpu` also compiles for Wasm32 and can be run on Web.
//! 
//! You can install it from [npm](https://www.npmjs.com/package/wgs-runtime-wgpu)
//! or use a high-level library [`wgs-player`](https://github.com/fralonra/wgs-player).
//! 

mod pausable_instant;
mod runtime;
mod uniform;
mod viewport;
#[cfg(target_arch = "wasm32")]
mod web;

pub use runtime::Runtime;
pub use viewport::Viewport;
pub use wgs_runtime_base::RuntimeExt;
