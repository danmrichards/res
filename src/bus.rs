use crate::cpu::Memory;
use crate::cartridge::Rom;

// | Address range | Size  | Device                                                                  |
// | ------------- | ----- | ----------------------------------------------------------------------- |
// | $0000-$07FF   | $0800 | 2KB internal RAM                                                        |
// | $0800-$0FFF   | $0800 | Mirrors of $0000-$07FF                                                  |
// | $1000-$17FF   | $0800 |                                                                         |
// | $1800-$1FFF   | $0800 |                                                                         |
// | $2000-$2007   | $0008 | NES PPU registers                                                       |
// | $2008-$3FFF   | $1FF8 | Mirrors of $2000-2007 (repeats every 8 bytes)                           |
// | $4000-$4017   | $0018 | NES APU and I/O registers                                               |
// | $4018-$401F   | $0008 | APU and I/O functionality that is normally disabled. See CPU Test Mode. |
// | $4020-$FFFF   | $BFE0 | Cartridge space: PRG ROM, PRG RAM, and mapper registers (See Note)      |
const RAM: u16 = 0x0000;
const RAM_MIRRORS_END: u16 = 0x1FFF;
const PPU_REGISTERS: u16 = 0x2000;
const PPU_REGISTERS_MIRRORS_END: u16 = 0x3FFF;
const PRG: u16 = 0x8000;
const PRG_END: u16 = 0xFFFF;

// Bus abstracts a single location data read/write, interrupts, memory mapping
// and PPU/CPU clock cycles.
pub struct Bus {
    cpu_vram: [u8; 2048],
    rom: Rom,
}

impl Bus {
    // Returns an instantiated Bus.
    pub fn new(rom: Rom) -> Self {
        Bus {
            cpu_vram: [0; 2048],
            rom,
        }
    }

    // Returns a byte from PRG ROM at the given address.
    fn read_prg(&self, mut addr: u16) -> u8 {
        addr -= PRG;
        if self.rom.prg.len() == 0x4000 && addr >= 0x4000 {
            // Mirror if needed
            addr = addr % 0x4000;
        }
        self.rom.prg[addr as usize]
    }
}

impl Memory for Bus {
    fn mem_read_byte(&self, addr: u16) -> u8 {
        match addr {
            RAM..=RAM_MIRRORS_END => {
                let mirror_down_addr = addr & 0b00000111_11111111;
                self.cpu_vram[mirror_down_addr as usize]
            }
            PPU_REGISTERS..=PPU_REGISTERS_MIRRORS_END => {
                let _mirror_down_addr = addr & 0b00100000_00000111;
                todo!("PPU is not supported yet")
            }
            PRG..=PRG_END => self.read_prg(addr),
            _ => {
                println!("Ignoring mem access at {}", addr);
                0
            }
        }
    }

    fn mem_write_byte(&mut self, addr: u16, data: u8) {
        match addr {
            RAM..=RAM_MIRRORS_END => {
                let mirror_down_addr = addr & 0b11111111111;
                self.cpu_vram[mirror_down_addr as usize] = data;
            }
            PPU_REGISTERS..=PPU_REGISTERS_MIRRORS_END => {
                let _mirror_down_addr = addr & 0b00100000_00000111;
                todo!("PPU is not supported yet");
            }
            PRG..=PRG_END => {
                panic!("Attempt to write to cartridge ROM space")
            }
            _ => {
                println!("Ignoring mem write-access at {}", addr);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::cartridge::test;

    #[test]
    fn test_mem_read_write_to_ram() {
        let mut bus = Bus::new(test::test_rom());
        bus.mem_write_byte(0x01, 0x55);
        assert_eq!(bus.mem_read_byte(0x01), 0x55);
    }
}
