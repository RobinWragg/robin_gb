use crate::common_types::*;
use crate::gpu::Gpu;
use egui;
use egui::epaint::{image::ImageData, textures::*};
use std::collections::HashMap;

#[derive(Default)]
pub struct Debugger {
    ctx: egui::Context,
    egui_to_gpu_tex_id: HashMap<u64, usize>,
}

impl Debugger {
    pub fn render(&mut self, gpu: &mut Gpu) {
        let raw_input = egui::RawInput::default();
        let full_output = self.ctx.run(raw_input, |ctx| {
            egui::Window::new("window!").show(&ctx, |ui| {
                ui.label("Hello world!");
                let mut wat = false;
                ui.checkbox(&mut wat, "checkbox");
                let _ = ui.button("button");
                let mut slider_value = 30.0;
                ui.add(egui::Slider::new(&mut slider_value, 0.0..=100.0).text("My value"));
            });
        });

        if !full_output.textures_delta.set.is_empty() {
            assert_eq!(full_output.textures_delta.set.len(), 1);
            let (egui_tex_id, delta) = &full_output.textures_delta.set[0];
            assert_eq!(delta.options.magnification, TextureFilter::Linear);
            assert_eq!(delta.options.minification, TextureFilter::Linear);
            assert_eq!(delta.options.wrap_mode, TextureWrapMode::ClampToEdge);
            assert_eq!(delta.pos, None);
            let font_image = match &delta.image {
                ImageData::Color(_) => panic!(),
                ImageData::Font(f) => f,
            };

            let gpu_tex_id = gpu.create_texture(font_image.size[0], font_image.size[1], true);
            let srgba_pixels = font_image.srgba_pixels(None);
            let mut monochrome_pixels = Vec::with_capacity(srgba_pixels.len());
            for pixel in srgba_pixels {
                monochrome_pixels.push(pixel.r());
                if pixel.r() != 0 {
                    println!("{} {} {} {}", pixel.r(), pixel.g(), pixel.b(), pixel.a());
                }
            }
            gpu.write_texture(gpu_tex_id, &monochrome_pixels);

            let egui_tex_id = match egui_tex_id {
                egui::TextureId::Managed(id) => *id,
                _ => panic!(),
            };
            assert!(egui_tex_id == 0);

            self.egui_to_gpu_tex_id.insert(egui_tex_id, gpu_tex_id);
        }
        assert!(full_output.textures_delta.free.is_empty());

        dbg!(full_output.shapes.len());
        let only = 1;
        let mut counter = -1;
        for prim in self
            .ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point)
        {
            counter += 1;
            if only != counter {
                continue;
            }
            let mesh = match prim.primitive {
                egui::epaint::Primitive::Mesh(m) => m,
                _ => panic!(),
            };

            let mut vert_positions = Vec::with_capacity(mesh.indices.len());
            let mut vert_colors = Vec::with_capacity(mesh.indices.len());
            let mut vert_uvs = Vec::with_capacity(mesh.indices.len());
            for index in mesh.indices {
                let vert = mesh.vertices[index as usize];
                vert_positions.push(Vec2::new(vert.pos.x, vert.pos.y));
                let rgba = vert.color.to_array(); // TODO: this is premultiplied
                vert_colors.extend_from_slice(&rgba);
                vert_uvs.push(Vec2::new(vert.uv.x, vert.uv.y));
            }

            let egui_tex_id = match mesh.texture_id {
                egui::TextureId::Managed(id) => id,
                _ => panic!(),
            };

            let gpu_tex_id = *self.egui_to_gpu_tex_id.get(&egui_tex_id).unwrap();
            assert!(gpu_tex_id != 0);

            let scale_x = 2.0 / gpu.width() as f32; // TODO: Arbitrary.
            let scale_y = 2.0 / gpu.height() as f32; // TODO: Arbitrary.
            let scale_matrix = Mat4::from_scale(Vec3::new(scale_x, -scale_y, 1.0));
            gpu.render_triangles(
                &vert_positions,
                None, // TODO
                Some((gpu_tex_id, &vert_uvs)),
                scale_matrix,
            );
        }
    }
}
