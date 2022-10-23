use crate::cpu::Memory;

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

// Bus abstracts a single location data read/write, interrupts, memory mapping
// and PPU/CPU clock cycles.
pub struct Bus {
    cpu_vram: [u8; 2048],
}

impl Bus {
    // Returns an instantiated Bus.
    pub fn new() -> Self {
        Bus {
            cpu_vram: [0; 2048],
        }
    }
}

impl Memory for Bus {
    fn mem_read_byte(&self, addr: u16) -> u8 {
        match addr {
            RAM ..= RAM_MIRRORS_END => {
                let mirror_down_addr = addr & 0b00000111_11111111;
                self.cpu_vram[mirror_down_addr as usize]
            }
            PPU_REGISTERS ..= PPU_REGISTERS_MIRRORS_END => {
                let _mirror_down_addr = addr & 0b00100000_00000111;
                todo!("PPU is not supported yet")
            }
            _ => {
                println!("Ignoring mem access at {}", addr);
                0
            }
        }
    }

    fn mem_write_byte(&mut self, addr: u16, data: u8) {
        match addr {
            RAM ..= RAM_MIRRORS_END => {
                let mirror_down_addr = addr & 0b11111111111;
                self.cpu_vram[mirror_down_addr as usize] = data;
            }
            PPU_REGISTERS ..= PPU_REGISTERS_MIRRORS_END => {
                let _mirror_down_addr = addr & 0b00100000_00000111;
                todo!("PPU is not supported yet");
            }
            _ => {
                println!("Ignoring mem write-access at {}", addr);
            }
        }
    }
}
