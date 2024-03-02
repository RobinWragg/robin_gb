use robin_gb;
use std::fs;
use wgpu;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

// rwtodo: This will become a simple app that loads multiple instances of the emulator and loads a different game in each one.

// rwtodo: Put this and winit/wgpu behind a feature, as I don't want users of the robin_gb library to have to download them.

fn main() {
    println!("Hello, world!");
    let rom_file_data = fs::read("roms/Tetris.gb").unwrap();
    let mut gb = robin_gb::GameBoy::new(&rom_file_data[..]);

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let surface = instance.create_surface(&window).unwrap();

    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
    });

    let _ = event_loop.run(move |event, elwt| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            println!("The close button was pressed; stopping");
            elwt.exit();
        }
        Event::AboutToWait => {
            let _ = gb.emulate_next_frame();
        }
        _ => (),
    });
}
