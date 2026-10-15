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
use crate::gfx::context::GPUContext;
use crate::gfx::pipeline::{VoxelRenderer, RENDER_DISTANCE};
use crate::world::{terrain};
use crate::world::chunk::CHUNK_SIZE;
use crate::world::world::World;

const WORLD_SIZE: i32 = 2;

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
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu_context.is_some() {
            return;
        }

        let window = setup_window(self, event_loop);

        let context = pollster::block_on(GPUContext::new(window))
            .expect("WGPU Init failed...");

        let world = generate_world(self);

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
            },
            WindowEvent::Resized(physical_size) => {
                let Some(ctx) = self.gpu_context.as_mut() else { return };
                ctx.resize(physical_size);
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(&ctx.device, &ctx.queue, ctx.config.width, ctx.config.height);
                }
                ctx.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {

                let now = Instant::now();
                let dt = self.last_frame
                    .replace(now)
                    .map_or(0.0, |last| (now - last).as_secs_f32().min(0.1));

                handle_input(self, dt);

                let Some(ctx) = self.gpu_context.as_mut() else { return };
                let frame = match ctx.surface.get_current_texture() {
                    CurrentSurfaceTexture::Success(frame) => frame,
                    CurrentSurfaceTexture::Suboptimal(frame) => {
                        ctx.surface.configure(&ctx.device, &ctx.config);
                        frame
                    }
                    CurrentSurfaceTexture::Lost | CurrentSurfaceTexture::Outdated => {
                        ctx.surface.configure(&ctx.device, &ctx.config);
                        ctx.window.request_redraw();
                        return;
                    },
                    CurrentSurfaceTexture::Timeout | CurrentSurfaceTexture::Occluded => {
                        return;
                    }
                    CurrentSurfaceTexture::Validation => {
                        eprintln!("WGPU validation error.");
                        return;
                    }
                };

                let view = frame.texture.create_view(&TextureViewDescriptor::default());
                let mut encoder = ctx.device.create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

                if let (Some(world), Some(renderer)) = (self.world.as_mut(), self.renderer.as_mut()) {
                    let chunk_size = CHUNK_SIZE as i32;
                    let center_x = (self.player.pos.x.floor() as i32).div_euclid(chunk_size);
                    let center_z = (self.player.pos.z.floor() as i32).div_euclid(chunk_size);

                    world.ensure_chunks_around(center_x, center_z, RENDER_DISTANCE as i32, 42);
                    renderer.sync_chunks(&ctx.device, world);
                }

                let renderer = self
                    .renderer
                    .as_mut()
                    .expect("renderer initialized with GPU context");
                renderer.render(&mut encoder, view);

                ctx.queue.submit(std::iter::once(encoder.finish()));
                ctx.queue.present(frame);
                ctx.window.request_redraw();
            },
            WindowEvent::KeyboardInput { event, .. } => {
                let Some(ctx) = self.gpu_context.as_mut() else { return };

                if event.state == ElementState::Pressed
                    && event.physical_key == PhysicalKey::Code(KeyCode::Escape)
                    && self.cursor_captured
                {
                    ctx.window
                        .set_cursor_grab(CursorGrabMode::None)
                        .expect("Failed to release cursor");
                    ctx.window.set_cursor_visible(true);
                    self.cursor_captured = false;
                    self.input.take_mouse_delta();
                } else {
                    self.input.key(event.physical_key, event.state);
                }
            },
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } if !self.cursor_captured => {
                let Some(ctx) = self.gpu_context.as_mut() else { return };

                ctx.window
                    .set_cursor_grab(CursorGrabMode::Locked)
                    .or_else(|_| ctx.window.set_cursor_grab(CursorGrabMode::Confined))
                    .expect("Failed to capture cursor");
                ctx.window.set_cursor_visible(false);
                self.cursor_captured = true;
                self.input.take_mouse_delta();
            },
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

fn setup_window(app: &mut App, event_loop: &ActiveEventLoop) -> Arc<Window> {
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
    app.cursor_captured = true;

    window
}

fn handle_input(app: &mut App, dt: f32) {

    let Some(ctx) = app.gpu_context.as_mut() else { return };

    let (mouse_dx, mouse_dy) = app.input.take_mouse_delta();

    let forward_input = f32::from(app.input.forward) - f32::from(app.input.backward);
    let right_input = f32::from(app.input.right) - f32::from(app.input.left);
    let jump = app.input.up && !app.jump_was_down;
    app.jump_was_down = app.input.up;

    if let (Some(renderer), Some(world)) = (app.renderer.as_mut(), app.world.as_ref()) {
        let camera = renderer.camera_mut();
        let sensitivity = 0.5_f32.to_radians();
        camera.rotate(mouse_dx * sensitivity, -mouse_dy * sensitivity);

        let view = camera.view_direction();
        let forward = Vec3::new(view.x, 0.0, view.z).normalize_or_zero();
        let right = forward.cross(Vec3::Y);
        let movement = forward * forward_input + right * right_input;

        app.player.update(world, movement, jump, dt);
        camera.set_position(app.player.eye_pos());

        renderer.update_camera(&ctx.queue, ctx.config.width, ctx.config.height);
    }
}

fn generate_world(app: &mut App, ) -> World {
    let mut world = World::default();

    for chunk_x in -WORLD_SIZE..=WORLD_SIZE {
        for chunk_z in -WORLD_SIZE..=WORLD_SIZE {
            world.insert_chunk(
                chunk_x,
                chunk_z,
                terrain::generate_chunk(chunk_x, chunk_z, 42),
            );
            println!("Chunk ({chunk_x}, {chunk_z}) generated.")
        }
    }

    app.player.pos = Player::find_spawn_coords(&world)
        .expect("world contains no suitable spawn point");

    world
}