pub mod registers;

mod frame;
mod palette;
mod sprite;

use crate::bus::Memory;
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
    /// Bus to allow PPU to interact with RAM/ROM.
    bus: Box<dyn Memory>,

    /// Object attribute memory (sprites).
    pub oam_addr: u8,
    pub oam_data: [u8; 256],

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
    pub fn new<'rcall, F>(bus: Box<dyn Memory>, render_callback: F) -> NESPPU<'rcall>
    where
        F: FnMut(&[u8]) + 'rcall,
    {
        NESPPU {
            bus,
            oam_addr: 0,
            oam_data: [0; 64 * 4],
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

    /// Increment the VRAM address based on the control register status.
    fn increment_vram_addr(&mut self) {
        self.addr.increment(self.ctrl.vram_addr_increment());
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

        // VBLANK is triggered at scanline 241.
        if self.scanline == 241 {
            self.status.set_vblank_status(true);
            self.status.set_sprite_zero_hit(false);

            // Set the interrupt if the control register allows it.
            if self.ctrl.vblank_nmi() {
                self.nmi_interrupt = Some(true);
            }

            self.render_bg();
            self.render_sprites();

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

    /// Renders the background pixels.
    fn render_bg(&mut self) {
        let scroll_x = (self.scroll.x) as usize;
        let scroll_y = (self.scroll.y) as usize;

        let (main_nametable, second_nametable) = self.bus.nametables(self.ctrl.nametable_addr());

        let bank = self.ctrl.bgrnd_pattern_addr();

        // Render the main viewport.
        let mut name_table = main_nametable;
        let mut view_port = Rect::new(scroll_x, scroll_y, 256, 240);
        let mut shift_x = -(scroll_x as isize);
        let mut shift_y = -(scroll_y as isize);

        let mut attribute_table = &name_table[SCREEN_SIZE..0x400];

        for i in 0..SCREEN_SIZE {
            let tile_column = i % 32;
            let tile_row = i / 32;
            let tile_idx = name_table[i] as u16;
            let tile = self.bus.read_chr_rom(
                (bank + tile_idx * 16) as usize,
                (bank + tile_idx * 16 + 15) as usize,
            );
            let palette = self.bus.bg_palette(attribute_table, tile_column, tile_row);

            for y in 0..=7 {
                let mut upper = tile[y];
                let mut lower = tile[y + 8];

                for x in (0..=7).rev() {
                    let mut value = 0;
                    if self.mask.show_background() {
                        value = (1 & lower) << 1 | (1 & upper);
                        upper = upper >> 1;
                        lower = lower >> 1;
                    }

                    let rgb = match value {
                        0 => &palette::COLOUR_PALETTE[self.bus.read_palette_table(0) as usize],
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

        // Render the second viewport.
        // TODO(dr): Yes this is a shit load of duplication, but it's to get
        // around the mutability checker problems. And this whole code path will
        // get deleted when rendering is implemented properly.

        if scroll_x > 0 {
            name_table = second_nametable;
            view_port = Rect::new(0, 0, scroll_x, 240);
            shift_x = (256 - scroll_x) as isize;
            shift_y = 0;
        } else if scroll_y > 0 {
            name_table = second_nametable;
            view_port = Rect::new(0, 0, 256, scroll_y);
            shift_x = 0;
            (240 - scroll_y) as isize;
        }

        attribute_table = &name_table[SCREEN_SIZE..0x400];
        for i in 0..SCREEN_SIZE {
            let tile_column = i % 32;
            let tile_row = i / 32;
            let tile_idx = name_table[i] as u16;
            let tile = self.bus.read_chr_rom(
                (bank + tile_idx * 16) as usize,
                (bank + tile_idx * 16 + 15) as usize,
            );
            let palette = self.bus.bg_palette(attribute_table, tile_column, tile_row);

            for y in 0..=7 {
                let mut upper = tile[y];
                let mut lower = tile[y + 8];

                for x in (0..=7).rev() {
                    let value = (1 & lower) << 1 | (1 & upper);
                    upper = upper >> 1;
                    lower = lower >> 1;
                    let rgb = match value {
                        0 => &palette::COLOUR_PALETTE[self.bus.read_palette_table(0) as usize],
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

            let sprite_palette = self.bus.sprite_palette(sprite.palette_index());

            let bank: u16 = self.ctrl.sprite_pattern_addr();

            let tile = &self.bus.read_chr_rom(
                (bank + sprite.index * 16) as usize,
                (bank + sprite.index * 16 + 15) as usize,
            );

            // Draw the 8x8 sprite.
            for y in 0..=7 {
                let mut upper = tile[y];
                let mut lower = tile[y + 8];
                for x in (0..=7).rev() {
                    let mut value = 0;
                    if self.mask.show_sprites() {
                        value = (1 & lower) << 1 | (1 & upper);
                        upper = upper >> 1;
                        lower = lower >> 1;
                    }

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
                        (true, true) => {
                            self.frame
                                .set_pixel(sprite.x + 7 - x, sprite.y + 7 - y, rgb)
                        }
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

    fn write_data(&mut self, data: u8) {
        self.bus.write_data(self.addr.get(), data);
        self.increment_vram_addr();
    }

    fn read_data(&mut self) -> u8 {
        let addr = self.addr.get();
        self.increment_vram_addr();

        // Reading data takes 2 reads to get the data. The first read put the
        // data in a buffer amd the second read puts the buffer data on the bus.
        let result = self.buf;
        self.buf = self.bus.read_data(addr);
        result
    }
}

#[cfg(test)]
pub mod test {
    use crate::bus::PPUBus;
    use crate::cartridge::Mirroring;

    use super::*;

    /// Returns an instatiated PPU with an empty ROM loaded.
    pub fn new_empty_rom_ppu() -> NESPPU<'static> {
        let bus = PPUBus::new(vec![0; 2048], Mirroring::Horizontal);
        NESPPU::new(Box::new(bus), |_| {})
    }

    #[test]
    fn test_ppu_vram_writes() {
        let mut ppu = new_empty_rom_ppu();
        ppu.write_addr(0x23);
        ppu.write_addr(0x05);
        ppu.write_data(0x66);

        assert_eq!(ppu.bus.read_data(0x0305), 0x66);
    }

    #[test]
    fn test_ppu_vram_reads() {
        let mut ppu = new_empty_rom_ppu();
        ppu.write_ctrl(0);
        ppu.bus.write_data(0x0305, 0x66);

        ppu.write_addr(0x23);
        ppu.write_addr(0x05);

        ppu.read_data();
        assert_eq!(ppu.addr.get(), 0x2306);
        assert_eq!(ppu.read_data(), 0x66);
    }

    #[test]
    fn test_ppu_vram_reads_cross_page() {
        let mut ppu = new_empty_rom_ppu();
        ppu.write_ctrl(0);
        ppu.bus.write_data(0x01ff, 0x66);
        ppu.bus.write_data(0x0200, 0x77);

        ppu.write_addr(0x21);
        ppu.write_addr(0xff);

        ppu.read_data();
        assert_eq!(ppu.read_data(), 0x66);
        assert_eq!(ppu.read_data(), 0x77);
    }

    #[test]
    fn test_ppu_vram_reads_step_32() {
        let mut ppu = new_empty_rom_ppu();
        ppu.write_ctrl(0b100);
        ppu.bus.write_data(0x01ff, 0x66);
        ppu.bus.write_data(0x01ff + 32, 0x77);
        ppu.bus.write_data(0x01ff + 64, 0x88);

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
        let mut ppu = new_empty_rom_ppu();
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
        let bus = PPUBus::new(vec![0; 2048], Mirroring::Vertical);
        let ppu = NESPPU::new(Box::new(bus), |_| {});

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
        let mut ppu = new_empty_rom_ppu();
        ppu.bus.write_data(0x0305, 0x66);

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
        let mut ppu = new_empty_rom_ppu();
        ppu.write_ctrl(0);
        ppu.bus.write_data(0x0305, 0x66);

        ppu.write_addr(0x63);
        ppu.write_addr(0x05);

        ppu.read_data();
        assert_eq!(ppu.read_data(), 0x66);
    }

    #[test]
    fn test_read_status_resets_vblank() {
        let mut ppu = new_empty_rom_ppu();
        ppu.status.set_vblank_status(true);

        let status = ppu.read_status();

        assert_eq!(status >> 7, 1);
        assert_eq!(ppu.status.snapshot() >> 7, 0);
    }

    #[test]
    fn test_oam_read_write() {
        let mut ppu = new_empty_rom_ppu();
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
        let mut ppu = new_empty_rom_ppu();

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
