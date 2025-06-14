use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

mod load_pcd;
use load_pcd::load_pcd;

mod adapter_info;

use glam::{Mat4, Vec3};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    uv: [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    transform: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Instance {
    offset: [f32; 3],
    scale: f32,
}

pub struct DepthTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub format: wgpu::TextureFormat,
}

impl DepthTexture {
    pub fn create_depth_texture(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        label: &str,
    ) -> Self {
        let format = wgpu::TextureFormat::Depth32Float;
        let size = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };

        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let texture = device.create_texture(&desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            texture,
            view,
            format,
        }
    }
}

impl Instance {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Instance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    // offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-0.25, -0.25, 0.25],
        uv: [0.0, 0.0, 0.0],
    }, //Left down: 0
    Vertex {
        position: [0.25, -0.25, 0.25],
        uv: [1.0, 0.0, 0.0],
    }, // Right down: 1
    Vertex {
        position: [0.25, 0.25, 0.25],
        uv: [1.0, 1.0, 0.0],
    }, // Right up: 2
    Vertex {
        position: [-0.25, 0.25, 0.25],
        uv: [0.0, 1.0, 0.0],
    }, // Left up: 3
    // Back
    Vertex {
        position: [-0.25, -0.25, -0.25],
        uv: [1.0, 0.0, 0.0],
    }, //Center above: 4
    Vertex {
        position: [0.25, -0.25, -0.25],
        uv: [0.0, 0.0, 0.0],
    }, // Right: 5
    Vertex {
        position: [0.25, 0.25, -0.25],
        uv: [0.0, 1.0, 0.0],
    }, // Down: 6
    Vertex {
        position: [-0.25, 0.25, -0.25],
        uv: [1.0, 1.0, 0.0],
    }, // Left: 7
];

const INDICES: &[u16] = &[
    // 前面
    0, 1, 2, 2, 3, 0, // 右面
    1, 5, 6, 6, 2, 1, // 背面
    5, 4, 7, 7, 6, 5, // 左面
    4, 0, 3, 3, 7, 4, // 上面
    3, 2, 6, 6, 7, 3, // 下面
    4, 5, 1, 1, 0, 4,
];

const CHUNK_SIZE: u64 = 256 * 1024 * 1024; // 256MB
const FLOAT_PER_CHUNK: usize = (CHUNK_SIZE / 4) as usize;
const TOTAL_FLOATS: usize = FLOAT_PER_CHUNK * 4;

struct State<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: &'a Window,
    render_pipeline: wgpu::RenderPipeline,
    instances: Vec<Instance>,
    // instance_buffers: Vec<wgpu::Buffer>,
    instance_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    num_indices: u32,
    rotation_angle: f32,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    view: Mat4,
    proj: Mat4,
    camera: cgmath::Vector3<f32>,
    focus_point: cgmath::Vector3<f32>,
}

impl<'a> State<'a> {
    async fn new(window: &'a Window) -> State<'a> {
        // let pcd_paths =
        //     match load_pcd_paths("/Users/kenji/workspace/Rust/wgpu-pcd/data/", "Laser_map") {
        //         Ok(paths) => paths,
        //         Err(e) => {
        //             eprintln!("Error loading paths: {}", e);
        //             panic!();
        //         }
        //     };
        let pcd_path: &str = "/Users/kenji/workspace/Rust/rerun-sample/data/combined_data_none_color/combined_105.pcd";
        // let pcd_path: &str = "/Users/kenji/workspace/Rust/rerun-sample/data/Laser_map/Laser_map_130.pcd";
        let pcd_paths: Vec<String> = vec![pcd_path.to_string()];

        let mut laser_map_points: Vec<(f32, f32, f32)> = Vec::new();

        for path in pcd_paths.iter() {
            let points_vec = match load_pcd(path) {
                Ok(points) => points,
                Err(e) => {
                    eprintln!("Error loading PCD file: {}", e);
                    panic!();
                }
            };

            laser_map_points.extend(points_vec.iter().map(|pt| (pt.x, pt.y, pt.z)));
        }
        println!("Num points: {:?}", laser_map_points.len());

        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let adapter_info = adapter.get_info();
        println!(
            "Using backend: {:?}, device: {}, vendor: {:#x})",
            adapter_info.backend, adapter_info.name, adapter_info.vendor
        );

        let adapter_limits = adapter.limits();
        adapter_info::display_adapter_limits_info(&adapter_limits);

        let buffer_limits = adapter.limits();
        println!(
            "GPU's max buffer size: {}bytes",
            buffer_limits.max_buffer_size
        );

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web we'll have to disable some.
                    required_limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    memory_hints: Default::default(),
                },
                // Some(&std::path::Path::new("trace")), // Trace path
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an Srgb surface texture. Using a different
        // one will result all the colors comming out darker. If you want to support non
        // Srgb surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            desired_maximum_frame_latency: 2,
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        let depth_texture = DepthTexture::create_depth_texture(&device, &config, "depth_texture");

        let aspect = config.width as f32 / config.height as f32;

        let proj = Mat4::perspective_rh(std::f32::consts::FRAC_PI_4, aspect, 0.1, 200.0);

        let view = Mat4::look_at_rh(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO, Vec3::Y);

        let camera = cgmath::Vector3::new(0.0, 0.0, 10.0);

        let focus_point = cgmath::Vector3::new(0.0, 0.0, 0.0);

        let initial_mvp = proj * view * Mat4::IDENTITY;
        let uniform = Uniforms {
            transform: initial_mvp.to_cols_array_2d(),
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("uniform_bind_group_layout"),
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
            });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("uniform_bind_group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&uniform_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc(), Instance::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default() // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
                                     // or Features::POLYGON_MODE_POINT
                                     // polygon_mode: wgpu::PolygonMode::Fill,
                                     // // Requires Features::DEPTH_CLIP_CONTROL
                                     // unclipped_depth: false,
                                     // // Requires Features::CONSERVATIVE_RASTERIZATION
                                     // conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                // format: wgpu::TextureFormat::Depth24Plus,
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            // multisample: wgpu::MultisampleState {
            //     count: 1,
            //     mask: !0,
            //     alpha_to_coverage_enabled: false,
            // },
            multisample: wgpu::MultisampleState::default(),
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview: None,
            // Useful for optimizing shader compilation on Android
            cache: None,
        });

        // let (sphere_vertices, sphere_indices) = generate_uv_sphere(6, 8);
        // let vertices = VERTICES;
        // let indices = INDICES;

        let mut instances = Vec::new();
        let points_num = 100;
        // let spacing = 1.0;
        let scale = 0.05;

        for &(x, y, z) in laser_map_points.iter() {
            let pos = Vec3::new(x, y, z);
            instances.push(Instance {
                offset: pos.to_array(),
                scale: 0.1,
            })
        }

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instance Buffer"),
            size: (instances.len() * std::mem::size_of::<Instance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&instance_buffer, 0, bytemuck::cast_slice(&instances));

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            // contents: bytemuck::cast_slice(VERTICES),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            // contents: bytemuck::cast_slice(INDICES),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let num_indices = INDICES.len() as u32;

        Self {
            surface,
            device,
            queue,
            config,
            size,
            window,
            render_pipeline,
            instances,
            instance_buffer,
            vertex_buffer,
            index_buffer,
            uniform_bind_group,
            uniform_buffer,
            num_indices,
            rotation_angle: 0.0,
            depth_texture: depth_texture.texture,
            depth_view: depth_texture.view,
            view,
            proj,
            camera,
            focus_point,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.depth_texture =
                DepthTexture::create_depth_texture(&self.device, &self.config, "depth_texture")
                    .texture;
            self.surface.configure(&self.device, &self.config);

            let depth =
                DepthTexture::create_depth_texture(&self.device, &self.config, "depth_texture");
            self.depth_texture = depth.texture;
            self.depth_view = depth.view;
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        let movement = 4.0;
        // false
        if let WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    state: ElementState::Pressed,
                    physical_key: PhysicalKey::Code(keycode),
                    ..
                },
            ..
        } = event
        {
            match keycode {
                // For camera movement
                KeyCode::ArrowRight => {
                    self.camera.x += movement;
                    true
                }
                KeyCode::ArrowLeft => {
                    self.camera.x -= movement;
                    true
                }
                KeyCode::ArrowUp => {
                    self.camera.y -= movement;
                    true
                }
                KeyCode::ArrowDown => {
                    self.camera.y += movement;
                    true
                }
                KeyCode::PageUp => {
                    self.camera.z += movement;
                    true
                }
                KeyCode::PageDown => {
                    self.camera.z -= movement;
                    true
                }
                // For focus point movement
                KeyCode::KeyW => {
                    self.focus_point.x += movement;
                    true
                }
                KeyCode::KeyA => {
                    self.focus_point.x -= movement;
                    true
                }
                KeyCode::KeyE => {
                    self.focus_point.y += movement;
                    true
                }
                KeyCode::KeyS => {
                    self.focus_point.y -= movement;
                    true
                }
                KeyCode::KeyR => {
                    self.focus_point.z += movement;
                    true
                }
                KeyCode::KeyD => {
                    self.focus_point.z -= movement;
                    true
                }
                // Reset the coordination
                KeyCode::KeyQ => {
                    self.camera = cgmath::Vector3::new(0.0, 0.0, 20.0);
                    self.focus_point = cgmath::Vector3::new(0.0, 0.0, 0.0);
                    true
                }
                _ => false,
            }
        } else {
            false
        }
    }

    fn update(&mut self) {
        self.rotation_angle += 0.01;

        let rot_y = Mat4::from_axis_angle(Vec3::Y, self.rotation_angle);
        let rot_x = Mat4::from_axis_angle(Vec3::X, 0.0);
        let rot = rot_y * rot_x;

        // let tx = self.rotation_angle.tan() * 0.5;
        let tx = self.rotation_angle.tan() * 0.0;
        // let translation = Mat4::from_translation(Vec3::new(0.0, 0.0, 0.0));
        let translation = Mat4::from_translation(Vec3::new(tx, 0.0, tx));

        let model = translation * rot;

        let camera_pos = Vec3::new(self.camera.x, self.camera.y, self.camera.z);
        let focus_point = Vec3::new(self.focus_point.x, self.focus_point.y, self.focus_point.z);
        self.view = Mat4::look_at_rh(camera_pos, focus_point, Vec3::Y);

        let mvp = self.proj * self.view * model;
        let uniforms = Uniforms {
            transform: mvp.to_cols_array_2d(),
        };

        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                // depth_stencil_attachment: None,
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            // render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
            // Instances
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.draw_indexed(0..self.num_indices, 0, 0..self.instances.len() as _);
        }

        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Info).expect("Couldn't initialize logger");
        } else {
            env_logger::init();
        }
    }

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    // State::new uses async code, so we're going to wait for it to finish
    let mut state = State::new(&window).await;
    let mut surface_configured = false;

    event_loop
        .run(move |event, control_flow| {
            match event {
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == state.window().id() => {
                    if !state.input(event) {
                        // UPDATED!
                        match event {
                            WindowEvent::CloseRequested
                            | WindowEvent::KeyboardInput {
                                event:
                                    KeyEvent {
                                        state: ElementState::Pressed,
                                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                                        ..
                                    },
                                ..
                            } => control_flow.exit(),
                            WindowEvent::Resized(physical_size) => {
                                log::info!("physical_size: {physical_size:?}");
                                surface_configured = true;
                                state.resize(*physical_size);
                            }
                            WindowEvent::RedrawRequested => {
                                // This tells winit that we want another frame after this one
                                state.window().request_redraw();

                                if !surface_configured {
                                    return;
                                }

                                state.update();
                                match state.render() {
                                    Ok(_) => {}
                                    // Reconfigure the surface if it's lost or outdated
                                    Err(
                                        wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated,
                                    ) => state.resize(state.size),
                                    // The system is out of memory, we should probably quit
                                    Err(
                                        wgpu::SurfaceError::OutOfMemory | wgpu::SurfaceError::Other,
                                    ) => {
                                        log::error!("OutOfMemory");
                                        control_flow.exit();
                                    }

                                    // This happens when the a frame takes too long to present
                                    Err(wgpu::SurfaceError::Timeout) => {
                                        log::warn!("Surface timeout")
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        })
        .unwrap();
}
