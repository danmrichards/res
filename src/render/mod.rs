pub mod frame;
pub mod palette;

use crate::cartridge::Mirroring;
use crate::ppu::NESPPU;
use frame::Frame;

const SCREEN_SIZE: usize = 0x3C0;

// Returns the background palette for a specific column and row on screen.
fn bg_palette(ppu: &NESPPU, attribute_table: &[u8], col: usize, row: usize) -> [u8; 4] {
    // Each background tile is one byte in the nametable space in VRAM.
    let attr_table_idx = row / 4 * 8 + col / 4;
    let attr = attribute_table[attr_table_idx];

    // A byte in an attribute table controls which palettes are used for 4x4
    // tile blocks or 32x32 pixels.
    //
    // A byte is split into four 2-bit blocks and each block is assigning a
    // background palette for four neighboring tiles.
    //
    // Determine which tile we're dealing with and match the appropriate part
    // of the byte.
    //
    // Example:
    //
    //  0b11011000 => 0b|11|01|10|00 => 11,01,10,00
    let palette_idx = match (col % 4 / 2, row % 4 / 2) {
        (0, 0) => attr & 0b11,
        (1, 0) => (attr >> 2) & 0b11,
        (0, 1) => (attr >> 4) & 0b11,
        (1, 1) => (attr >> 6) & 0b11,
        (_, _) => panic!("invalid palette index"),
    };

    let start: usize = 1 + (palette_idx as usize) * 4;
    [
        ppu.palette_table[0],
        ppu.palette_table[start],
        ppu.palette_table[start + 1],
        ppu.palette_table[start + 2],
    ]
}

// Returns the sprite palette for a given index
fn sprite_palette(ppu: &NESPPU, idx: u8) -> [u8; 4] {
    let start = 0x11 + (idx * 4) as usize;
    [
        0,
        ppu.palette_table[start],
        ppu.palette_table[start + 1],
        ppu.palette_table[start + 2],
    ]
}

// Represents a
struct Rect {
    x1: usize,
    y1: usize,
    x2: usize,
    y2: usize,
}

impl Rect {
    // Returns an instantiated Rect.
    fn new(x1: usize, y1: usize, x2: usize, y2: usize) -> Self {
        Rect {
            x1: x1,
            y1: y1,
            x2: x2,
            y2: y2,
        }
    }
}

// Renders a given view port.
fn render_view_port(
    ppu: &NESPPU,
    frame: &mut Frame,
    name_table: &[u8],
    view_port: Rect,
    shift_x: isize,
    shift_y: isize,
) {
    let bank = ppu.ctrl.bgrnd_pattern_addr();

    let attribute_table = &name_table[SCREEN_SIZE..0x400];

    for i in 0..SCREEN_SIZE {
        let tile_column = i % 32;
        let tile_row = i / 32;
        let tile_idx = name_table[i] as u16;
        let tile =
            &ppu.chr_rom[(bank + tile_idx * 16) as usize..=(bank + tile_idx * 16 + 15) as usize];
        let palette = bg_palette(ppu, attribute_table, tile_column, tile_row);

        for y in 0..=7 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];

            for x in (0..=7).rev() {
                let value = (1 & lower) << 1 | (1 & upper);
                upper = upper >> 1;
                lower = lower >> 1;
                let rgb = match value {
                    0 => &palette::COLOUR_PALETTE[ppu.palette_table[0] as usize],
                    1 => &palette::COLOUR_PALETTE[palette[1] as usize],
                    2 => &palette::COLOUR_PALETTE[palette[2] as usize],
                    3 => &palette::COLOUR_PALETTE[palette[3] as usize],
                    _ => panic!("can't be"),
                };
                let pixel_x = tile_column * 8 + x;
                let pixel_y = tile_row * 8 + y;

                if pixel_x >= view_port.x1
                    && pixel_x < view_port.x2
                    && pixel_y >= view_port.y1
                    && pixel_y < view_port.y2
                {
                    frame.set_pixel(
                        (shift_x + pixel_x as isize) as usize,
                        (shift_y + pixel_y as isize) as usize,
                        rgb,
                    );
                }
            }
        }
    }
}

// Renders the background pixels.
fn render_bg(ppu: &NESPPU, frame: &mut Frame) {
    let scroll_x = (ppu.scroll.x) as usize;
    let scroll_y = (ppu.scroll.y) as usize;

    let (main_nametable, second_nametable) = match (&ppu.mirroring, ppu.ctrl.nametable_addr()) {
        (Mirroring::Vertical, 0x2000)
        | (Mirroring::Vertical, 0x2800)
        | (Mirroring::Horizontal, 0x2000)
        | (Mirroring::Horizontal, 0x2400) => (&ppu.vram[0..0x400], &ppu.vram[0x400..0x800]),
        (Mirroring::Vertical, 0x2400)
        | (Mirroring::Vertical, 0x2C00)
        | (Mirroring::Horizontal, 0x2800)
        | (Mirroring::Horizontal, 0x2C00) => (&ppu.vram[0x400..0x800], &ppu.vram[0..0x400]),
        (_, _) => {
            panic!("Not supported mirroring type {:?}", ppu.mirroring);
        }
    };

    render_view_port(
        ppu,
        frame,
        main_nametable,
        Rect::new(scroll_x, scroll_y, 256, 240),
        -(scroll_x as isize),
        -(scroll_y as isize),
    );
    if scroll_x > 0 {
        render_view_port(
            ppu,
            frame,
            second_nametable,
            Rect::new(0, 0, scroll_x, 240),
            (256 - scroll_x) as isize,
            0,
        );
    } else if scroll_y > 0 {
        render_view_port(
            ppu,
            frame,
            second_nametable,
            Rect::new(0, 0, 256, scroll_y),
            0,
            (240 - scroll_y) as isize,
        );
    }
}

// Renders sprites.
fn render_sprites(ppu: &NESPPU, frame: &mut Frame) {
    // Iterate the OAM in reverse to ensure sprite priority is maintained. In
    // the NES OAM, the sprite that occurs first in memory will overlap any that
    // follow.
    for i in (0..ppu.oam_data.len()).step_by(4).rev() {
        let tile_idx = ppu.oam_data[i + 1] as u16;
        let tile_x = ppu.oam_data[i + 3] as usize;
        let tile_y = ppu.oam_data[i] as usize;

        // TODO(dr) - i+2 is actually the sprite attributes in this format:
        //
        // 76543210
        // ||||||||
        // ||||||++- Palette (4 to 7) of sprite
        // |||+++--- Unimplemented (read 0)
        // ||+------ Priority (0: in front of background; 1: behind background)
        // |+------- Flip sprite horizontally
        // +-------- Flip sprite vertically
        //
        // Hence, implement sprite priority to fix clipping with background.

        // Sprite orientation.
        let flip_vertical = if ppu.oam_data[i + 2] >> 7 & 1 == 1 {
            true
        } else {
            false
        };
        let flip_horizontal = if ppu.oam_data[i + 2] >> 6 & 1 == 1 {
            true
        } else {
            false
        };
        let pallette_idx = ppu.oam_data[i + 2] & 0b11;
        let sprite_palette = sprite_palette(ppu, pallette_idx);

        let bank: u16 = ppu.ctrl.sprite_pattern_addr();

        let tile =
            &ppu.chr_rom[(bank + tile_idx * 16) as usize..=(bank + tile_idx * 16 + 15) as usize];

        // Draw the 8x8 sprite.
        for y in 0..=7 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];
            for x in (0..=7).rev() {
                let value = (1 & lower) << 1 | (1 & upper);
                upper = upper >> 1;
                lower = lower >> 1;
                let rgb = match value {
                    0 => continue,
                    1 => &palette::COLOUR_PALETTE[sprite_palette[1] as usize],
                    2 => &palette::COLOUR_PALETTE[sprite_palette[2] as usize],
                    3 => &palette::COLOUR_PALETTE[sprite_palette[3] as usize],
                    _ => panic!("invalid sprite index"),
                };
                match (flip_horizontal, flip_vertical) {
                    (false, false) => frame.set_pixel(tile_x + x, tile_y + y, rgb),
                    (true, false) => frame.set_pixel(tile_x + 7 - x, tile_y + y, rgb),
                    (false, true) => frame.set_pixel(tile_x + x, tile_y + 7 - y, rgb),
                    (true, true) => frame.set_pixel(tile_x + 7 - x, tile_y + 7 - y, rgb),
                }
            }
        }
    }
}

// Renders a screen of pixels to the frame based on PPU state.
pub fn render(ppu: &NESPPU, frame: &mut Frame) {
    render_bg(ppu, frame);

    render_sprites(ppu, frame);
}
