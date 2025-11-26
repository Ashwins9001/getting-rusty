/*
swap-chain is a queue of textures (frames) that the GPU uses for rendering and displaying on-screen, it allows for smooth tear-free images
GPU renders to one texture while another is displayed, once rendering is done the texture is swapped to the screen
GPU given drawing commands that define what color is displayed to each pixel of a texture

implicitly when state.surface.configure(&state.device, &state.config); called: state.surface connects to OS window
state.config includes w/h of window, pixel format, usage, how frames are swapped (present mode)
configure() tells WGPU to make textures that can be rendered

calling: let frame = state.surface.get_current_texture();
will get next available texture from swap chain, render it using a view and command encoder, then frame.present()
swaps it to screen

WGPU is a rust lib that talks to GPU via system's graphics API, which for windows is usually DirectX/Vulkan
let instance = wgpu::Instance::new(wgpu::Backends::all()); -> WGPU finds all available APIs and selects
*/


//import wgpu library
use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

//create State that keeps track of surface rendered, queue for frame buffer, device connection to GPU and general surface configs
//general import syntax crate::module::type where crate is the package, module is a namespace, and type is the custom data-type formed 
struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
}

//Implement the type
impl State {
    // Async constructor
    //use & to indicate we are borrowing the Window instance
    async fn new(window: &winit::window::Window) -> Self {
        // Create instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
        });
            
        // Create surface (unsafe because it interacts with OS window)
        let surface = unsafe { instance.create_surface(window) }.unwrap();
        
        // Request an adapter (GPU)
        //..Default::default() syntax indicates that all other fields of type (wgpu::RequestAdapterOptions here) are set to default values
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            },
        ).await.unwrap();
        
        // Request device and queue
        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
                label: None,
            },
            None,
        ).await.unwrap();
        
        // Get surface capabilities and choose a format
        // search through all &Format types from surface_caps, generate vector via iter(), copy them to get reference, then run a closure (f.is_srgb()) that checks 
        // if srgb surface found
        let surface_caps = surface.get_capabilities(&adapter);
        let format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        
        // Configure surface
        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);
        
        Self { surface, device, queue, config }
    }
}

fn main() {
    // Create event loop and window
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("WGPU Example")
        .build(&event_loop)
        .unwrap();
    

    // Initialize GPU state asynchronously
    let mut state = pollster::block_on(State::new(&window));

    // Start the event loop
    // Create another closure called move that has event, control_flow as params
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            // match window event where it is either closed or resized
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(size) => {
                    state.config.width = size.width;
                    state.config.height = size.height;
                    state.surface.configure(&state.device, &state.config);
                }
                _ => {}
            },
            // match redraw requested event where window is successfully redrawn or fails
            Event::RedrawRequested(_) => {
                // Acquire next frame
                let frame = match state.surface.get_current_texture() {
                    Ok(frame) => frame,
                    Err(_) => {
                        state.surface.configure(&state.device, &state.config);
                        state.surface.get_current_texture().unwrap()
                    }
                };
                
                //get window frame, apply texture, and encode this to commands to send to GPU to render 
                //don't modify frame directly using GPU, create a view for it to modify 
                let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
                let mut encoder = state.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") });

                // Begin render pass (clear screen to black)
                {
                    let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Render Pass"),
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
                }

                // Submit commands
                state.queue.submit(Some(encoder.finish()));
                frame.present();
            }
            Event::MainEventsCleared => window.request_redraw(),
            _ => {}
        }
    });
}
