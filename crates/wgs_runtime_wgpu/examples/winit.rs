use winit::event_loop::{ControlFlow, EventLoop};

fn main() -> Result<(), impl std::error::Error> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = app::App::default();
    event_loop.run_app(&mut app)
}

mod app {
    use std::sync::Arc;
    use wgs_core::WgsData;
    use wgs_runtime_wgpu::{Runtime, RuntimeExt};
    use winit::{
        application::ApplicationHandler,
        event::WindowEvent,
        event_loop::ActiveEventLoop,
        window::{Window, WindowId},
    };

    #[derive(Default)]
    pub struct App<'a> {
        runtime: Option<Runtime<'a>>,
        window: Option<Arc<Window>>,
    }

    impl<'a> ApplicationHandler for App<'a> {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            let window = event_loop
                .create_window(Window::default_attributes())
                .unwrap();
            let window = Arc::new(window);

            let mut runtime =
                futures::executor::block_on(Runtime::new(window.clone(), WgsData::default(), None))
                    .unwrap();
            let size = window.inner_size();
            runtime.resize(size.width as f32, size.height as f32);

            self.runtime = Some(runtime);
            self.window = Some(window)
        }

        fn window_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            _id: WindowId,
            event: WindowEvent,
        ) {
            match event {
                WindowEvent::CloseRequested => {
                    event_loop.exit();
                }
                WindowEvent::RedrawRequested => {
                    if let Some(runtime) = &mut self.runtime {
                        runtime.frame_start().unwrap();

                        runtime.render().unwrap();

                        runtime.frame_finish().unwrap();
                    }

                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
                WindowEvent::Resized(size) => {
                    if let Some(runtime) = &mut self.runtime {
                        runtime.resize(size.width as f32, size.height as f32);
                    }
                }
                _ => (),
            }
        }
    }
}
