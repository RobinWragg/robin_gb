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

        let mut vertices = Vec::<Vec2>::new();
        for prim in clipped_primitives {
            let mesh = match prim.primitive {
                egui::epaint::Primitive::Mesh(m) => m,
                _ => unreachable!(),
            };

            for index in mesh.indices {
                let pos = mesh.vertices[index as usize].pos;
                let v = Vec2::new(pos.x, pos.y);
                vertices.push(v);
            }
        }
        gpu.render_triangles(&vertices, Mat4::IDENTITY);
    }
}
