// This file produces a binary that loads multiple roms and emulates them simultaneously,
// rendering them in a grid using wgpu.

use robin_gb::common_types::*;
use robin_gb::debugger::Debugger;
use robin_gb::gpu::Gpu;
use robin_gb::GameBoy;
use std::fs;
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

// rwtodo: Put this and winit/wgpu behind a feature, as I don't want users of the robin_gb library to have to download them.

const GAME_BOYS_PER_COLUMN: u32 = 3;
const GAME_BOYS_PER_ROW: u32 = 3;
const WINDOW_WIDTH: u32 = 160 * GAME_BOYS_PER_COLUMN;
const WINDOW_HEIGHT: u32 = 144 * GAME_BOYS_PER_ROW;

#[derive(Default)]
struct App<'a> {
    window: Option<Arc<Window>>,
    gpu: Option<Gpu<'a>>,
    tex_nearest: usize,
    tex_linear: usize,
    game_boys: Vec<GameBoy>,
    tile_transforms: Vec<Mat4>,
    debugger: Debugger,
}

impl ApplicationHandler for App<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // TODO:
        let size = LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT);

        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("robin_gb smoke test")
                        .with_inner_size(size),
                )
                .unwrap(),
        );

        // Set up wgpu rendering and the transforms for the game boy screens.
        self.gpu = Some(pollster::block_on(Gpu::new(&window))); // TODO: Figure out how to move this complexity into gpu.rs.
        self.tex_nearest = self.gpu.as_mut().unwrap().create_texture(160, 144, false);
        self.tex_linear = self.gpu.as_mut().unwrap().create_texture(160, 144, true);
        self.window = Some(window.clone());

        // rwtodo: make this a command line argument.
        let paths = fs::read_dir("/Users/robin/Desktop/robin_gb/roms/romonly").unwrap();

        // Grab the first (GAME_BOYS_PER_ROW * GAME_BOYS_PER_COLUMN) roms from the folder.
        let roms = {
            let mut roms = vec![];
            for path in paths {
                let path = path.unwrap();

                // Skip non-files
                if !path.file_type().unwrap().is_file() {
                    continue;
                }

                // Skip non-.gb files
                let name = path.file_name().into_string().unwrap();
                if !name.ends_with(".gb") {
                    continue;
                }

                let bytes = fs::read(path.path()).unwrap();
                println!("{}", path.path().display());
                roms.push(bytes);
                if roms.len() == (GAME_BOYS_PER_ROW * GAME_BOYS_PER_COLUMN) as usize {
                    break;
                }
            }
            roms
        };

        // Boot up the game boys.
        self.game_boys = roms.iter().map(|rom| GameBoy::new(&rom)).collect();

        let fullscreen_transform = {
            let mut m = Mat4::IDENTITY;
            m.x_axis.x = 2.0;
            m.y_axis.y = 2.0;
            m.x_axis.w = -1.0;
            m.y_axis.w = -1.0;
            m
        };

        for column in 0..GAME_BOYS_PER_COLUMN {
            for row in 0..GAME_BOYS_PER_ROW {
                let mut tile_transform = fullscreen_transform;
                tile_transform.x_axis.x /= GAME_BOYS_PER_COLUMN as f32;
                tile_transform.y_axis.y /= GAME_BOYS_PER_ROW as f32;
                tile_transform.x_axis.w += (column as f32 / GAME_BOYS_PER_COLUMN as f32) * 2.0;
                tile_transform.y_axis.w += (row as f32 / GAME_BOYS_PER_COLUMN as f32) * 2.0;
                self.tile_transforms.push(tile_transform);
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.window.as_ref().unwrap().request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let gpu = self.gpu.as_mut().unwrap();
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                gpu.begin_frame();

                let mut screen: [u8; 160 * 144] = [0; 160 * 144];
                for i in 0..self.game_boys.len() {
                    self.game_boys[i].emulate_next_frame(&mut screen);
                    gpu.write_monochrome_texture(self.tex_nearest, &screen);
                    gpu.write_monochrome_texture(self.tex_linear, &screen);
                    gpu.render_textured_quad(self.tex_nearest, self.tile_transforms[i]);
                    gpu.render_textured_quad(self.tex_linear, self.tile_transforms[i + 1]);
                    break;
                }

                let positions = vec![
                    Vec2::new(0.1, 0.1),
                    Vec2::new(0.9, 0.1),
                    Vec2::new(0.1, 0.9),
                ];

                let colors = vec![
                    Vec4::new(0.0, 1.0, 0.0, 1.0),
                    Vec4::new(1.0, 0.0, 0.0, 1.0),
                    Vec4::new(0.0, 0.0, 1.0, 0.0),
                ];

                let mut m = self.tile_transforms[2];
                gpu.render_triangles(&positions, None, Some((self.tex_nearest, &positions)), m);
                m.x_axis.w += 0.2;
                gpu.render_triangles(&positions, Some(&colors), None, m);
                m.x_axis.w += 0.2;
                gpu.render_triangles(&positions, None, None, m);
                m.x_axis.w += 0.2;
                gpu.render_triangles(
                    &positions,
                    Some(&colors),
                    Some((self.tex_nearest, &positions)),
                    m,
                );

                self.debugger.render(gpu);
                gpu.finish_frame();
            }
            _ => (),
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    // Run the emulations and render to the grid.
    let mut app = App::default();
    let _ = event_loop.run_app(&mut app);
}
