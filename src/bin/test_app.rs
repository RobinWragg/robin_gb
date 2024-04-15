use futures::executor::block_on;
use robin_gb;
use std::fs;
use std::sync::Arc;
use wgpu;
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

// rwtodo: This will become a simple app that loads multiple instances of the emulator and loads a different game in each one.

// rwtodo: Put this and winit/wgpu behind a feature, as I don't want users of the robin_gb library to have to download them.

struct GpuState<'a> {
    surface: wgpu::Surface<'a>,
}

impl GpuState<'static> {
    async fn new(window: &Arc<Window>) -> GpuState<'static> {
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
        Self { surface }
    }
}

const WINDOW_WIDTH: u32 = 160 * 4;
const WINDOW_HEIGHT: u32 = 144 * 4;

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
        multiview: None, // 5.
    })
}

async fn wgpu_init(
    window: &Window,
) -> (
    wgpu::Surface,
    wgpu::Device,
    wgpu::Queue,
    wgpu::RenderPipeline,
    wgpu::Texture,
    wgpu::BindGroup,
) {
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
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
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
                resource: wgpu::BindingResource::TextureView(&texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
        label: None,
    });

    let pipeline = create_pipeline(&device, &config, &bind_group_layout);

    (surface, device, queue, pipeline, texture, bind_group)
}

fn render(
    surface: &wgpu::Surface,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipeline: &wgpu::RenderPipeline,
    texture: &wgpu::Texture,
    bind_group: &wgpu::BindGroup,
    game_boy_screen: &Vec<u8>, // rwtodo: I think I can pass around a fixed-size array, but I would have to keep moving it.
) {
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture,
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

    let output = surface.get_current_texture().unwrap();

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
    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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

        render_pass.set_pipeline(&pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..4, 0..1);
    } // We're dropping render_pass here to unborrow the encoder.

    queue.submit(std::iter::once(encoder.finish()));
    output.present();
}

fn main() {
    let roms = [fs::read("roms/Tetris.gb").unwrap()];
    let mut game_boys = roms.map(|rom| robin_gb::GameBoy::new(&rom));

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let size = PhysicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT);
    let window: Arc<Window> = WindowBuilder::new()
        .with_title("robin_gb")
        .with_inner_size(size)
        .build(&event_loop)
        .unwrap()
        .into();

    let state = GpuState::new(&window);

    let (surface, device, queue, pipeline, texture, bind_group) = block_on(wgpu_init(&window));

    let _ = event_loop.run(move |event, elwt| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            println!("The close button was pressed; stopping");
            elwt.exit();
        }
        Event::AboutToWait => {
            let screen = game_boys[0].emulate_next_frame(); // Just emulate one game boy for now.
            render(
                &surface,
                &device,
                &queue,
                &pipeline,
                &texture,
                &bind_group,
                &screen,
            );
        }
        _ => (),
    });
}
