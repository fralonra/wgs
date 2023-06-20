use wgs_core::WgsData;
use wgs_runtime_wgpu::{Runtime, RuntimeExt};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

fn main() {
    let event_loop = EventLoop::new();

    let window = Window::new(&event_loop).unwrap();

    let mut runtime =
        futures::executor::block_on(Runtime::new(&window, WgsData::default(), None)).unwrap();

    let size = window.inner_size();

    runtime.resize(size.width as f32, size.height as f32);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::RedrawEventsCleared => window.request_redraw(),
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                runtime.resize(size.width as f32, size.height as f32);

                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                runtime.frame_start().unwrap();

                runtime.render().unwrap();

                runtime.frame_finish().unwrap();

                window.request_redraw();
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => {}
        }
    });
}
