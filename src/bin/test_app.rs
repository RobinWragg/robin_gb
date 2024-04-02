use futures::executor::block_on;
use robin_gb;
use std::fs;
use wgpu;
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

// rwtodo: This will become a simple app that loads multiple instances of the emulator and loads a different game in each one.

// rwtodo: Put this and winit/wgpu behind a feature, as I don't want users of the robin_gb library to have to download them.

const WINDOW_WIDTH: u32 = 160 * 4;
const WINDOW_HEIGHT: u32 = 144 * 4;

async fn wgpu_init(window: &Window) -> (wgpu::Surface, wgpu::Device, wgpu::Queue) {
    let (surface, adapter) = {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface = instance.create_surface(window).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        (surface, adapter)
    };

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                label: None,
            },
            None,
        )
        .await
        .unwrap();

    let config = surface
        .get_default_config(&adapter, WINDOW_WIDTH, WINDOW_HEIGHT)
        .unwrap();
    {
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });
    }

    surface.configure(&device, &config);

    (surface, device, queue)
}

fn render(surface: &wgpu::Surface, device: &wgpu::Device, queue: &wgpu::Queue) {
    let output = surface.get_current_texture().unwrap();

    let encoder = {
        let clear_color = wgpu::Color {
            r: 0.2,
            g: 0.3,
            b: 0.1,
            a: 1.0,
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        let _ = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear_color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        encoder
    };

    queue.submit(std::iter::once(encoder.finish()));
    output.present();
}

fn main() {
    let roms = [fs::read("roms/Tetris.gb").unwrap()];
    let mut game_boys = roms.map(|rom| robin_gb::GameBoy::new(&rom));

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let size = PhysicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT);
    let window = WindowBuilder::new()
        .with_title("robin_gb")
        .with_inner_size(size)
        .build(&event_loop)
        .unwrap();

    let (surface, device, queue) = block_on(wgpu_init(&window));

    let _ = event_loop.run(move |event, elwt| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            println!("The close button was pressed; stopping");
            elwt.exit();
        }
        Event::AboutToWait => {
            let game_boy_screens = game_boys.iter_mut().map(|gb| gb.emulate_next_frame());
            render(&surface, &device, &queue);
        }
        _ => (),
    });
}
