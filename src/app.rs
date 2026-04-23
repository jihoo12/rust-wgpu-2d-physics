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
            WindowEvent::MouseInput {
                state: element_state,
                button: winit::event::MouseButton::Left,
                ..
            } => {
                if let (Some(state), Some(window)) = (&mut self.state, &self.window) {
                    if element_state == winit::event::ElementState::Pressed {
                        state.is_dragging = true;
                    } else {
                        state.is_dragging = false;
                        // 던지는 힘은 마지막 마우스 이동 속도에 비례하도록 구현 가능
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                if let (Some(state), Some(window)) = (&mut self.state, &self.window) {
                    let size = window.inner_size();
                    // 화면 좌표를 -1.0 ~ 1.0 좌표계로 변환
                    let new_x = (position.x as f32 / size.width as f32) * 2.0 - 1.0;
                    let new_y = -((position.y as f32 / size.height as f32) * 2.0 - 1.0);

                    if state.is_dragging {
                        // 드래그 중이면 속도 계산 (현재 위치 - 이전 위치)
                        state.velocity[0] = (new_x - state.offset[0]) * 0.5;
                        state.velocity[1] = (new_y - state.offset[1]) * 0.5;

                        state.offset[0] = new_x;
                        state.offset[1] = new_y;
                    }
                }
            }
            _ => (),
        }
    }
}
