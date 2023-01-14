extern crate core;

pub mod bus;
pub mod cartridge;
pub mod cpu;
pub mod instructions;
pub mod ppu;
pub mod render;
pub mod tile;
pub mod trace;

use bus::Bus;
use cartridge::Rom;
use cpu::Memory;
use cpu::CPU;
use rand::Rng;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;
use sdl2::EventPump;
use tile::show_tile;

const RNG: u16 = 0xFE;
const LAST_PRESSED: u16 = 0xFF;

fn color(byte: u8) -> Color {
    match byte {
        0 => Color::BLACK,
        1 => Color::WHITE,
        2 | 9 => Color::GREY,
        3 | 10 => Color::RED,
        4 | 11 => Color::GREEN,
        5 | 12 => Color::BLUE,
        6 | 13 => Color::MAGENTA,
        7 | 14 => Color::YELLOW,
        _ => Color::CYAN,
    }
}

fn handle_user_input(cpu: &mut CPU, event_pump: &mut EventPump) {
    for event in event_pump.poll_iter() {
        match event {
            Event::Quit { .. }
            | Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } => std::process::exit(0),
            Event::KeyDown {
                keycode: Some(Keycode::W),
                ..
            } => {
                cpu.mem_write_byte(LAST_PRESSED, 0x77);
            }
            Event::KeyDown {
                keycode: Some(Keycode::S),
                ..
            } => {
                cpu.mem_write_byte(LAST_PRESSED, 0x73);
            }
            Event::KeyDown {
                keycode: Some(Keycode::A),
                ..
            } => {
                cpu.mem_write_byte(LAST_PRESSED, 0x61);
            }
            Event::KeyDown {
                keycode: Some(Keycode::D),
                ..
            } => {
                cpu.mem_write_byte(LAST_PRESSED, 0x64);
            }
            _ => {}
        }
    }
}

fn read_screen_state(cpu: &mut CPU, frame: &mut [u8; 32 * 3 * 32]) -> bool {
    let mut frame_idx = 0;
    let mut update = false;
    for i in 0x0200..0x600 {
        let color_idx = cpu.mem_read_byte(i as u16);
        let (b1, b2, b3) = color(color_idx).rgb();
        if frame[frame_idx] != b1 || frame[frame_idx + 1] != b2 || frame[frame_idx + 2] != b3 {
            frame[frame_idx] = b1;
            frame[frame_idx + 1] = b2;
            frame[frame_idx + 2] = b3;
            update = true;
        }
        frame_idx += 3;
    }
    update
}

fn main() {
    // Initialise SDL.
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("Tile Viewer", 640, 640)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    canvas.set_scale(20.0, 20.0).unwrap();

    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_target(PixelFormatEnum::RGB24, 32, 32)
        .unwrap();

    let bytes: Vec<u8> = std::fs::read("Alter_Ego.nes").unwrap();
    let rom = Rom::new(&bytes).unwrap();

    let tile_frame = show_tile(&rom.chr, 1,0);

    texture.update(None, &tile_frame.data, 256 * 3).unwrap();
    canvas.copy(&texture, None, None).unwrap();
    canvas.present();

    loop {
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
     }

    // let bus = Bus::new(rom);
    // let mut cpu = CPU::new(bus);
    // cpu.reset();

    // let mut screen_state = [0 as u8; 32 * 3 * 32];
    // let mut rng = rand::thread_rng();

    // cpu.run_with_callback(move |cpu| {
    //     handle_user_input(cpu, &mut event_pump);
    //     cpu.mem_write_byte(RNG, rng.gen_range(1..16));

    //     if read_screen_state(cpu, &mut screen_state) {
    //         texture.update(None, &screen_state, 32 * 3).unwrap();
    //         canvas.copy(&texture, None, None).unwrap();
    //         canvas.present();
    //     }

    //     ::std::thread::sleep(std::time::Duration::new(0, 70_000));
    // });
}
