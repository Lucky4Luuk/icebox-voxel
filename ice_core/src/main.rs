#[macro_use] extern crate log;

use vulkano::framebuffer::Subpass;
use std::sync::Arc;
use std::time::Instant;

use glam::*;

use ice_vox_mem::octree::VoxelOctree;

use ice_render::VkRenderer;
use ice_render::window::{create_window, get_physical_device, vulkan_setup};

use ice_ui::IceUI;

use vulkano::command_buffer::SubpassContents;

use winit::event::{
    Event,
    WindowEvent,
};
use winit::event_loop::ControlFlow;

fn main() {
    println!("Hello, world!");
    pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::max())
        .init();

    //Vulkan setup
    let (instance, event_loop, surface) = create_window(1280, 720, "Icebox");
    {
        let physical = get_physical_device(&instance);
        debug!("Physical device: {:?}", physical);
    }
    let (device, mut queues) = {
        let physical = get_physical_device(&instance);
        vulkan_setup(&instance, &physical)
    };
    let queue = queues.next().unwrap(); //Right now, our queues iter only contains 1 element

    //Create renderer
    let mut renderer = {
        let physical = get_physical_device(&instance);
        VkRenderer::new(&device, &queue, &physical, &surface)
    };

    //UI stuff
    //TODO: Render pass for UI and for voxel raytracing probably needs to be
    //different, so stop relying on the renderers current render pass
    let mut ui_manager = IceUI::new(&surface, &device, &queue, renderer.render_pass.clone());

    let start_time = Instant::now();
    let mut frame_time = Instant::now();

    event_loop.run(move |raw_event, _, control_flow| {
        ui_manager.platform.handle_event(&raw_event);
        if ui_manager.platform.captures_event(&raw_event) {
            return;
        }
        match raw_event {
            Event::MainEventsCleared => {
                ui_manager.platform.update_time(start_time.elapsed().as_secs_f64());

                let old_frame_time = frame_time;
                frame_time = Instant::now();
                let delta = frame_time - old_frame_time;
                let delta_s = delta.as_secs() as f32 + delta.subsec_nanos() as f32 / 1_000_000_000.0;

                //Updating

                //game.update(delta_s);

                //Rendering
                {
                    let physical = get_physical_device(&instance);
                    renderer.prepare_frame(&device, &queue, &physical, &surface);
                }

                //Render UI
                {
                    // let egui_data = ui_manager.render(&surface.window());

                    //Rendering
                    renderer.render(&device, &queue, &surface, &mut ui_manager, delta_s);
                }
            },
            Event::WindowEvent { ref event, ..} => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                _ => {},
            },
            _ => {},
        }
    });
}
