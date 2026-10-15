use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use glam::Vec3;
use wgpu::{CurrentSurfaceTexture, TextureViewDescriptor};
use wgpu::wgt::CommandEncoderDescriptor;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition, Position};
use winit::event::{DeviceEvent, DeviceId, ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowId};
use crate::engine::input::InputState;
use crate::engine::player::Player;
use crate::engine::worker_thread::ChunkWorker;
use crate::gfx::context::GPUContext;
use crate::gfx::pipeline::{VoxelRenderer, RENDER_DISTANCE};
use crate::world::{terrain};
use crate::world::chunk::CHUNK_SIZE;
use crate::world::world::World;

const WORLD_SIZE: i32 = 16;
const MAX_CHUNKS_PER_FRAME: usize = 2; // Must be kept small
const SEED: u32 = 1806;

#[derive(Default)]
pub struct App {
    gpu_context: Option<GPUContext>,
    renderer: Option<VoxelRenderer>,
    input: InputState,
    last_frame: Option<Instant>,
    world: Option<World>,
    player: Player,
    jump_was_down: bool,
    cursor_captured: bool,
    chunk_worker: Option<ChunkWorker>,
    requested_chunks: HashSet<(i32, i32)>,
}

impl App {
    fn handle_resize(&mut self, physical_size: winit::dpi::PhysicalSize<u32>) {
        let Some(ctx) = self.gpu_context.as_mut() else {
            return;
        };

        ctx.resize(physical_size);
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.resize(
                &ctx.device,
                &ctx.queue,
                ctx.config.width,
                ctx.config.height,
            );
        }
        ctx.window.request_redraw();
    }

    fn handle_redraw(&mut self) {
        let now = Instant::now();
        let dt = self
            .last_frame
            .replace(now)
            .map_or(0.0, |last| (now - last).as_secs_f32().min(0.1));

        self.handle_input(dt);

        let Some(ctx) = self.gpu_context.as_mut() else {
            return;
        };
        let Some(frame) = acquire_frame(ctx) else {
            return;
        };

        let view = frame
            .texture
            .create_view(&TextureViewDescriptor::default());
        let mut encoder = ctx.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        if let (Some(world), Some(renderer)) =
            (self.world.as_mut(), self.renderer.as_mut())
        {
            let worker = self.chunk_worker.as_ref().expect("chunk worker initialized");
            update_chunks(
                world,
                renderer,
                worker,
                &mut self.requested_chunks,
                self.player.pos,
                &ctx.device,
            );
        }

        let renderer = self
            .renderer
            .as_mut()
            .expect("renderer initialized with GPU context");
        renderer.render(&mut encoder, view);

        ctx.queue.submit(std::iter::once(encoder.finish()));
        ctx.queue.present(frame);
        ctx.window.request_redraw();
    }

    fn handle_keyboard_input(&mut self, key: PhysicalKey, state: ElementState) {
        let Some(ctx) = self.gpu_context.as_mut() else {
            return;
        };

        if state == ElementState::Pressed
            && key == PhysicalKey::Code(KeyCode::Escape)
            && self.cursor_captured
        {
            ctx.window
                .set_cursor_grab(CursorGrabMode::None)
                .expect("Failed to release cursor");
            ctx.window.set_cursor_visible(true);
            self.cursor_captured = false;
            self.input.take_mouse_delta();
        } else {
            self.input.key(key, state);
        }
    }

    fn handle_mouse_input(&mut self) {
        let Some(ctx) = self.gpu_context.as_mut() else {
            return;
        };

        ctx.window
            .set_cursor_grab(CursorGrabMode::Locked)
            .or_else(|_| ctx.window.set_cursor_grab(CursorGrabMode::Confined))
            .expect("Failed to capture cursor");
        ctx.window.set_cursor_visible(false);
        self.cursor_captured = true;
        self.input.take_mouse_delta();
    }

    fn setup_window(&mut self, event_loop: &ActiveEventLoop) -> Arc<Window> {
        let window_attributes = Window::default_attributes()
            .with_title("Voxel Engine")
            .with_inner_size(LogicalSize::new(1280, 720));

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        let size = window.inner_size();
        window
            .set_cursor_position(Position::Physical(PhysicalPosition::new(
                (size.width / 2) as i32,
                (size.height / 2) as i32,
            )))
            .expect("Failed to position cursor");

        window
            .set_cursor_grab(CursorGrabMode::Locked)
            .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined))
            .expect("Failed to capture cursor");
        window.set_cursor_visible(false);
        self.cursor_captured = true;

        window
    }

    fn handle_input(&mut self, dt: f32) {

        let Some(ctx) = self.gpu_context.as_mut() else { return };

        let (mouse_dx, mouse_dy) = self.input.take_mouse_delta();

        let forward_input = f32::from(self.input.forward) - f32::from(self.input.backward);
        let right_input = f32::from(self.input.right) - f32::from(self.input.left);
        let jump = self.input.up && !self.jump_was_down;
        self.jump_was_down = self.input.up;

        if let (Some(renderer), Some(world)) = (self.renderer.as_mut(), self.world.as_ref()) {
            let camera = renderer.camera_mut();
            let sensitivity = 0.5_f32.to_radians();
            camera.rotate(mouse_dx * sensitivity, -mouse_dy * sensitivity);

            let view = camera.view_direction();
            let forward = Vec3::new(view.x, 0.0, view.z).normalize_or_zero();
            let right = forward.cross(Vec3::Y);
            let movement = forward * forward_input + right * right_input;

            self.player.update(world, movement, jump, dt);
            camera.set_position(self.player.eye_pos());

            renderer.update_camera(&ctx.queue, ctx.config.width, ctx.config.height);
        }
    }

    fn generate_world(&mut self) -> World {
        let mut world = World::default();

        for chunk_x in -WORLD_SIZE..=WORLD_SIZE {
            for chunk_z in -WORLD_SIZE..=WORLD_SIZE {
                world.insert_chunk(
                    chunk_x,
                    chunk_z,
                    terrain::generate_chunk(chunk_x, chunk_z, SEED),
                );
                println!("Chunk ({chunk_x}, {chunk_z}) generated.")
            }
        }

        self.player.pos = Player::find_spawn_coords(&world)
            .expect("world contains no suitable spawn point");

        world
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu_context.is_some() {
            return;
        }

        let window = self.setup_window(event_loop);

        let context = pollster::block_on(GPUContext::new(window))
            .expect("WGPU Init failed...");

        let world = self.generate_world();

        self.chunk_worker = Some(ChunkWorker::new());

        self.renderer = Some(VoxelRenderer::new(
            &context.device,
            context.config.format,
            context.config.width,
            context.config.height,
            &context.queue,
            &world
        ));

        self.world = Some(world);

        context.window.request_redraw();
        self.gpu_context = Some(context);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                println!("Close button was pressed. closing...");
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => self.handle_resize(physical_size),
            WindowEvent::RedrawRequested => self.handle_redraw(),
            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_keyboard_input(event.physical_key, event.state);
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } if !self.cursor_captured => self.handle_mouse_input(),
            _ => (),
        }
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _device_id: DeviceId, event: DeviceEvent) {
        if self.cursor_captured {
            if let DeviceEvent::MouseMotion { delta } = event {
                self.input.add_mouse_delta(delta.0, delta.1);
            }
        }
    }
}

fn acquire_frame(ctx: &mut GPUContext) -> Option<wgpu::SurfaceTexture> {
    match ctx.surface.get_current_texture() {
        CurrentSurfaceTexture::Success(frame) => Some(frame),
        CurrentSurfaceTexture::Suboptimal(frame) => {
            ctx.surface.configure(&ctx.device, &ctx.config);
            Some(frame)
        }
        CurrentSurfaceTexture::Lost | CurrentSurfaceTexture::Outdated => {
            ctx.surface.configure(&ctx.device, &ctx.config);
            ctx.window.request_redraw();
            None
        }
        CurrentSurfaceTexture::Timeout | CurrentSurfaceTexture::Occluded => None,
        CurrentSurfaceTexture::Validation => {
            eprintln!("WGPU validation error.");
            None
        }
    }
}

fn update_chunks(
    world: &mut World,
    renderer: &mut VoxelRenderer,
    worker: &ChunkWorker,
    requested_chunks: &mut HashSet<(i32, i32)>,
    player_pos: Vec3,
    device: &wgpu::Device,
) {
    let radius = RENDER_DISTANCE as i32;
    let radius_squared = i64::from(radius).pow(2);
    let chunk_size = CHUNK_SIZE as i32;
    let center_x = (player_pos.x.floor() as i32).div_euclid(chunk_size);
    let center_z = (player_pos.z.floor() as i32).div_euclid(chunk_size);

    world.unload_chunks_outside(center_x, center_z, radius);

    for _ in 0..=MAX_CHUNKS_PER_FRAME {
        let Some(((chunk_x, chunk_z), chunk)) = worker.try_result() else {
            break;
        };

        requested_chunks.remove(&(chunk_x, chunk_z));

        let dx = i64::from(chunk_x) - i64::from(center_x);
        let dz = i64::from(chunk_z) - i64::from(center_z);
        if dx * dx + dz * dz <= radius_squared {
            world.insert_chunk(chunk_x, chunk_z, chunk);
        }
    }

    let mut missing = Vec::new();
    for dx in -radius..=radius {
        for dz in -radius..=radius {
            let distance_squared = i64::from(dx).pow(2) + i64::from(dz).pow(2);
            if distance_squared > radius_squared {
                continue;
            }

            let coords = (
                center_x.saturating_add(dx),
                center_z.saturating_add(dz),
            );

            if !world.chunks.contains_key(&coords)
                && !requested_chunks.contains(&coords)
            {
                missing.push((distance_squared, coords));
            }
        }
    }

    missing.sort_unstable_by_key(|(distance, _)| *distance);
    for (_, coords) in missing {
        if requested_chunks.insert(coords) {
            worker.request(coords, SEED);
        }
    }

    renderer.sync_chunks(device, world);
}
