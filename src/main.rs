use anyhow::Context;
use futures_executor::block_on;
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

    fn create_swap_chain(
        device: &wgpu::Device,
        surface: &wgpu::Surface,
        window: &winit::window::Window,
    ) -> wgpu::SwapChain {
        device.create_swap_chain(
            &surface,
            &wgpu::SwapChainDescriptor {
                usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                width: window.inner_size().width,
                height: window.inner_size().height,
                present_mode: wgpu::PresentMode::Fifo,
            },
        )
    }
    let mut swap_chain = create_swap_chain(&device, &surface, &window);

    event_loop.run(move |event, target, control_flow| match event {
        Event::RedrawRequested(..) => {
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

            queue.submit(std::iter::once(encoder.finish()))
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
