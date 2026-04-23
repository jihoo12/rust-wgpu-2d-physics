use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::window::Window;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceRaw {
    position: [f32; 2],
}

pub struct Entity {
    pub position: [f32; 2],
    pub velocity: [f32; 2],
}

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [0.0, 0.5, 0.0],
        color: [1.0, 0.0, 0.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.0],
        color: [0.0, 1.0, 0.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.0],
        color: [0.0, 0.0, 1.0],
    },
];

// 최대 인스턴스 개수 설정
const MAX_INSTANCES: usize = 1000;

pub struct WgpuState<'a> {
    pub surface: wgpu::Surface<'a>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub render_pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub num_vertices: u32,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,
    pub offset: [f32; 2],
    pub velocity: [f32; 2],
    pub is_dragging: bool,
    pub last_mouse_pos: [f32; 2],
    pub entities: Vec<Entity>,
    pub instance_buffer: wgpu::Buffer,
    pub dragged_entity_idx: Option<usize>,
}

impl<'a> WgpuState<'a> {
    pub async fn new(window: Arc<Window>) -> Self {
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: Default::default(),
                experimental_features: Default::default(),
            })
            .await
            .unwrap();

        let size = window.inner_size();
        let config = surface
            .get_default_config(&adapter, size.width, size.height)
            .unwrap();
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // 1. 유니폼 버퍼 및 바인드 그룹 설정
        let offset = [0.0f32, 0.0f32];
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&offset),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        // 2. 정점 버퍼 및 인스턴스 버퍼 생성
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let num_vertices = VERTICES.len() as u32;

        // 인스턴스 데이터를 담을 빈 버퍼를 미리 생성합니다.
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instance Buffer"),
            size: (std::mem::size_of::<InstanceRaw>() * MAX_INSTANCES) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // 3. 레이아웃 설정
        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        };

        let instance_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance, // 인스턴스 단위 데이터 업데이트
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 2,
                format: wgpu::VertexFormat::Float32x2,
            }],
        };

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[Some(&camera_bind_group_layout)],
                ..Default::default()
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[vertex_buffer_layout, instance_buffer_layout],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        // 4. 필드 초기화 (에러 해결)
        Self {
            surface,
            device,
            queue,
            config,
            render_pipeline,
            vertex_buffer,
            num_vertices,
            camera_buffer,
            camera_bind_group,
            offset,
            velocity: [0.0, 0.0],
            is_dragging: false,
            last_mouse_pos: [0.0, 0.0],
            entities: Vec::new(), // 빈 벡터로 시작
            instance_buffer,      // 생성한 버퍼 할당
            dragged_entity_idx: None,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn update(&mut self) {
        let mut instance_data = Vec::new();

        // 물리 상수 설정
        let gravity = -0.001; // 중력
        let friction = 0.98; // 공기 저항 (매 프레임 속도 유지 비율)
        let wall_bounce = -0.7; // 벽 충돌 반발 계수
        let floor_bounce = -0.6; // 바닥 충돌 반발 계수
        let ground_friction = 0.7; // 바닥 마찰력 (낮을수록 빨리 멈춤)

        for (i, entity) in self.entities.iter_mut().enumerate() {
            if Some(i) == self.dragged_entity_idx {
                entity.position = self.last_mouse_pos;
                entity.velocity = [0.0, 0.0];
            } else {
                // 1. 기본 물리 법칙 적용
                entity.velocity[1] += gravity; // 중력 적용
                entity.velocity[0] *= friction; // 공기 저항 적용
                entity.velocity[1] *= friction;

                entity.position[0] += entity.velocity[0];
                entity.position[1] += entity.velocity[1];

                // 2. 바닥 충돌 처리 (미끄러짐 방지 핵심)
                if entity.position[1] < -0.9 {
                    entity.position[1] = -0.9;
                    entity.velocity[1] *= floor_bounce; // 위로 튕김

                    // 바닥에 닿아있을 때 좌우 속도를 크게 줄임
                    entity.velocity[0] *= ground_friction;
                }

                // 3. 좌우 벽 충돌 처리
                if entity.position[0] < -1.0 {
                    entity.position[0] = -1.0;
                    entity.velocity[0] *= wall_bounce;
                } else if entity.position[0] > 1.0 {
                    entity.position[0] = 1.0;
                    entity.velocity[0] *= wall_bounce;
                }
            }

            instance_data.push(InstanceRaw {
                position: entity.position,
            });
        }

        // GPU 버퍼 업데이트 로직 (기존 유지)
        if !instance_data.is_empty() {
            self.queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(&instance_data),
            );
        }
    }
    pub fn try_grab(&mut self) {
        let grab_threshold = 0.5; // 클릭 인정 범위
        for (i, entity) in self.entities.iter().enumerate() {
            let dx = entity.position[0] - self.last_mouse_pos[0];
            let dy = entity.position[1] - self.last_mouse_pos[1];
            let distance = (dx * dx + dy * dy).sqrt();

            if distance < grab_threshold {
                self.dragged_entity_idx = Some(i);
                self.is_dragging = true;
                return;
            }
        }
        // 아무것도 못 잡았으면 새로 생성 (기존 클릭 생성 로직과 통합 가능)
        self.add_entity(self.last_mouse_pos[0], self.last_mouse_pos[1]);
    }

    pub fn render(&self) -> wgpu::CurrentSurfaceTexture {
        self.surface.get_current_texture()
    }

    pub fn draw(&self, frame: &wgpu::SurfaceTexture) {
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.05,
                            b: 0.05,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_bind_group(0, &self.camera_bind_group, &[]);
            rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            rpass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            rpass.draw(0..self.num_vertices, 0..self.entities.len() as u32);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }
    pub fn add_entity(&mut self, x: f32, y: f32) {
        // 최대 인스턴스 개수를 넘지 않도록 체크
        if self.entities.len() < MAX_INSTANCES {
            self.entities.push(Entity {
                position: [x, y],
                velocity: [
                    (rand::random::<f32>() - 0.5) * 0.05, // 좌우로 랜덤하게 튀게 설정
                    0.02,                                 // 약간 위로 솟구치며 생성
                ],
            });
        }
    }
}
