use crate::common_types::*;
use crate::gpu::Gpu;
use egui;

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

            let scale = 0.01; // TODO: Arbitrary.
            let scale_matrix = Mat4::from_scale(v3::new(scale, scale, 1.0));
            gpu.render_textured_triangles(&vert_positions, &vert_texcoords, scale_matrix);
        }
    }
}
