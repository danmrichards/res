use crate::cartridge::Rom;
use crate::cpu::Memory;
use crate::joypad::Joypad;
use crate::ppu::NesPpu;
use crate::ppu::Ppu;

use super::PPUBus;

/// | Address range | Size  | Device                                                                  |
/// | ------------- | ----- | ----------------------------------------------------------------------- |
/// | $0000-$07FF   | $0800 | 2KB internal RAM                                                        |
/// | $0800-$0FFF   | $0800 | Mirrors of $0000-$07FF                                                  |
/// | $1000-$17FF   | $0800 |                                                                         |
/// | $1800-$1FFF   | $0800 |                                                                         |
/// | $2000-$2007   | $0008 | NES PPU registers                                                       |
/// | $2008-$3FFF   | $1FF8 | Mirrors of $2000-2007 (repeats every 8 bytes)                           |
/// | $4000-$4017   | $0018 | NES APU and I/O registers                                               |
/// | $4018-$401F   | $0008 | APU and I/O functionality that is normally disabled. See CPU Test Mode. |
/// | $4020-$FFFF   | $BFE0 | Cartridge space: PRG ROM, PRG RAM, and mapper registers (See Note)      |
const RAM: u16 = 0x0000;
const RAM_MIRRORS_END: u16 = 0x1FFF;
const PPU_REGISTERS: u16 = 0x2000;
const PPU_REGISTERS_MIRRORS_END: u16 = 0x3FFF;
const PRG: u16 = 0x8000;
const PRG_END: u16 = 0xFFFF;

/// SystemBus abstracts a single location for data read/write, interrupts,
/// memory mapping and PPU/CPU clock cycles.
pub struct SystemBus<'a> {
    ram: [u8; 2048],
    prg_rom: Vec<u8>,
    ppu: NesPpu<'a>,
    pub joypad1: Joypad,
}

impl<'a> SystemBus<'a> {
    /// Returns an instantiated Bus.
    pub fn new<F>(rom: Rom, render_callback: F) -> Self
    where
        F: FnMut(&[u8]) + 'a,
    {
        let ppu_bus = PPUBus::new(rom.chr, rom.screen_mirroring);
        let ppu = NesPpu::new(Box::new(ppu_bus), Box::new(render_callback));

        SystemBus {
            ram: [0; 2048],
            prg_rom: rom.prg,
            ppu,
            joypad1: Joypad::new(),
        }
    }

    /// Returns a byte from PRG ROM at the given address.
    fn read_prg(&self, mut addr: u16) -> u8 {
        addr -= PRG;
        if self.prg_rom.len() == 0x4000 && addr >= 0x4000 {
            // Mirror if needed
            addr %= 0x4000;
        }
        self.prg_rom[addr as usize]
    }

    /// For every CPU tick, run the PPU and APU appropriately.
    pub fn tick(&mut self, cycles: u8) {
        for _ in 0..cycles {
            // PPU runs three times faster than CPU.
            for _ in 0..3 {
                self.ppu.clock();
            }

            // TODO(dr): Clock the APU.
        }
    }

    /// Returns the NMI status of the PPU.
    pub fn nmi_status(&mut self) -> bool {
        self.ppu.poll_nmi()
    }

    /// Returns the number of rendered frames from the PPU.
    pub fn ppu_frame_count(&self) -> u128 {
        self.ppu.read_frame_count()
    }
}

impl Memory for SystemBus<'_> {
    fn mem_read_byte(&mut self, addr: u16) -> u8 {
        match addr {
            RAM..=RAM_MIRRORS_END => {
                let mirror_down_addr = addr & 0b00000111_11111111;
                self.ram[mirror_down_addr as usize]
            }
            PPU_REGISTERS | 0x2001 | 0x2003 | 0x2005 | 0x2006 | 0x4014 => 0,
            0x2002 => self.ppu.read_status(),
            0x2004 => self.ppu.read_oam_data(),
            0x2007 => self.ppu.read_data(),

            0x4000..=0x4015 => {
                //ignore APU
                0
            }

            0x4016 => self.joypad1.read(),

            0x4017 => {
                // ignore joypad 2
                0
            }
            0x2008..=PPU_REGISTERS_MIRRORS_END => {
                let mirror_down_addr = addr & 0b00100000_00000111;
                self.mem_read_byte(mirror_down_addr)
            }
            PRG..=PRG_END => self.read_prg(addr),

            _ => 0,
        }
    }

    fn mem_write_byte(&mut self, addr: u16, data: u8) {
        self.ppu.refresh_open_bus(data);

        match addr {
            RAM..=RAM_MIRRORS_END => {
                let mirror_down_addr = addr & 0b11111111111;
                self.ram[mirror_down_addr as usize] = data;
            }
            PPU_REGISTERS => {
                self.ppu.write_ctrl(data);
            }

            0x2001 => {
                self.ppu.write_mask(data);
            }
            0x2002 => panic!("attempt to write to PPU status register"),

            0x2003 => {
                self.ppu.write_oam_addr(data);
            }
            0x2004 => {
                self.ppu.write_oam_data(data);
            }
            0x2005 => {
                self.ppu.write_scroll(data);
            }
            0x2006 => {
                self.ppu.write_addr(data);
            }
            0x2007 => {
                self.ppu.write_data(data);
            }
            0x4000..=0x4013 | 0x4015 => {
                //ignore APU
            }
            0x4016 => {
                self.joypad1.write(data);
            }
            0x4017 => {
                // ignore joypad 2
            }
            0x4014 => {
                let mut buffer: [u8; 256] = [0; 256];
                let hi: u16 = (data as u16) << 8;
                for i in 0..256u16 {
                    buffer[i as usize] = self.mem_read_byte(hi + i);
                }

                self.ppu.write_oam_dma(&buffer);
            }
            0x2008..=PPU_REGISTERS_MIRRORS_END => {
                let mirror_down_addr = addr & 0b00100000_00000111;
                self.mem_write_byte(mirror_down_addr, data);
            }
            0x8000..=0xFFFF => panic!("Attempt to write to Cartridge ROM space: {:x}", addr),

            _ => {
                println!("Ignoring mem write-access at {}", addr);
            }
        }
    }
}

#[cfg(test)]
mod test {
    // use super::*;
    // use crate::cartridge::test;

    // #[test]
    // fn test_mem_read_write_to_ram() {
    //     let mut bus = SystemBus::new(test::test_rom(), |_| {});
    //     bus.mem_write_byte(0x01, 0x55);
    //     assert_eq!(bus.mem_read_byte(0x01), 0x55);
    // }
}
