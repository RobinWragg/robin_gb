// This file produces a binary that loads multiple roms and emulates them simultaneously,
// rendering them in a grid using wgpu.

use bytemuck;
use glam::f32::Mat4;
use robin_gb::GameBoy;
use std::fs;
use std::sync::Arc;
use wgpu;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

// rwtodo: Put this and winit/wgpu behind a feature, as I don't want users of the robin_gb library to have to download them.

const GAME_BOYS_PER_COLUMN: u32 = 4;
const GAME_BOYS_PER_ROW: u32 = 4;
const WINDOW_WIDTH: u32 = 160 * GAME_BOYS_PER_COLUMN;
const WINDOW_HEIGHT: u32 = 144 * GAME_BOYS_PER_ROW;

const TEXTURE_SIZE: wgpu::Extent3d = wgpu::Extent3d {
    width: 160,  // rwtodo: constants
    height: 144, // rwtodo: constants
    depth_or_array_layers: 1,
};

fn create_pipeline(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: config.format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    })
}

struct GpuState<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    matrix_buffer: wgpu::Buffer,
}

impl<'a> GpuState<'a> {
    async fn new(window: &Arc<Window>) -> GpuState<'a> {
        let (surface, adapter) = {
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends: wgpu::Backends::all(),
                ..Default::default()
            });
            let surface = instance.create_surface(window.clone()).unwrap();
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
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .unwrap();

        let size = window.inner_size(); // Size in physical pixels
        let config = surface
            .get_default_config(&adapter, size.width, size.height)
            .unwrap();

        surface.configure(&device, &config);

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: TEXTURE_SIZE,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm, // One byte per pixel
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: None,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            // rwtodo: what are the defaults?
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let matrix_buffer = {
            let desc = wgpu::BufferDescriptor {
                label: None,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                size: std::mem::size_of::<Mat4>() as u64,
                mapped_at_creation: false,
            };
            // device.create_buffer_init(&d)
            device.create_buffer(&desc)
        };
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    // This should match the filterable field of the
                    // corresponding Texture entry above.
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: matrix_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: None,
        });

        let pipeline = create_pipeline(&device, &config, &bind_group_layout);

        Self {
            surface,
            device,
            queue,
            pipeline,
            texture,
            bind_group,
            matrix_buffer,
        }
    }

    #[must_use]
    fn begin_render(&self) -> wgpu::SurfaceTexture {
        self.surface.get_current_texture().unwrap()
        // rwtodo: clear.
    }

    fn finish_render(&self, surface_texture: wgpu::SurfaceTexture) {
        surface_texture.present();
    }

    // rwtodo: I think I can pass around a fixed-size array, but I would have to keep moving it.
    fn render_gb_screen(
        &self,
        surface_texture: &wgpu::SurfaceTexture,
        game_boy_screen: &[u8],
        matrix: Mat4,
    ) {
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            game_boy_screen,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(160),  // rwtodo: constant
                rows_per_image: Some(144), // rwtodo: constant
            },
            TEXTURE_SIZE,
        );

        let matrix_floats = matrix.to_cols_array();
        let matrix_bytes = bytemuck::bytes_of(&matrix_floats);
        self.queue
            .write_buffer(&self.matrix_buffer, 0, matrix_bytes);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let view = surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.draw(0..4, 0..1);
        } // We're dropping render_pass here to unborrow the encoder.

        self.queue.submit(std::iter::once(encoder.finish()));
    }
}

#[derive(Default)]
struct App<'a> {
    window: Option<Arc<Window>>,
    state: Option<GpuState<'a>>,
    game_boys: Vec<GameBoy>,
    tile_transforms: Vec<Mat4>,
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
        self.state = Some(pollster::block_on(GpuState::new(&window)));
        self.window = Some(window.clone());

        // rwtodo: make this a command line argument.
        let paths = fs::read_dir("roms/romonly").unwrap();

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
        let state = self.state.as_ref().unwrap();
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                let surface_texture = state.begin_render();
                let mut screen: [u8; 160 * 144] = [0; 160 * 144];
                for i in 0..self.game_boys.len() {
                    self.game_boys[i].emulate_next_frame(&mut screen);
                    state.render_gb_screen(&surface_texture, &screen, self.tile_transforms[i]);
                }

                state.finish_render(surface_texture);
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
