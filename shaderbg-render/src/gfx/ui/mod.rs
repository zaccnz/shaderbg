mod scene;
pub use scene::Scene;

use egui::{Context, FullOutput, RawInput};
use egui_wgpu::{renderer::ScreenDescriptor, Renderer};
use wgpu::{
    CommandEncoder, Device, Queue, RenderPassColorAttachment, RenderPassDescriptor, TextureFormat,
    TextureView,
};

pub struct Ui {
    pub context: Context,
    renderer: Renderer,
}

impl Ui {
    pub fn new(device: &Device, format: TextureFormat) -> Ui {
        Ui {
            context: Context::default(),
            renderer: Renderer::new(device, format, None, 1),
        }
    }

    pub fn context(&self) -> &Context {
        &self.context
    }

    pub fn render<F: FnOnce(&egui::Context)>(
        &mut self,
        encoder: &mut CommandEncoder,
        device: &Device,
        queue: &Queue,
        view: &TextureView,
        render: F,
        input: RawInput,
        pixels_per_point: f32,
        width: u32,
        height: u32,
    ) -> FullOutput {
        let output = self.context.run(input, render);

        let paint_jobs = self.context.tessellate(output.shapes.clone());
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [width, height],
            pixels_per_point,
        };

        for (id, image_delta) in &output.textures_delta.set {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
        }

        self.renderer
            .update_buffers(device, queue, encoder, &paint_jobs, &screen_descriptor);

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("egui render pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
            self.renderer
                .render(&mut render_pass, &paint_jobs, &screen_descriptor);
        }

        output
    }
}
