pub mod registers;

mod frame;
mod sprite;
mod palette;

use crate::cartridge::Mirroring;
use registers::addr::Addr;
use registers::control::Control;
use registers::mask::Mask;
use registers::scroll::Scroll;
use registers::status::Status;

use self::frame::Frame;
use self::sprite::Sprite;

const SCREEN_SIZE: usize = 0x3C0;


/// Represents a rectangle viewport.
struct Rect {
    x1: usize,
    y1: usize,
    x2: usize,
    y2: usize,
}

impl Rect {
    /// Returns an instantiated Rect.
    fn new(x1: usize, y1: usize, x2: usize, y2: usize) -> Self {
        Rect { x1, y1, x2, y2 }
    }
}


/// Represents the NES PPU.
pub struct NESPPU<'rcall> {
    /// Character (visuals) ROM.
    pub chr_rom: Vec<u8>,

    /// Internal reference to colour palettes.
    pub palette_table: [u8; 32],

    /// Video RAM.
    pub vram: [u8; 2048],

    /// Object attribute memory (sprites).
    pub oam_addr: u8,
    pub oam_data: [u8; 256],

    pub mirroring: Mirroring,

    /// Registers.
    pub addr: Addr,
    pub ctrl: Control,
    pub mask: Mask,
    pub scroll: Scroll,
    pub status: Status,

    /// Is the NMI interrupt set?
    pub nmi_interrupt: Option<bool>,

    /// Buffer for data read from previous request.
    buf: u8,

    /// Current picture scan line
    scanline: u16,

    /// Number of cycles.
    cycles: usize,

    /// Number of frames rendered by the PPU.
    frame_count: u128,

    /// Current frame.
    frame: Frame,

    /// Callback to render frame.
    render_callback: Box<dyn FnMut(&[u8]) + 'rcall>,
}

pub trait PPU {
    fn write_addr(&mut self, value: u8);
    fn write_ctrl(&mut self, value: u8);
    fn write_mask(&mut self, value: u8);
    fn write_scroll(&mut self, value: u8);
    fn write_data(&mut self, value: u8);
    fn write_oam_addr(&mut self, value: u8);
    fn write_oam_data(&mut self, value: u8);
    fn write_oam_dma(&mut self, value: &[u8; 256]);
    fn read_data(&mut self) -> u8;
    fn read_status(&mut self) -> u8;
    fn read_oam_data(&self) -> u8;
    fn read_frame_count(&self) -> u128;
}

impl<'a> NESPPU<'a> {
    /// Returns an instantiated PPU.
    pub fn new<'rcall, F>(
        chr_rom: Vec<u8>,
        mirroring: Mirroring,
        render_callback: F,
    ) -> NESPPU<'rcall>
    where
        F: FnMut(&[u8]) + 'rcall,
    {
        NESPPU {
            chr_rom: chr_rom,
            palette_table: [0; 32],
            vram: [0; 2048],
            oam_addr: 0,
            oam_data: [0; 64 * 4],
            mirroring: mirroring,
            buf: 0,
            addr: Addr::new(),
            ctrl: Control::new(),
            mask: Mask::new(),
            scroll: Scroll::new(),
            status: Status::new(),
            scanline: 0,
            cycles: 0,
            nmi_interrupt: None,
            frame_count: 0,
            frame: Frame::new(),
            render_callback: Box::from(render_callback),
        }
    }

    /// Returns an instatiated PPU with an empty ROM loaded.
    pub fn new_empty_rom() -> Self {
        NESPPU::new(vec![0; 2048], Mirroring::Horizontal, |_| {})
    }

    /// Increment the VRAM address based on the control register status.
    fn increment_vram_addr(&mut self) {
        self.addr.increment(self.ctrl.vram_addr_increment());
    }

    /// Horizontal:
    ///   [ A ] [ a ]
    ///   [ B ] [ b ]
    ///
    /// Vertical:
    ///   [ A ] [ B ]
    ///   [ a ] [ b ]
    fn mirror_vram_addr(&self, addr: u16) -> u16 {
        // Mirror down 0x3000-0x3EFF to 0x2000 - 0x2EFF
        let mirrored_vram = addr & 0b1011111_1111111;

        // To VRAM vector.
        let vram_index = mirrored_vram - 0x2000;
        let name_table = vram_index / 0x400;

        match (&self.mirroring, name_table) {
            (Mirroring::Vertical, 2) | (Mirroring::Vertical, 3) => vram_index - 0x800,
            (Mirroring::Horizontal, 2) => vram_index - 0x400,
            (Mirroring::Horizontal, 1) => vram_index - 0x400,
            (Mirroring::Horizontal, 3) => vram_index - 0x800,
            _ => vram_index,
        }
    }

    /// Returns true if a frame has been completed, while incrementing the cycle
    /// count and scanline as appropriate.
    pub fn clock(&mut self, cycles: u8) -> bool {
        self.cycles += cycles as usize;

        // Each scanline lasts for 341 PPU clock cycles.
        if self.cycles < 341 {
            return false;
        }

        if self.sprite_zero_hit(self.cycles) {
            self.status.set_sprite_zero_hit(true);
        }

        self.cycles -= 341;
        self.scanline += 1;

        self.render_bg();
        self.render_sprites();

        // VBLANK is triggered at scanline 241.
        if self.scanline == 241 {
            self.status.set_vblank_status(true);
            self.status.set_sprite_zero_hit(false);

            // Set the interrupt if the control register allows it.
            if self.ctrl.vblank_nmi() {
                self.nmi_interrupt = Some(true);
            }

            self.frame_count = self.frame_count.wrapping_add(1);

            (self.render_callback)(self.frame.pixels());
        } else if self.scanline >= 262 {
            // There are 262 scanlines per frame.
            self.scanline = 0;
            self.nmi_interrupt = None;
            self.status.set_sprite_zero_hit(false);
            self.status.reset_vblank_status();
            return true;
        }

        return false;
    }

    /// Returns true when a nonzero pixel of sprite 0 overlaps a nonzero
    /// background pixel.
    fn sprite_zero_hit(&self, cycle: usize) -> bool {
        let y = self.oam_data[0] as usize;
        let x = self.oam_data[3] as usize;
        (y == self.scanline as usize) && x <= cycle && self.mask.show_sprites()
    }

    /// Returns the background palette for a specific column and row on screen.
    fn bg_palette(&self, attribute_table: &[u8], col: usize, row: usize) -> [u8; 4] {
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
            self.palette_table[0],
            self.palette_table[start],
            self.palette_table[start + 1],
            self.palette_table[start + 2],
        ]
    }

    /// Returns the sprite palette for a given index
    fn sprite_palette(&self, idx: u8) -> [u8; 4] {
        let start = 0x11 + (idx * 4) as usize;
        [
            0,
            self.palette_table[start],
            self.palette_table[start + 1],
            self.palette_table[start + 2],
        ]
    }

    /// Renders a given view port.
    fn render_view_port(
        &mut self,
        name_table: &[u8],
        view_port: Rect,
        shift_x: isize,
        shift_y: isize,
    ) {
        let bank = self.ctrl.bgrnd_pattern_addr();

        let attribute_table = &name_table[SCREEN_SIZE..0x400];

        for i in 0..SCREEN_SIZE {
            let tile_column = i % 32;
            let tile_row = i / 32;
            let tile_idx = name_table[i] as u16;
            let tile =
                &self.chr_rom[(bank + tile_idx * 16) as usize..=(bank + tile_idx * 16 + 15) as usize];
            let palette = self.bg_palette(attribute_table, tile_column, tile_row);

            for y in 0..=7 {
                let mut upper = tile[y];
                let mut lower = tile[y + 8];

                for x in (0..=7).rev() {
                    let value = (1 & lower) << 1 | (1 & upper);
                    upper = upper >> 1;
                    lower = lower >> 1;
                    let rgb = match value {
                        0 => &palette::COLOUR_PALETTE[self.palette_table[0] as usize],
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
                        self.frame.set_pixel(
                            (shift_x + pixel_x as isize) as usize,
                            (shift_y + pixel_y as isize) as usize,
                            rgb,
                        );
                    }
                }
            }
        }
    }

    /// Renders the background pixels.
    fn render_bg(&mut self) {
        let scroll_x = (self.scroll.x) as usize;
        let scroll_y = (self.scroll.y) as usize;

        // TODO(dr): Abstracting this out to the PPU bus should fix the
        // ownership mutable/immutable issue.
        let (main_nametable, second_nametable) = match (&self.mirroring, self.ctrl.nametable_addr()) {
            (Mirroring::Vertical, 0x2000)
            | (Mirroring::Vertical, 0x2800)
            | (Mirroring::Horizontal, 0x2000)
            | (Mirroring::Horizontal, 0x2400) => (&self.vram[0..0x400], &self.vram[0x400..0x800]),
            (Mirroring::Vertical, 0x2400)
            | (Mirroring::Vertical, 0x2C00)
            | (Mirroring::Horizontal, 0x2800)
            | (Mirroring::Horizontal, 0x2C00) => (&self.vram[0x400..0x800], &self.vram[0..0x400]),
            (_, _) => {
                panic!("Not supported mirroring type {:?}", self.mirroring);
            }
        };

        self.render_view_port(
            main_nametable,
            Rect::new(scroll_x, scroll_y, 256, 240),
            -(scroll_x as isize),
            -(scroll_y as isize),
        );
        if scroll_x > 0 {
            self.render_view_port(
                second_nametable,
                Rect::new(0, 0, scroll_x, 240),
                (256 - scroll_x) as isize,
                0,
            );
        } else if scroll_y > 0 {
            self.render_view_port(
                second_nametable,
                Rect::new(0, 0, 256, scroll_y),
                0,
                (240 - scroll_y) as isize,
            );
        }
    }

    /// Renders sprites.
    fn render_sprites(&mut self) {
        // Iterate the OAM in reverse to ensure sprite priority is maintained. In
        // the NES OAM, the sprite that occurs first in memory will overlap any that
        // follow.
        for i in (0..self.oam_data.len()).step_by(4).rev() {
            let sprite = Sprite::new(&self.oam_data[i..i + 4]);

            if sprite.behind_background() {
                continue;
            }

            let sprite_palette = self.sprite_palette(sprite.palette_index());

            let bank: u16 = self.ctrl.sprite_pattern_addr();

            let tile = &self.chr_rom
                [(bank + sprite.index * 16) as usize..=(bank + sprite.index * 16 + 15) as usize];

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
                    match (sprite.flip_horizontal(), sprite.flip_vertical()) {
                        (false, false) => self.frame.set_pixel(sprite.x + x, sprite.y + y, rgb),
                        (true, false) => self.frame.set_pixel(sprite.x + 7 - x, sprite.y + y, rgb),
                        (false, true) => self.frame.set_pixel(sprite.x + x, sprite.y + 7 - y, rgb),
                        (true, true) => self.frame.set_pixel(sprite.x + 7 - x, sprite.y + 7 - y, rgb),
                    }
                }
            }
        }
    }
}

impl PPU for NESPPU<'_> {
    /// Writes value to the address register.
    fn write_addr(&mut self, value: u8) {
        self.addr.update(value);
    }

    /// Writes to the control register.
    fn write_ctrl(&mut self, value: u8) {
        let start_nmi = self.ctrl.vblank_nmi();

        self.ctrl.update(value);

        if !start_nmi && self.ctrl.vblank_nmi() && self.status.is_in_vblank() {
            self.nmi_interrupt = Some(true);
        }
    }

    /// Writes to the mask register.
    fn write_mask(&mut self, value: u8) {
        self.mask.update(value);
    }

    /// Writes to the scroll register.
    fn write_scroll(&mut self, value: u8) {
        self.scroll.write(value);
    }

    fn write_oam_addr(&mut self, value: u8) {
        self.oam_addr = value;
    }

    fn write_oam_data(&mut self, value: u8) {
        self.oam_data[self.oam_addr as usize] = value;
        self.oam_addr = self.oam_addr.wrapping_add(1);
    }

    fn write_oam_dma(&mut self, data: &[u8; 256]) {
        for x in data.iter() {
            self.oam_data[self.oam_addr as usize] = *x;
            self.oam_addr = self.oam_addr.wrapping_add(1);
        }
    }

    /// Writes data to appropriate location based on the address register.
    fn write_data(&mut self, value: u8) {
        let addr = self.addr.get();
        match addr {
            0..=0x1FFF => println!("attempt to write to chr rom space {}", addr),
            0x2000..=0x2FFF => {
                self.vram[self.mirror_vram_addr(addr) as usize] = value;
            }
            0x3000..=0x3eff => unimplemented!("addr {} shouldn't be used in reallity", addr),

            // Addresses $3F10/$3F14/$3F18/$3F1C are mirrors of
            // $3F00/$3F04/$3F08/$3F0C
            0x3F10 | 0x3F14 | 0x3F18 | 0x3F1C => {
                let add_mirror = addr - 0x10;
                self.palette_table[(add_mirror - 0x3F00) as usize] = value;
            }
            0x3F00..=0x3FFF => {
                self.palette_table[(addr - 0x3F00) as usize] = value;
            }
            _ => panic!("unexpected access to mirrored space {}", addr),
        }
        self.increment_vram_addr();
    }

    /// Retuns data from appropriate source based on the address register.
    fn read_data(&mut self) -> u8 {
        let addr = self.addr.get();
        self.increment_vram_addr();

        match addr {
            0..=0x1FFF => {
                let result = self.buf;
                self.buf = self.chr_rom[addr as usize];
                result
            }
            0x2000..=0x2FFF => {
                let result = self.buf;
                self.buf = self.vram[self.mirror_vram_addr(addr) as usize];
                result
            }
            0x3000..=0x3EFF => panic!(
                "addr space 0x3000..0x3EFF is not expected to be used, requested = {} ",
                addr
            ),
            0x3F00..=0x3FFF => self.palette_table[(addr - 0x3F00) as usize],
            _ => panic!("unexpected access to mirrored space {}", addr),
        }
    }

    /// Returns the PPU status register and resets VBLANK + addr.
    fn read_status(&mut self) -> u8 {
        let data = self.status.snapshot();
        self.status.reset_vblank_status();
        self.addr.reset();
        self.scroll.reset_latch();
        data
    }

    fn read_oam_data(&self) -> u8 {
        self.oam_data[self.oam_addr as usize]
    }

    /// Returns number of frames rendered.
    fn read_frame_count(&self) -> u128 {
        self.frame_count
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    fn test_ppu_vram_writes() {
        let mut ppu = NESPPU::new_empty_rom();
        ppu.write_addr(0x23);
        ppu.write_addr(0x05);
        ppu.write_data(0x66);

        assert_eq!(ppu.vram[0x0305], 0x66);
    }

    #[test]
    fn test_ppu_vram_reads() {
        let mut ppu = NESPPU::new_empty_rom();
        ppu.write_ctrl(0);
        ppu.vram[0x0305] = 0x66;

        ppu.write_addr(0x23);
        ppu.write_addr(0x05);

        ppu.read_data();
        assert_eq!(ppu.addr.get(), 0x2306);
        assert_eq!(ppu.read_data(), 0x66);
    }

    #[test]
    fn test_ppu_vram_reads_cross_page() {
        let mut ppu = NESPPU::new_empty_rom();
        ppu.write_ctrl(0);
        ppu.vram[0x01ff] = 0x66;
        ppu.vram[0x0200] = 0x77;

        ppu.write_addr(0x21);
        ppu.write_addr(0xff);

        ppu.read_data();
        assert_eq!(ppu.read_data(), 0x66);
        assert_eq!(ppu.read_data(), 0x77);
    }

    #[test]
    fn test_ppu_vram_reads_step_32() {
        let mut ppu = NESPPU::new_empty_rom();
        ppu.write_ctrl(0b100);
        ppu.vram[0x01ff] = 0x66;
        ppu.vram[0x01ff + 32] = 0x77;
        ppu.vram[0x01ff + 64] = 0x88;

        ppu.write_addr(0x21);
        ppu.write_addr(0xff);

        ppu.read_data();
        assert_eq!(ppu.read_data(), 0x66);
        assert_eq!(ppu.read_data(), 0x77);
        assert_eq!(ppu.read_data(), 0x88);
    }

    // Horizontal: https://wiki.nesdev.com/w/index.php/Mirroring
    //   [0x2000 A ] [0x2400 a ]
    //   [0x2800 B ] [0x2C00 b ]
    #[test]
    fn test_vram_horizontal_mirror() {
        let mut ppu = NESPPU::new_empty_rom();
        ppu.write_addr(0x24);
        ppu.write_addr(0x05);

        ppu.write_data(0x66);

        ppu.write_addr(0x28);
        ppu.write_addr(0x05);

        ppu.write_data(0x77);

        ppu.write_addr(0x20);
        ppu.write_addr(0x05);

        ppu.read_data();
        assert_eq!(ppu.read_data(), 0x66);

        ppu.write_addr(0x2C);
        ppu.write_addr(0x05);

        ppu.read_data();
        assert_eq!(ppu.read_data(), 0x77);
    }

    // Vertical: https://wiki.nesdev.com/w/index.php/Mirroring
    //   [0x2000 A ] [0x2400 B ]
    //   [0x2800 a ] [0x2C00 b ]
    #[test]
    fn test_vram_vertical_mirror() {
        let mut ppu = NESPPU::new(vec![0; 2048], Mirroring::Vertical, |_| {});

        ppu.write_addr(0x20);
        ppu.write_addr(0x05);

        ppu.write_data(0x66);

        ppu.write_addr(0x2C);
        ppu.write_addr(0x05);

        ppu.write_data(0x77);

        ppu.write_addr(0x28);
        ppu.write_addr(0x05);

        ppu.read_data();
        assert_eq!(ppu.read_data(), 0x66);

        ppu.write_addr(0x24);
        ppu.write_addr(0x05);

        ppu.read_data();
        assert_eq!(ppu.read_data(), 0x77);
    }

    #[test]
    fn test_read_status_resets_latch() {
        let mut ppu = NESPPU::new_empty_rom();
        ppu.vram[0x0305] = 0x66;

        ppu.write_addr(0x21);
        ppu.write_addr(0x23);
        ppu.write_addr(0x05);

        ppu.read_data();
        assert_ne!(ppu.read_data(), 0x66);

        ppu.read_status();

        ppu.write_addr(0x23);
        ppu.write_addr(0x05);

        ppu.read_data();
        assert_eq!(ppu.read_data(), 0x66);
    }

    #[test]
    fn test_ppu_vram_mirroring() {
        let mut ppu = NESPPU::new_empty_rom();
        ppu.write_ctrl(0);
        ppu.vram[0x0305] = 0x66;

        ppu.write_addr(0x63);
        ppu.write_addr(0x05);

        ppu.read_data();
        assert_eq!(ppu.read_data(), 0x66);
    }

    #[test]
    fn test_read_status_resets_vblank() {
        let mut ppu = NESPPU::new_empty_rom();
        ppu.status.set_vblank_status(true);

        let status = ppu.read_status();

        assert_eq!(status >> 7, 1);
        assert_eq!(ppu.status.snapshot() >> 7, 0);
    }

    #[test]
    fn test_oam_read_write() {
        let mut ppu = NESPPU::new_empty_rom();
        ppu.write_oam_addr(0x10);
        ppu.write_oam_data(0x66);
        ppu.write_oam_data(0x77);

        ppu.write_oam_addr(0x10);
        assert_eq!(ppu.read_oam_data(), 0x66);

        ppu.write_oam_addr(0x11);
        assert_eq!(ppu.read_oam_data(), 0x77);
    }

    #[test]
    fn test_oam_dma() {
        let mut ppu = NESPPU::new_empty_rom();

        let mut data = [0x66; 256];
        data[0] = 0x77;
        data[255] = 0x88;

        ppu.write_oam_addr(0x10);
        ppu.write_oam_dma(&data);

        ppu.write_oam_addr(0xF);
        assert_eq!(ppu.read_oam_data(), 0x88);

        ppu.write_oam_addr(0x10);
        assert_eq!(ppu.read_oam_data(), 0x77);

        ppu.write_oam_addr(0x11);
        assert_eq!(ppu.read_oam_data(), 0x66);
    }
}
