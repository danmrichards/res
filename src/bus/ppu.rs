use std::{cell::RefCell, rc::Rc};

use crate::cartridge::{Cartridge, Mirroring};

const ROM: u16 = 0x0000;
const ROM_END: u16 = 0x1FFF;
const VRAM: u16 = 0x2000;
const VRAM_END: u16 = 0x3EFF;
const PALETTE: u16 = 0x3F00;
const PALETTE_END: u16 = 0x3FFF;

/// PPUBus abstracts a single location for interacting with vram and palette
/// memory.
pub struct PPUBus {
    cart: Rc<RefCell<Cartridge>>,

    /// Internal reference to colour palettes.
    pub palette_table: [u8; 32],

    /// Video RAM.
    pub vram: [u8; 2048],
}

pub trait Memory {
    fn write_data(&mut self, addr: u16, value: u8);
    fn read_data(&mut self, addr: u16) -> u8;
}

impl PPUBus {
    pub fn new(cart: Rc<RefCell<Cartridge>>) -> Self {
        PPUBus {
            cart,
            palette_table: [0; 32],
            vram: [0; 2048],
        }
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

        match (self.cart.borrow().mirroring(), name_table) {
            (Mirroring::Vertical, 2) | (Mirroring::Vertical, 3) => vram_index - 0x800,
            (Mirroring::Horizontal, 2) => vram_index - 0x400,
            (Mirroring::Horizontal, 1) => vram_index - 0x400,
            (Mirroring::Horizontal, 3) => vram_index - 0x800,
            _ => vram_index,
        }
    }
}

impl Memory for PPUBus {
    /// Writes data to appropriate location based on the address register.
    fn write_data(&mut self, addr: u16, data: u8) {
        match addr {
            ROM..=ROM_END => self.cart.borrow_mut().write_chr(addr, data),
            VRAM..=VRAM_END => {
                self.vram[self.mirror_vram_addr(addr) as usize] = data;
            }
            // Addresses $3F10/$3F14/$3F18/$3F1C are mirrors of
            // $3F00/$3F04/$3F08/$3F0C
            0x3F10 | 0x3F14 | 0x3F18 | 0x3F1C => {
                let add_mirror = addr - 0x10;
                self.palette_table[(add_mirror - 0x3F00) as usize] = data;
            }
            PALETTE..=PALETTE_END => {
                self.palette_table[(addr - 0x3F00) as usize] = data;
            }
            _ => unreachable!("unexpected access to mirrored space {}", addr),
        }
    }

    /// Retuns data from appropriate source based on the address register.
    fn read_data(&mut self, addr: u16) -> u8 {
        match addr {
            ROM..=ROM_END => self.cart.borrow().read_chr(addr),
            VRAM..=VRAM_END => self.vram[self.mirror_vram_addr(addr) as usize],
            PALETTE..=PALETTE_END => self.palette_table[(addr - 0x3F00) as usize],
            _ => unreachable!("unexpected access to mirrored space {}", addr),
        }
    }
}
