use crate::common_types::*;
use crate::gpu::Gpu;
use egui::epaint::{image::ImageData, textures::*};
use egui::{self, FontImage};
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
            egui::CentralPanel::default().show(&ctx, |ui| {
                ui.label("Hello world!");
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

            let gpu_tex_id = gpu.create_texture(font_image.size[0], font_image.size[1]);
            let srgba_pixels = font_image.srgba_pixels(None);
            let mut monochrome_pixels = Vec::with_capacity(srgba_pixels.len());
            for pixel in srgba_pixels {
                monochrome_pixels.push(pixel.r());
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

        let clipped_primitives = self
            .ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        for prim in clipped_primitives {
            let mesh = match prim.primitive {
                egui::epaint::Primitive::Mesh(m) => m,
                _ => panic!(),
            };

            let mut vert_positions = Vec::with_capacity(mesh.indices.len());
            let mut vert_texcoords = Vec::with_capacity(mesh.indices.len());
            for index in mesh.indices {
                let vert = mesh.vertices[index as usize];
                vert_positions.push(v2::new(vert.pos.x, vert.pos.y));
                vert_texcoords.push(v2::new(vert.uv.x, vert.uv.y));
            }

            let egui_tex_id = match mesh.texture_id {
                egui::TextureId::Managed(id) => id,
                _ => panic!(),
            };

            let gpu_tex_id = *self.egui_to_gpu_tex_id.get(&egui_tex_id).unwrap();
            assert!(gpu_tex_id != 0);

            let scale = 0.01; // TODO: Arbitrary.
            let scale_matrix = Mat4::from_scale(v3::new(scale, -scale, 1.0));
            gpu.render_textured_triangles(
                &vert_positions,
                &vert_texcoords,
                gpu_tex_id,
                scale_matrix,
            );
        }
    }
}
