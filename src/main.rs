use anyhow::Context;
use futures_executor::block_on;
use std::time::Instant;
use wgpu_glyph::ab_glyph::FontArc;
use wgpu_glyph::{GlyphBrushBuilder, Section, Text};
use winit::event::{Event, WindowEvent};
use winit::event_loop::ControlFlow;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("FlipFlop")
        .build(&event_loop)?;

    let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
    let surface = unsafe { instance.create_surface(&window) };
    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: Default::default(),
        compatible_surface: Some(&surface),
    }))
    .context("Failed to find a suitable adapter")?;
    let (device, queue) = block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: None,
            features: Default::default(),
            limits: Default::default(),
        },
        None,
    ))
    .context("Failed to open device")?;

    const RENDER_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
    fn create_swap_chain(
        device: &wgpu::Device,
        surface: &wgpu::Surface,
        window: &winit::window::Window,
    ) -> wgpu::SwapChain {
        device.create_swap_chain(
            &surface,
            &wgpu::SwapChainDescriptor {
                usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
                format: RENDER_FORMAT,
                width: window.inner_size().width,
                height: window.inner_size().height,
                present_mode: wgpu::PresentMode::Fifo,
            },
        )
    }
    let mut swap_chain = create_swap_chain(&device, &surface, &window);

    let fira_sans = FontArc::try_from_slice(include_bytes!("FiraSans-Regular.ttf"))?;
    let mut glyph_brush = GlyphBrushBuilder::using_font(fira_sans).build(&device, RENDER_FORMAT);
    let mut staging_belt = wgpu::util::StagingBelt::new(1024);
    let mut local_pool = futures_executor::LocalPool::new();
    let local_spawner = local_pool.spawner();

    let mut last_render: Option<Instant> = None;
    let mut fps = 0.0;

    event_loop.run(move |event, target, control_flow| match event {
        Event::RedrawRequested(..) => {
            let this_render = Instant::now();
            if let Some(last_render) = last_render {
                fps = 1.0 / (this_render - last_render).as_secs_f32();
            }
            last_render = Some(this_render);

            let frame = loop {
                match swap_chain.get_current_frame() {
                    Ok(frame) => break frame.output,
                    Err(wgpu::SwapChainError::Lost) | Err(wgpu::SwapChainError::Outdated) => {
                        swap_chain = create_swap_chain(&device, &surface, &window);
                    }
                    Err(wgpu::SwapChainError::Timeout) => {
                        return;
                    }
                    Err(err) => {
                        eprintln!("{:?}", err);
                        *control_flow = ControlFlow::Exit;
                        return;
                    }
                }
            };
            let mut encoder = device.create_command_encoder(&Default::default());

            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                ..Default::default()
            });

            let size = window.inner_size();
            glyph_brush.queue(Section {
                screen_position: (0.0, 0.0),
                bounds: (size.width as f32, size.height as f32),
                text: vec![Text::new(&format!("FPS: {:.1}", fps))
                    .with_color([1.0, 1.0, 1.0, 1.0])
                    .with_scale(24.0)],
                ..Default::default()
            });
            glyph_brush
                .draw_queued(
                    &device,
                    &mut staging_belt,
                    &mut encoder,
                    &frame.view,
                    size.width,
                    size.height,
                )
                .expect("Text draw error");
            staging_belt.finish();

            queue.submit(std::iter::once(encoder.finish()));

            use futures_util::task::SpawnExt;
            local_spawner
                .spawn(staging_belt.recall())
                .expect("Recall error");
            local_pool.run_until_stalled();
        }
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *control_flow = ControlFlow::Exit;
        }
        Event::MainEventsCleared => {
            window.request_redraw();
        }
        _ => {}
    });
}
