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
                if let Some(state) = &mut self.state {
                    if element_state == winit::event::ElementState::Pressed {
                        state.try_grab(); // 잡기 시도 (없으면 생성)
                    } else {
                        state.is_dragging = false;
                        state.dragged_entity_idx = None; // 놓기
                    }
                }
            }

            // app.rs의 CursorMoved 내부
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(state) = &mut self.state {
                    let size = self.window.as_ref().unwrap().inner_size();
                    let new_x = (position.x as f32 / size.width as f32) * 2.0 - 1.0;
                    let new_y = -((position.y as f32 / size.height as f32) * 2.0 - 1.0);

                    // [추가] 드래그 중일 때 놓는 순간의 속도를 위해 미리 계산
                    if state.is_dragging {
                        if let Some(idx) = state.dragged_entity_idx {
                            state.entities[idx].velocity[0] =
                                (new_x - state.last_mouse_pos[0]) * 0.5;
                            state.entities[idx].velocity[1] =
                                (new_y - state.last_mouse_pos[1]) * 0.5;
                        }
                    }

                    state.last_mouse_pos = [new_x, new_y];
                }
            }
            _ => (),
        }
    }
}
