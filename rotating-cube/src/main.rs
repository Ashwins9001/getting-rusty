// DeviceExt creates frame buffer which is dedicated block of memory that stores pixel data fed to GPU
use wgpu::util::DeviceExt;

// import Mat4 and Vec3 which are data types that store a 4x4 matrix and 3x1 vec
// need 4x4 matrix to implement camera projection including rotation, translation, scaling and adding perspective
// to view frustum
use glam::{Mat4, Vec3};

// window event loop imports
use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

// bytemuck traits to safely copy uniforms to GPU
use bytemuck::{Pod, Zeroable};

// guarantee struct memory layout matches C, needed for GPU buffer
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct ModelUniform {
    model: [[f32; 4]; 4],
}

struct State {
    surface: wgpu::Surface, // target for rendering, usually screen
    device: wgpu::Device,   // handle to GPU
    queue: wgpu::Queue,     // queue of GPU commands
    config: wgpu::SurfaceConfiguration, // store surface settings (res, px format)

    render_pipeline: wgpu::RenderPipeline, // encapsulate GPU program (shaders, depth, blending)

    vertex_buffer: wgpu::Buffer, // store vertex data (positions, colors)
    index_buffer: wgpu::Buffer,  // stores indices to reuse vertex
    num_indices: u32,            // num indices in index_buffer

    camera_buffer: wgpu::Buffer, // store view matrix
    model_buffer: wgpu::Buffer,  // stores model matrix
    bind_group: wgpu::BindGroup, // groups of resources for GPU

    rotation: f32, // rotation value updated each frame
}

impl State {
    async fn new(window: &winit::window::Window) -> Self {
        // ----- Instance + Surface -----
        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        let surface = unsafe { instance.create_surface(window) }.unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .unwrap();

        // ----- Swapchain config -----
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_capabilities(&adapter).formats[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        // ----- Cube vertices -----
        #[rustfmt::skip]
        let vertices: &[f32] = &[
            // X     Y     Z     R   G   B
            -1.0,-1.0,-1.0, 1.0,0.0,0.0,
             1.0,-1.0,-1.0, 0.0,1.0,0.0,
             1.0, 1.0,-1.0, 0.0,0.0,1.0,
            -1.0, 1.0,-1.0, 1.0,1.0,0.0,
            -1.0,-1.0, 1.0, 1.0,0.0,1.0,
             1.0,-1.0, 1.0, 0.0,1.0,1.0,
             1.0, 1.0, 1.0, 1.0,1.0,1.0,
            -1.0, 1.0, 1.0, 0.0,0.0,0.0,
        ];

        let indices: &[u16] = &[
            0,1,2, 2,3,0,
            4,5,6, 6,7,4,
            0,4,7, 7,3,0,
            1,5,6, 6,2,1,
            3,2,6, 6,7,3,
            0,1,5, 5,4,0,
        ];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // ----- Camera (fixed) -----
        //define view matrix and starting position
        let view = Mat4::look_at_rh(
            Vec3::new(3.0, 3.0, 3.0), // camera position
            Vec3::ZERO,               // looks at origin
            Vec3::Y,                  // up direction
        );

        //define projection matrix and starting field of view, along with near and far-clipping limits to encapsulate frustum 
        let proj = Mat4::perspective_rh_gl(
            45f32.to_radians(),
            config.width as f32 / config.height as f32,
            0.1,
            100.0,
        );

        //define camera matrix as projection * view matrices and convert it to 2D array compatible with GPU func
        let camera_uniform = CameraUniform {
            view_proj: (proj * view).to_cols_array_2d(),
        };

        //create camera and model vertex buffers that will contain each vertex as [[x, y, z],[r,g,b]]
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::bytes_of(&camera_uniform),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // ----- Model (rotation updated each frame) -----
        let model_uniform = ModelUniform {
            model: Mat4::IDENTITY.to_cols_array_2d(),
        };

        let model_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Model Buffer"),
            contents: bytemuck::bytes_of(&model_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        //define bindings so GPU knows how to access each vertex correctly
        // ----- Bind Group Layout -----
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                // camera
                wgpu::BindGroupLayoutEntry {
                    binding: 0, //camera information for vertex shader
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // model
                wgpu::BindGroupLayoutEntry {
                    binding: 1, //model information for vertex shader
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: model_buffer.as_entire_binding(),
                },
            ],
        });

        // ----- Shader -----
        //reference the shader module
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        // ----- Pipeline -----
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { 
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 6 * 4, //each vertex has 6 floating point values at 4 bytes each, hence each is 6*4=24 bytes 
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            shader_location: 0,
                            offset: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        wgpu::VertexAttribute {
                            shader_location: 1,
                            offset: 12, //recall the last three values are color, reference these directly in GPU to proc together by offset 12 (3 floats at 4 bytes each = 4*3=12 byte offset)
                            format: wgpu::VertexFormat::Float32x3,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Self {
            surface,
            device,
            queue,
            config,
            render_pipeline,

            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,

            camera_buffer,
            model_buffer,
            bind_group,

            rotation: 0.0,
        }
    }

    fn update(&mut self) {
        // Rotate the cube every frame
        self.rotation += 0.01;
        let rot = Mat4::from_rotation_y(self.rotation) * Mat4::from_rotation_x(self.rotation * 0.5); //define rotation matrix along y and x-axes with fom_rotation_y/x func

        let model = ModelUniform {
            model: rot.to_cols_array_2d(), //convert to 2D array again for GPU to understand
        };

        self.queue.write_buffer(&self.model_buffer, 0, bytemuck::bytes_of(&model)); //load the model information to buffer after rotation changes applied
    }

    fn render(&mut self) {
        let frame = self.surface.get_current_texture().unwrap();
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default()); //get current texture and display it (vertices proc by shader)

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None }); //write GPU commands and encode them 

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { //render pass to black out view
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            pass.set_pipeline(&self.render_pipeline); //set up the pipeline and bindings, then fetch vertex information from buffer after shader has applied position and color transformations
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..self.num_indices, 0, 0..1); //draw command 
        }

        self.queue.submit(Some(encoder.finish())); //send to encoder and call on GPU to present it
        frame.present();
    }
}

fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().with_title("Rotating Cube").build(&event_loop).unwrap();

    let mut state = pollster::block_on(State::new(&window));

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::MainEventsCleared => {
                state.update();
                state.render();
            }
            _ => {}
        }
    });
}
