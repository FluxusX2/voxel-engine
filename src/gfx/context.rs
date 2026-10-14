use std::default::Default;
use std::sync::Arc;
use wgpu::{Device, DeviceDescriptor, Features, Limits, PresentMode, Queue, RequestAdapterOptions, Surface, SurfaceConfiguration, TextureUsages};
use wgpu::PowerPreference::HighPerformance;
use winit::dpi::PhysicalSize;
use winit::window::Window;

pub struct GPUContext {
    pub window: Arc<Window>,
    pub surface: Surface<'static>, // Used to represent image.
    pub device: Device, // Connection to GPU.
    pub queue: Queue, // Queue of commands for GPU.
    pub config: SurfaceConfiguration, // Swapchain configuration.
    pub size: PhysicalSize<u32>, // Windowsize in pixel.
}

impl GPUContext {
    pub async fn new(window: Arc<Window>) -> Result<Self, Box<dyn std::error::Error>> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });

        let surface = instance.create_surface(window.clone())?;

        let adapter = instance.request_adapter(
            &RequestAdapterOptions {
                power_preference: HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
                apply_limit_buckets: false,
            }
        ).await?;

        let (device, queue) = adapter.request_device(
            &DeviceDescriptor {
                label:Some("Primary Device"),
                required_features: Features::empty(),
                required_limits: Limits::default(),
                experimental_features: Default::default(),
                memory_hints: Default::default(),
                trace: Default::default(),
            },
        ).await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            color_space: Default::default(),
            width: size.width,
            height: size.height,
            present_mode: PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
        })
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }
}