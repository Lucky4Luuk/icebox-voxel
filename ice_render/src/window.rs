use std::sync::Arc;

use winit::window::Window;
use winit::window::WindowBuilder;
use winit::event_loop::EventLoop;

use vulkano::image::SwapchainImage;
use vulkano::image::ImageUsage;

use vulkano::device::{
    Device,
    Queue,
    QueuesIter,
    DeviceExtensions,
    Features,
};

use vulkano::swapchain::{
    Swapchain,
    Surface,
    SurfaceTransform,
    PresentMode,
    ColorSpace,
    FullscreenExclusive,
};

use vulkano::instance::PhysicalDevice;
use vulkano::instance::Instance;

use vulkano_win::VkSurfaceBuild;

pub fn create_window(width: u32, height: u32, title: &str) -> (Arc<Instance>, EventLoop<()>, Arc<Surface<Window>>) {
    let size = winit::dpi::PhysicalSize {
        width: width,
        height: height,
    };

    let instance = {
        let extensions = vulkano_win::required_extensions();
        Instance::new(None, &extensions, None).expect("Failed to create Vulkan instance")
    };

    let event_loop = EventLoop::new();
    let surface = WindowBuilder::new()
        .with_title(title)
        .with_inner_size(size)
        .build_vk_surface(&event_loop, instance.clone()).unwrap();

    (instance, event_loop, surface)
}

//TODO: Mark a gpu as chosen, so this function just has to grab it from the list, instead of doing all the work everytime
pub fn get_physical_device(instance: &Arc<Instance>) -> PhysicalDevice {
    PhysicalDevice::enumerate(&instance).next().expect("No device available")
}

pub fn vulkan_setup(instance: &Arc<Instance>, physical: &PhysicalDevice) -> (Arc<Device>, QueuesIter) {
    let device_ext = DeviceExtensions {
        khr_swapchain: true,
        .. DeviceExtensions::none()
    };

    let queue_family = physical.queue_families().find(|&q| q.supports_graphics()).expect("Couldn't find a graphical queue family!");
    Device::new(*physical, &Features::none(), &device_ext, [(queue_family, 0.5)].iter().cloned()).expect("Failed to create device!")
}

pub fn create_swapchain(device: &Arc<Device>, queue: &Arc<Queue>, physical: &PhysicalDevice, surface: &Arc<Surface<Window>>) -> (Arc<Swapchain<Window>>, Vec<Arc<SwapchainImage<Window>>>) {
    let caps = surface.capabilities(*physical)
        .expect("Failed to get surface capabilities");

    let dimensions = caps.current_extent.unwrap_or([1280, 1024]);
    let alpha = caps.supported_composite_alpha.iter().next().unwrap();
    let format = caps.supported_formats[0].0;

    Swapchain::new(device.clone(), surface.clone(),
        caps.min_image_count, format, dimensions, 1, ImageUsage::color_attachment(), queue,
        SurfaceTransform::Identity, alpha, PresentMode::Fifo, FullscreenExclusive::Default,
    	true, ColorSpace::SrgbNonLinear)
        .expect("Failed to create swapchain")
}
