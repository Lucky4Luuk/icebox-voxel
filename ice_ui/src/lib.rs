use vulkano::framebuffer::Subpass;
use vulkano::framebuffer::RenderPassAbstract;
use std::sync::Arc;

use winit::window::Window;

use vulkano::device::{Device, Queue};
use vulkano::swapchain::Surface;

use egui_winit_platform::{Platform, PlatformDescriptor};

pub struct IceUI {
    pub platform: Platform,
    pub painter: egui_vulkano::Painter<Arc<dyn RenderPassAbstract + Send + Sync>>,
}

impl IceUI {
    pub fn new(surface: &Arc<Surface<Window>>, device: &Arc<Device>, queue: &Arc<Queue>, render_pass: Arc<dyn RenderPassAbstract + Sync + Send>) -> Self {
        let size = surface.window().inner_size();
        let platform = Platform::new(PlatformDescriptor {
            physical_width: size.width as u32,
            physical_height: size.height as u32,
            scale_factor: surface.window().scale_factor(),
            font_definitions: egui::FontDefinitions::default(),
            style: Default::default(),
        });

        let painter = egui_vulkano::Painter::new(device.clone(), queue.clone(), Subpass::from(render_pass, 1).unwrap()).unwrap();

        let mut style = egui::Style::default();
        style.visuals.window_corner_radius = 0.0;
        style.visuals.window_shadow.extrusion = 0.5;
        let mut text_col = style.visuals.text_color();
        println!("text_col: {:?}", text_col);
        text_col[0] += 55;
        text_col[1] += 55;
        text_col[2] += 55;
        println!("text_col: {:?}", text_col);
        style.visuals.override_text_color = Some(text_col);

        platform.context().set_style(style);

        Self {
            platform: platform,
            painter: painter,
        }
    }

    pub fn render(&mut self, window: &winit::window::Window, delta_s: f32) -> (egui::Output, Vec<egui::paint::ClippedShape>) {
        self.platform.begin_frame();

        egui::Window::new("Debug")
            .scroll(true)
            .show(&self.platform.context(), |ui| {
                ui.label(format!("FPS: {} / {:.2} ms", 1.0 / delta_s, delta_s * 1000.0));
                ui.separator();
                ui.hyperlink("https://github.com/lucky4luuk/icebox-voxel");
            });

        self.platform.end_frame()
    }
}
