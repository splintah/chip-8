extern crate chip_8;
extern crate glutin;

mod graphics;

use chip_8::{Processor, HEIGHT, WIDTH};
use glutin::GlContext;
use graphics::Graphics;
use std::fs::File;
use std::io::prelude::*;

fn main() -> std::io::Result<()> {
    let mut processor = if let Some(filename) = std::env::args().skip(1).next() {
        let mut file = File::open(filename)?;
        let mut contents: Vec<u8> = Vec::new();
        file.read_to_end(&mut contents)?;
        Processor::with_file(&contents)
    } else {
        eprintln!("Error: no file found.");
        println!("Usage: chip-8 <file>");
        std::process::exit(1);
    };

    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new()
        .with_title("CHIP-8")
        .with_dimensions(glutin::dpi::LogicalSize::new(640.0, 340.0))
        .with_resizable(false);

    let context = glutin::ContextBuilder::new().with_vsync(true);
    let gl_window = glutin::GlWindow::new(window, context, &events_loop).unwrap();

    unsafe {
        gl_window.make_current().unwrap();
    }

    let mut graphics = Graphics::new();
    graphics.init(&gl_window).unwrap();

    let mut closed = false;
    while !closed {
        use glutin::{ElementState, Event, VirtualKeyCode::*, WindowEvent};
        events_loop.poll_events(|e| {
            if let Event::WindowEvent { event, .. } = e {
                match event {
                    WindowEvent::CloseRequested => closed = true,
                    WindowEvent::KeyboardInput { input, .. } => {
                        if let Some(keycode) = input.virtual_keycode {
                            let pressed = input.state == ElementState::Pressed;
                            match keycode {
                                Key1 => processor.set_key(0x1, pressed),
                                Key2 => processor.set_key(0x2, pressed),
                                Key3 => processor.set_key(0x3, pressed),
                                Key4 => processor.set_key(0xC, pressed),
                                Q => processor.set_key(0x4, pressed),
                                W => processor.set_key(0x5, pressed),
                                E => processor.set_key(0x6, pressed),
                                R => processor.set_key(0xD, pressed),
                                A => processor.set_key(0x7, pressed),
                                S => processor.set_key(0x8, pressed),
                                D => processor.set_key(0x9, pressed),
                                F => processor.set_key(0xE, pressed),
                                Z => processor.set_key(0xA, pressed),
                                X => processor.set_key(0x0, pressed),
                                C => processor.set_key(0xB, pressed),
                                V => processor.set_key(0xF, pressed),
                                Escape => closed = true,
                                // Question mark.
                                Slash if input.modifiers.shift => println!(
                                    "index = 0x{:X}, opcode = 0x{:04X}",
                                    processor.program_counter,
                                    processor.opcode()
                                ),
                                _ => (),
                            }
                        }
                    }
                    _ => (),
                }
            }
        });

        processor.run_cycle().unwrap();

        if processor.draw {
            graphics.clear_colour(0.0, 0.0, 0.0, 1.0);
            for y in 0..HEIGHT {
                for x in 0..WIDTH {
                    if processor.display[x + y * WIDTH] {
                        graphics.draw_square_at(x, y);
                    }
                }
            }
            gl_window.swap_buffers().unwrap();
            processor.draw = false;
        }
    }

    Ok(())
}
