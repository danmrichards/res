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

    // Program for the CPU to interpret.
    program: Vec<u8>,
}

impl CPU {
    // Returns an instantiated CPU.
    pub fn new() -> Self {
        CPU {
            a: 0,
            x: 0,
            status: 0,
            pc: 0,
            program: vec![],
        }
    }

    // Loads a NES program, as a byte vector, into memory.
    pub fn load_program(&mut self, program: Vec<u8>) {
        self.program = program;
    }

    // Runs the program loaded into memory.
    pub fn run(&mut self) {
        self.pc = 0;

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

    // Returns the next byte from memory indicated by the program counter.
    //
    // The program counter is incremented by one after the read.
    fn immediate_byte(&mut self) -> u8 {
        let opcode = self.program[self.pc as usize];
        self.pc += 1;

        opcode
    }

    fn update_zero_and_negative_flags(&mut self, result: u8) {
        if result == 0 {
            self.status = self.status | 0b00000010;
        } else {
            self.status = self.status & 0b11111101;
        }

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
        cpu.load_program(vec![0xa9, 0x05, 0x00]);

        cpu.run();
        assert_eq!(cpu.a, 0x05);
        assert_eq!(cpu.status & 0b00000010, 0b00);
        assert_eq!(cpu.status & 0b1, 0);
    }

    #[test]
    fn test_0xa9_lda_zero_flag() {
        let mut cpu = CPU::new();
        cpu.load_program(vec![0xa9, 0x00, 0x00]);

        cpu.run();
        assert_eq!(cpu.status & 0b00000010, 0b10);
    }

    #[test]
    fn test_0xaa_tax_move_a_to_x() {
        let mut cpu = CPU::new();
        cpu.load_program(vec![0xaa, 0x00]);
        cpu.a = 10;

        cpu.run();
        assert_eq!(cpu.x, 10)
    }

    #[test]
    fn test_0xe8_inx_increment_x() {
        let mut cpu = CPU::new();
        cpu.load_program(vec![0xe8, 0x00]);
        cpu.x = 1;

        cpu.run();
        assert_eq!(cpu.x, 2)
    }

    #[test]
    fn test_inx_overflow() {
        let mut cpu = CPU::new();
        cpu.x = 0xff;
        cpu.load_program(vec![0xe8, 0xe8, 0x00]);

        cpu.run();
        assert_eq!(cpu.x, 1)
    }

    #[test]
    fn test_5_ops_working_together() {
        let mut cpu = CPU::new();
        cpu.load_program(vec![0xa9, 0xc0, 0xaa, 0xe8, 0x00]);

        cpu.run();
        assert_eq!(cpu.x, 0xc1)
    }
}
