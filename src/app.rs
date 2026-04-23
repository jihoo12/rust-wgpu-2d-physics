use crate::state::WgpuState;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

pub struct App<'a> {
    pub window: Option<Arc<Window>>,
    pub state: Option<WgpuState<'a>>,
}

impl<'a> ApplicationHandler for App<'a> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );
        self.window = Some(window.clone());

        let state = pollster::block_on(WgpuState::new(window.clone()));
        self.state = Some(state);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(state) = &mut self.state {
                    state.resize(size);
                }
            }
            WindowEvent::RedrawRequested => {
                if let (Some(state), Some(window)) = (&mut self.state, &self.window) {
                    state.update();
                    match state.render() {
                        wgpu::CurrentSurfaceTexture::Success(frame) => {
                            state.draw(&frame);
                            frame.present();
                        }
                        wgpu::CurrentSurfaceTexture::Outdated => {
                            state.resize(window.inner_size());
                        }
                        wgpu::CurrentSurfaceTexture::Timeout => {
                            eprintln!("Surface timeout");
                        }
                        wgpu::CurrentSurfaceTexture::Lost => {
                            state.resize(window.inner_size());
                        }
                        // OutOfMemory 대신 와일드카드를 사용하여 기타 에러 처리
                        _ => {
                            eprintln!("Unhandled surface state");
                        }
                    }
                    window.request_redraw();
                }
            }
            _ => (),
        }
    }
}
