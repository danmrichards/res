pub struct CPU {
    // Accumulator, a special register for storing results of arithmetic and
    // logical operations.
    pub a: u8,

    // X index register.
    pub x: u8,

    // Processor status register.
    //
    // 7     bit     0
    // ------- -------
    // N V s s D I Z C
    // | | | | | | | |
    // | | | | | | | +- Carry
    // | | | | | | +-- Zero
    // | | | | | +--- Interrupt Disable
    // | | | | +---- Decimal
    // | | + +------ No CPU effect, see: the B flag
    // | +-------- Overflow
    // +--------- Negative
    pub status: u8,

    // Program counter, stores the address of the instruction being executed.
    pub pc: u16,

    // Memory stores the data available to the CPU.
    memory: [u8; 0xFFFF],
}

impl CPU {
    // Returns an instantiated CPU.
    pub fn new() -> Self {
        CPU {
            a: 0,
            x: 0,
            status: 0,
            pc: 0,
            memory: [0; 0xFFFF],
        }
    }

    // Resets the CPU and marks where it should begin execution.
    //
    // Emulates the "reset interrupt" signal that is sent to the NES CPU when a
    // cartridge is inserted.
    pub fn reset(&mut self) {
        self.a = 0;
        self.x = 0;
        self.status = 0;

        self.pc = self.mem_read_word(0xFFFC);
    }

    // Loads the program into memory.
    //
    // Program ROM starts at 0x8000 for the NES.
    pub fn load(&mut self, program: Vec<u8>) {
        self.memory[0x8000..(0x8000 + program.len())].copy_from_slice(&program[..]);
        self.mem_write_word(0xFFFC, 0x8000);
    }

    // Loads the program into memory and runs the CPU.
    pub fn load_and_run(&mut self, program: Vec<u8>) {
        self.load(program);
        self.reset();
        self.run();
    }

    // Runs the program loaded into memory.
    pub fn run(&mut self) {
        loop {
            let opcode = self.immediate_byte();

            match opcode {
                0x00 => return,
                0xA9 => self.lda(),
                0xAA => self.tax(),
                0xE8 => self.inx(),
                _ => todo!(""),
            }
        }
    }

    // LDA: Load Accumulator.
    //
    // Loads a byte of memory into the accumulator setting the zero and
    // negative flags as appropriate.
    fn lda(&mut self) {
        let param = self.immediate_byte();
        self.a = param;

        self.update_zero_and_negative_flags(self.a);
    }

    // TAX: Transfer Accumulator to X.
    //
    // Copies the current contents of the accumulator into the X register and
    // sets the zero and negative flags as appropriate.
    fn tax(&mut self) {
        self.x = self.a;
        self.update_zero_and_negative_flags(self.x);
    }

    // INX: Increment X register.
    //
    // Adds one to the X register setting the zero and negative flags as
    // appropriate.
    fn inx(&mut self) {
        self.x = self.x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.x);
    }

    // Returns the byte at the given address in memory.
    fn mem_read_byte(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    // Writes the data at the given address in memory.
    fn mem_write_byte(&mut self, addr: u16, data: u8) {
        self.memory[addr as usize] = data
    }

    // Returns a word from memory, merged from the two bytes at pos and pos + 1.
    fn mem_read_word(&mut self, pos: u16) -> u16 {
        let lo = self.mem_read_byte(pos);
        let hi = self.mem_read_byte(pos + 1);

        u16::from_le_bytes([hi, lo])
    }

    // Writes two bytes to memory, split from the data word, as pos and pos + 1.
    fn mem_write_word(&mut self, pos: u16, data: u16) {
        let bytes = data.to_le_bytes();

        self.mem_write_byte(pos, bytes[1]);
        self.mem_write_byte(pos + 1, bytes[0]);
    }

    // Returns the next byte from memory indicated by the program counter.
    //
    // The program counter is incremented by one after the read.
    fn immediate_byte(&mut self) -> u8 {
        let opcode = self.mem_read_byte(self.pc);
        self.pc += 1;

        opcode
    }

    // Sets the Z (zero) and N (negative) flags on the CPU status based on the
    // given result.
    fn update_zero_and_negative_flags(&mut self, result: u8) {
        // Zero flag should be set if the result is 0.
        if result == 0 {
            self.status = self.status | 0b00000010;
        } else {
            self.status = self.status & 0b11111101;
        }

        // Negative flag should be set if bit 7 of the result is set.
        if result & 0b1000_0000 != 0 {
            self.status = self.status | 0b10000000;
        } else {
            self.status = self.status & 0b01111111;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_0xa9_lda_immediate_load_data() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x05, 0x00]);

        assert_eq!(cpu.a, 0x05);
        assert_eq!(cpu.status & 0b00000010, 0b00);
        assert_eq!(cpu.status & 0b1, 0);
    }

    #[test]
    fn test_0xa9_lda_zero_flag() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x00, 0x00]);

        assert_eq!(cpu.status & 0b00000010, 0b10);
    }

    #[test]
    fn test_0xaa_tax_move_a_to_x() {
        let mut cpu = CPU::new();
        cpu.load(vec![0xaa, 0x00]);
        cpu.reset();
        cpu.a = 10;

        cpu.run();
        assert_eq!(cpu.x, 10)
    }

    #[test]
    fn test_0xe8_inx_increment_x() {
        let mut cpu = CPU::new();
        cpu.load(vec![0xe8, 0x00]);
        cpu.reset();
        cpu.x = 1;

        cpu.run();
        assert_eq!(cpu.x, 2)
    }

    #[test]
    fn test_inx_overflow() {
        let mut cpu = CPU::new();
        cpu.load(vec![0xe8, 0xe8, 0x00]);
        cpu.reset();

        cpu.x = 0xff;
        cpu.run();

        assert_eq!(cpu.x, 1)
    }

    #[test]
    fn test_5_ops_working_together() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0xc0, 0xaa, 0xe8, 0x00]);

        assert_eq!(cpu.x, 0xc1)
    }
}
