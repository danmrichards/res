extern crate core;

pub mod bus;
pub mod cartridge;
pub mod cpu;
pub mod instructions;
pub mod ppu;
pub mod render;
pub mod tile;

use bus::Bus;
use cartridge::Rom;
use clap::Parser;
use cpu::CPU;
use ppu::NESPPU;
use render::frame::Frame;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;

#[derive(Parser, Debug)]
#[command(version = "0.1.0", about = "A NES emulator implemented in Rust")]
struct Args {
    /// Width of emulator window.
    #[arg(short = 'x', long, default_value_t = 256)]
    window_w: u32,

    /// Height of emulator window.
    #[arg(short = 'y', long, default_value_t = 240)]
    window_h: u32,

    /// Pixel scaling factor.
    #[arg(short, long, default_value_t = 3.0)]
    pixel_scale: f32,

    /// path/to/rom
    #[arg(short, long)]
    rom: String,
}

impl Args {
    fn scaled_window_w(&self) -> u32 {
        (self.window_w as f32 * self.pixel_scale) as u32
    }

    fn scaled_window_h(&self) -> u32 {
        (self.window_h as f32 * self.pixel_scale) as u32
    }
}


fn main() {
    let args = Args::parse();

    let window_w = args.scaled_window_w();

    // Initialise SDL.
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("NESOxide", window_w, args.scaled_window_h())
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    canvas.set_scale(args.pixel_scale, args.pixel_scale).unwrap();

    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_target(
            PixelFormatEnum::RGB24, args.window_w, args.window_h,
        )
        .unwrap();

    let bytes: Vec<u8> = std::fs::read(args.rom).unwrap();
    let rom = Rom::new(&bytes).unwrap();

    let mut frame = Frame::new();

    let bus = Bus::new(rom, move |ppu: &NESPPU| {
        render::render(ppu, &mut frame);
        texture.update(None, &frame.data, window_w as usize).unwrap();

        canvas.copy(&texture, None, None).unwrap();

        canvas.present();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => std::process::exit(0),
                _ => { /* do nothing */ }
            }
        }
    });

    let mut cpu = CPU::new(bus);

    cpu.reset();
    cpu.run();
}
