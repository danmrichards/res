use crate::cartridge::Mirroring;
use registers::addr::Addr;
use registers::control::Control;

pub mod registers;

// Represents the NES CPU.
pub struct PPU {
    // Character (visuals) ROM.
    pub chr_rom: Vec<u8>,

    // Internal reference to colour palettes.
    pub palette_table: [u8; 32],

    // Video RAM.
    pub vram: [u8; 2048],

    // Object attribute memory (sprites).
    pub oam_data: [u8; 256],

    pub mirroring: Mirroring,

    // Registers.
    pub addr: Addr,
    pub ctrl: Control,
}

impl PPU {
    // Returns an instantiated PPU.
    pub fn new(chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        PPU {
            chr_rom: chr_rom,
            palette_table: [0; 32],
            vram: [0; 2048],
            oam_data: [0; 64 * 4],
            mirroring: mirroring,
            addr: Addr::new(),
            ctrl: Control::new(),
        }
    }

    // Writes value to the address register.
    fn write_to_addr(&mut self, value: u8) {
        self.addr.update(value);
    }

    // Writes to the control register.
    fn write_to_ctrl(&mut self, value: u8) {
        self.ctrl.update(value);
    }

    // Increment the VRAM address based on the control register status.
    fn increment_vram_addr(&mut self) {
        self.addr.increment(self.ctrl.vram_addr_increment());
    }
 
    // Retuns data from appropriate source based on the address register.
    fn read_data(&mut self) -> u8 {
        let addr = self.addr.get();
        self.increment_vram_addr();
 
        match addr {
            0..=0x1fff => todo!("read from chr_rom"),
            0x2000..=0x2fff => todo!("read from RAM"),
            0x3000..=0x3eff => panic!("addr space 0x3000..0x3EFF is not expected to be used, requested = {} ", addr),
            0x3f00..=0x3fff =>
            {
                self.palette_table[(addr - 0x3f00) as usize]
            }
            _ => panic!("unexpected access to mirrored space {}", addr),
        }
    }
}
