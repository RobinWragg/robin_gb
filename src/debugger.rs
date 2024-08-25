use crate::common_types::*;
use crate::gpu::Gpu;
use egui;
use egui::epaint::{image::ImageData, textures::*};

#[derive(Default)]
pub struct Debugger {
    ctx: egui::Context,
}

impl Debugger {
    pub fn render(&self, gpu: &Gpu) {
        let raw_input = egui::RawInput::default();
        let full_output = self.ctx.run(raw_input, |ctx| {
            egui::CentralPanel::default().show(&ctx, |ui| {
                ui.label("Hello world!");
            });
        });

        if !full_output.textures_delta.set.is_empty() {
            assert_eq!(full_output.textures_delta.set.len(), 1);
            let (tex_id, delta) = &full_output.textures_delta.set[0];
            assert_eq!(delta.options.magnification, TextureFilter::Linear);
            assert_eq!(delta.options.minification, TextureFilter::Linear);
            assert_eq!(delta.options.wrap_mode, TextureWrapMode::ClampToEdge);
            assert_eq!(delta.pos, None);
            let font_image = match &delta.image {
                ImageData::Color(_) => todo!(),
                ImageData::Font(f) => f,
            };
            dbg!(font_image.size);
            dbg!(font_image.pixels[0]);
        }
        assert!(full_output.textures_delta.free.is_empty());

        let clipped_primitives = self
            .ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        for prim in clipped_primitives {
            let mesh = match prim.primitive {
                egui::epaint::Primitive::Mesh(m) => m,
                _ => unreachable!(),
            };

            let mut vert_positions = Vec::with_capacity(mesh.indices.len());
            let mut vert_texcoords = Vec::with_capacity(mesh.indices.len());
            for index in mesh.indices {
                let vert = mesh.vertices[index as usize];
                vert_positions.push(v2::new(vert.pos.x, vert.pos.y));
                vert_texcoords.push(v2::new(vert.uv.x, vert.uv.y));
            }

            let tex_id = match mesh.texture_id {
                egui::TextureId::Managed(id) => id,
                _ => unreachable!(),
            };

            let scale = 0.01; // TODO: Arbitrary.
            let scale_matrix = Mat4::from_scale(v3::new(scale, scale, 1.0));
            gpu.render_textured_triangles(&vert_positions, &vert_texcoords, scale_matrix);
        }
    }
}
