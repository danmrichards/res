extern crate core;

pub mod bus;
pub mod cartridge;
pub mod cpu;
pub mod instructions;
pub mod joypad;
pub mod ppu;
pub mod render;
pub mod tile;
pub mod trace;

use bus::Bus;
use cartridge::Rom;
use clap::Parser;
use cpu::CPU;
use ppu::NESPPU;
use render::frame::Frame;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Parser, Debug)]
#[command(
    version = "0.1.0",
    about = "A NES emulator implemented in Rust",
    long_about = "A NES emulator implemented in Rust\n\nControls:\n\nUp arrow\t= D-pad up\nDown arrow\t= D-pad down\nLeft arrow\t= D-pad left\nRight arrow\t= D-pad right\nSpace bar\t= Select\nReturn\t\t= Start\nA\t\t= A\nS\t\t= B"
)]
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
    canvas
        .set_scale(args.pixel_scale, args.pixel_scale)
        .unwrap();

    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_target(PixelFormatEnum::RGB24, args.window_w, args.window_h)
        .unwrap();

    let bytes: Vec<u8> = std::fs::read(args.rom).unwrap();
    let rom = Rom::new(&bytes).unwrap();

    let mut frame = Frame::new();

    // Initialise joypad.
    let mut key_map = HashMap::new();
    key_map.insert(Keycode::Up, joypad::JOYPAD_UP);
    key_map.insert(Keycode::Down, joypad::JOYPAD_DOWN);
    key_map.insert(Keycode::Left, joypad::JOYPAD_LEFT);
    key_map.insert(Keycode::Right, joypad::JOYPAD_RIGHT);
    key_map.insert(Keycode::Space, joypad::JOYPAD_SELECT);
    key_map.insert(Keycode::Return, joypad::JOYPAD_START);
    key_map.insert(Keycode::A, joypad::JOYPAD_BUTTON_A);
    key_map.insert(Keycode::S, joypad::JOYPAD_BUTTON_B);

    // Forcing a 60FPS frame-time.
    let frame_duration = Duration::new(0, 1000000000 / 60);
    let mut timestamp = Instant::now();

    let bus = Bus::new(rom, move |ppu: &NESPPU, joypad: &mut joypad::Joypad| {
        render::render(ppu, &mut frame);
        texture
            .update(None, &frame.data, window_w as usize)
            .unwrap();

        canvas.copy(&texture, None, None).unwrap();

        canvas.present();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => std::process::exit(0),
                Event::KeyDown { keycode, .. } => {
                    if let Some(key) = key_map.get(&keycode.unwrap_or(Keycode::Ampersand)) {
                        joypad.set_button_pressed_status(*key, true);
                    }
                }
                Event::KeyUp { keycode, .. } => {
                    if let Some(key) = key_map.get(&keycode.unwrap_or(Keycode::Ampersand)) {
                        joypad.set_button_pressed_status(*key, false);
                    }
                }
                _ => { /* do nothing */ }
            }
        }

        // Pause for the next frame.
        let now = Instant::now();
        let sleep_dur = frame_duration
            .checked_sub(now.saturating_duration_since(timestamp))
            .unwrap_or(Duration::new(0, 0));
        ::std::thread::sleep(sleep_dur);

        timestamp = now;
    });

    let mut cpu = CPU::new(bus);

    cpu.reset();
    cpu.run();
}
