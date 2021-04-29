#[macro_use] extern crate log;

use vulkano::framebuffer::RenderPass;
use std::sync::Arc;

use winit::window::Window;

use vulkano::sync::GpuFuture;
use vulkano::SafeDeref;

// use vulkano::buffer::BufferUsage;
// use vulkano::buffer::CpuAccessibleBuffer;

use vulkano::pipeline::viewport::Viewport;

use vulkano::framebuffer::Framebuffer;
use vulkano::framebuffer::FramebufferAbstract;
use vulkano::framebuffer::RenderPassAbstract;

use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::command_buffer::DynamicState;
use vulkano::command_buffer::SubpassContents;

use vulkano::swapchain::Surface;
use vulkano::swapchain::Swapchain;

use vulkano::image::SwapchainImage;
use vulkano::image::view::ImageView;

use vulkano::instance::PhysicalDevice;

use vulkano::device::Queue;
use vulkano::device::Device;

pub mod window;

/// This method is called once during initialization, then again whenever the window is resized
fn window_size_dependent_setup(
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
    dynamic_state: &mut DynamicState,
) -> Vec<Arc<dyn FramebufferAbstract + Send + Sync + 'static>> {
    let dimensions = images[0].dimensions();

    let viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [dimensions[0] as f32, dimensions[1] as f32],
        depth_range: 0.0..1.0,
    };
    dynamic_state.viewports = Some(vec![viewport]);

    images
        .iter()
        .map(|image| {
            Arc::new(
                Framebuffer::start(render_pass.clone())
                    .add(ImageView::new(image.clone()).unwrap())
                    .unwrap()
                    .build()
                    .unwrap(),
            ) as Arc<dyn FramebufferAbstract + Send + Sync>
        })
        .collect::<Vec<_>>()
}

pub struct VkRenderer {
    previous_frame_end: Option<Box<dyn GpuFuture>>,

    //Swapchain
    swapchain: Arc<Swapchain<Window>>,
    swapchain_images: Vec<Arc<SwapchainImage<Window>>>,
    recreate_swapchain: bool, //If true, recreates the swapchain. Needed for resizing (and certain platform quirks)

    pub render_pass: Arc<dyn RenderPassAbstract + Send + Sync + 'static>,
    dynamic_state: DynamicState,

    //Framebuffers
    framebuffers: Vec<Arc<dyn FramebufferAbstract + Send + Sync + 'static>>,
}

impl VkRenderer {
    pub fn new(device: &Arc<Device>, queue: &Arc<Queue>, physical: &PhysicalDevice, surface: &Arc<Surface<Window>>) -> Self {
        //This will keep track of our last submission, so we can clean up resources
        let previous_frame_end = Some(vulkano::sync::now(device.clone()).boxed());
        trace!("Basic setup completed!");

        trace!("Buffer setup completed!");

        let (swapchain, swapchain_images) = window::create_swapchain(device, queue, physical, surface);
        trace!("Swapchain setup completed!");

        let render_pass = Arc::new(
            vulkano::ordered_passes_renderpass!(
                device.clone(),
                attachments: {
                    color: {
                        load: Clear,
                        store: Store,
                        format: swapchain.format(),
                        samples: 1,
                    }
                },
                passes: [
                    { color: [color], depth_stencil: {}, input: [] },
                    { color: [color], depth_stencil: {}, input: [] } // Create a second renderpass to draw egui
                ]
            )
            .unwrap(),
        );


        // Dynamic viewports allow us to recreate just the viewport when the window is resized
        // Otherwise we would have to recreate the whole pipeline.
        let mut dynamic_state = DynamicState {
            line_width: None,
            viewports: None,
            scissors: None,
            compare_mask: None,
            write_mask: None,
            reference: None,
        };

        let framebuffers = window_size_dependent_setup(&swapchain_images[..], render_pass.clone(), &mut dynamic_state);
        trace!("Framebuffers setup completed!");

        Self {
            previous_frame_end: previous_frame_end,

            swapchain: swapchain,
            swapchain_images: swapchain_images,
            recreate_swapchain: false,

            render_pass: render_pass,
            dynamic_state: dynamic_state,

            framebuffers: framebuffers,
        }
    }

    pub fn render(&mut self, device: &Arc<Device>, queue: &Arc<Queue>, surface: &Arc<Surface<Window>>, ui_manager: &mut ice_ui::IceUI, delta_s: f32)
    {
        //This function polls various fences in order to determine what the GPU has already
        //processed, and frees up any resources that are no longer needed
        // self.previous_frame_end.as_mut().unwrap().cleanup_finished();
        if let &mut Some(ref mut prev_frame_end) = &mut self.previous_frame_end {
            prev_frame_end.cleanup_finished();
        } else {
            panic!("Something went wrong!");
        }

        if self.recreate_swapchain {
            let dimensions: [u32; 2] = surface.window().inner_size().into();
            let (swapchain, images) = self.swapchain.recreate_with_dimensions(dimensions).expect("Failed to recreate swapchain!");
            self.swapchain = swapchain;
            self.swapchain_images = images;
            self.recreate_swapchain = false;

            self.framebuffers = window_size_dependent_setup(&self.swapchain_images[..], self.render_pass.clone(), &mut self.dynamic_state);
            trace!("Swapchain recreated!");
        }

        // Before we can draw on the output, we have to *acquire* an image from the swapchain. If
        // no image is available (which happens if you submit draw commands too quickly), then the
        // function will block.
        // This operation returns the index of the image that we are allowed to draw upon.
        //
        // This function can block if no image is available. The parameter is an optional timeout
        // after which the function call will return an error.
        let (image_num, suboptimal, acquire_future) =
            match vulkano::swapchain::acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => r,
                Err(vulkano::swapchain::AcquireError::OutOfDate) => {
                    trace!("Swapchain needs to be recreated!");
                    self.recreate_swapchain = true;
                    return;
                }
                Err(e) => panic!("Failed to acquire next image: {:?}", e),
            };

        // acquire_next_image can be successful, but suboptimal. This means that the swapchain image
        // will still work, but it may not display correctly. With some drivers this can be when
        // the window resizes, but it may not cause the swapchain to become out of date.
        if suboptimal {
            trace!("Swapchain needs to be recreated!");
            self.recreate_swapchain = true;
        }

        let clear_values = vec![[0.8, 0.15, 0.9, 1.0].into()];

        let mut main_cmd_builder = AutoCommandBufferBuilder::primary_one_time_submit(device.clone(), queue.family()).unwrap();
        main_cmd_builder.begin_render_pass(self.framebuffers[image_num].clone(), SubpassContents::Inline, clear_values).unwrap();

        let (_output, clipped_shapes) = ui_manager.render(surface.window(), delta_s);
        let size = surface.window().inner_size();
        ui_manager.painter.draw(&mut main_cmd_builder, &self.dynamic_state, [size.width as f32, size.height as f32], &ui_manager.platform.context(), clipped_shapes).expect("Failed to render ui!");

        main_cmd_builder.end_render_pass().unwrap();
        let main_command_buffer = main_cmd_builder.build().unwrap();

        let future = self.previous_frame_end
                    .take()
                    .unwrap()
                    .join(acquire_future)
                    .then_execute(queue.clone(), main_command_buffer)
                    .unwrap()
                    // This commits the image to the swapchain, which presents it, but not immediately.
                    .then_swapchain_present(queue.clone(), self.swapchain.clone(), image_num)
                    .then_signal_fence_and_flush();

        match future {
            Ok(future) => {
                self.previous_frame_end = Some(future.boxed());
            }
            Err(vulkano::sync::FlushError::OutOfDate) => {
                trace!("Swapchain needs to be recreated!");
                self.recreate_swapchain = true;
                self.previous_frame_end = Some(vulkano::sync::now(device.clone()).boxed());
            }
            Err(e) => {
                error!("Failed to flush future: {:?}", e);
                self.previous_frame_end = Some(vulkano::sync::now(device.clone()).boxed());
            }
        }
    }
}
