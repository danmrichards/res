use crate::bus::Bus;
use crate::instructions;
use std::collections::HashMap;

#[derive(Debug)]
#[allow(non_camel_case_types)]
// Represents the different types of addressing mode supported by the CPU.
pub enum AddressingMode {
    // Allows the programmer to directly specify an 8 bit constant within the
    // instruction.
    Immediate,

    // An instruction using zero page addressing mode has only an 8 bit address
    // operand. This limits it to addressing only the first 256 bytes of memory
    // (e.g. $0000 to $00FF) where the most significant byte of the address is
    // always zero. In zero page mode only the least significant byte of the
    // address is held in the instruction making it shorter by one byte
    // (important for space saving) and one less memory fetch during execution
    // (important for speed).
    ZeroPage,

    // The address to be accessed by an instruction using indexed zero page
    // addressing is calculated by taking the 8 bit zero page address from the
    // instruction and adding the current value of the X register to it.
    ZeroPageX,

    // The address to be accessed by an instruction using indexed zero page
    // addressing is calculated by taking the 8 bit zero page address from the
    // instruction and adding the current value of the Y register to it. This
    // mode can only be used with the LDX and STX instructions.
    ZeroPageY,

    // Instructions using absolute addressing contain a full 16 bit address to
    // identify the target location.
    Absolute,

    // The address to be accessed by an instruction using X register indexed
    // absolute addressing is computed by taking the 16 bit address from the
    // instruction and added the contents of the X register.
    AbsoluteX,

    // The Y register indexed absolute addressing mode is the same as the
    // previous mode only with the contents of the Y register added to the 16
    // bit address from the instruction.
    AbsoluteY,

    // Indexed indirect addressing is normally used in conjunction with a table
    // of address held on zero page. The address of the table is taken from the
    // instruction and the X register added to it (with zero page wrap around)
    // to give the location of the least significant byte of the target address.
    IndirectX,

    // Indirect indirect addressing is the most common indirection mode used on
    // the 6502. In instruction contains the zero page location of the least
    // significant byte of 16 bit address. The Y register is dynamically added
    // to this value to generated the actual target address for operation.
    IndirectY,

    // Used when an opcode takes no operand.
    Implied,
}

pub trait Memory {
    // Returns the byte at the given address in memory.
    fn mem_read_byte(&mut self, addr: u16) -> u8;

    // Writes the data at the given address in memory.
    fn mem_write_byte(&mut self, addr: u16, data: u8);

    // Returns a word from memory, merged from the two bytes at pos and pos + 1.
    fn mem_read_word(&mut self, pos: u16) -> u16 {
        let lo = self.mem_read_byte(pos);
        let hi = self.mem_read_byte(pos + 1);

        u16::from_le_bytes([lo, hi])
    }

    // Writes two bytes to memory, split from the data word, as pos and pos + 1.
    fn mem_write_word(&mut self, pos: u16, data: u16) {
        let bytes = data.to_le_bytes();

        self.mem_write_byte(pos, bytes[0]);
        self.mem_write_byte(pos + 1, bytes[1]);
    }
}

// Stack is located from $0100-$01FF.
const STACK: u16 = 0x0100;
const STACK_RESET: u8 = 0xFD;
const STATUS_DEFAULT: u8 = 0b00100100;

// Represents the NES CPU.
pub struct CPU {
    // Accumulator, a special register for storing results of arithmetic and
    // logical operations.
    pub a: u8,

    // X index register.
    pub x: u8,

    // Y index register.
    pub y: u8,

    // Processor status register.
    //
    // 7     bit     0
    // ------- -------
    // N V _ B D I Z C
    // | |   | | | | |
    // | |   | | | | +- Carry
    // | |   | | | +--- Zero
    // | |   | | +----- Interrupt disable
    // | |   | +------- Decimal
    // | |   +--------- Break flag
    // | |
    // | +------------- Overflow
    // +--------------- Negative
    pub status: u8,

    // Program counter, stores the address of the instruction being executed.
    pub pc: u16,

    // Stack pointer, an 8-bit register which serves as an offset from $0100.
    // The stack works top-down, so when a byte is pushed on to the stack, the
    // stack pointer is decremented and when a byte is pulled from the stack,
    // the stack pointer is incremented
    pub sp: u8,

    // Handles data read/write, interrupts, memory mapping and PPU/CPU clock
    // cycles.
    pub bus: Bus,
}

impl Memory for CPU {
    // Returns the byte at the given address in memory.
    fn mem_read_byte(&mut self, addr: u16) -> u8 {
        self.bus.mem_read_byte(addr)
    }

    // Writes the data at the given address in memory.
    fn mem_write_byte(&mut self, addr: u16, data: u8) {
        self.bus.mem_write_byte(addr, data)
    }

    // Returns a word from memory, merged from the two bytes at pos and pos + 1.
    fn mem_read_word(&mut self, pos: u16) -> u16 {
        self.bus.mem_read_word(pos)
    }

    // Writes two bytes to memory, split from the data word, as pos and pos + 1.
    fn mem_write_word(&mut self, pos: u16, data: u16) {
        self.bus.mem_write_word(pos, data)
    }
}

impl CPU {
    // Returns an instantiated CPU.
    pub fn new(bus: Bus) -> Self {
        CPU {
            a: 0,
            x: 0,
            y: 0,
            status: STATUS_DEFAULT,
            pc: 0,
            sp: STACK_RESET,
            bus,
        }
    }

    // Resets the CPU and marks where it should begin execution.
    //
    // Emulates the "reset interrupt" signal that is sent to the NES CPU when a
    // cartridge is inserted.
    pub fn reset(&mut self) {
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.sp = STACK_RESET;
        self.status = STATUS_DEFAULT;

        self.pc = self.mem_read_word(0xFFFC);
    }

    // Pops a byte off the stack and increments the stack pointer.
    fn stack_pop_byte(&mut self) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        self.mem_read_byte((STACK as u16) + self.sp as u16)
    }

    // Pushes a byte onto the stack and decrements the stack pointer.
    fn stack_push_byte(&mut self, data: u8) {
        self.mem_write_byte((STACK as u16) + self.sp as u16, data);
        self.sp = self.sp.wrapping_sub(1);
    }

    // Pushes two bytes onto the stack.
    fn stack_push_word(&mut self, data: u16) {
        let bytes = data.to_le_bytes();

        self.stack_push_byte(bytes[1]);
        self.stack_push_byte(bytes[0]);
    }

    // Pops two bytes from the stack and returns a word.
    fn stack_pop_word(&mut self) -> u16 {
        let lo = self.stack_pop_byte();
        let hi = self.stack_pop_byte();

        u16::from_le_bytes([lo, hi])
    }

    // Jumps the program to a point in memory if a given condition is true.
    fn branch(&mut self, condition: bool) {
        if condition {
            let jump: i8 = self.mem_read_byte(self.pc) as i8;
            let jump_addr = self.pc.wrapping_add(1).wrapping_add(jump as u16);

            self.pc = jump_addr;
        }
    }

    // Loads the program into memory.
    //
    // Program ROM starts at 0x8000 for the NES.
    pub fn load(&mut self, program: Vec<u8>) {
        for i in 0..(program.len() as u16) {
            self.mem_write_byte(0x0600 + i, program[i as usize]);
        }
    }

    // Loads the program into memory and runs the CPU.
    pub fn load_and_run(&mut self, program: Vec<u8>) {
        self.load(program);
        self.reset();
        self.pc = 0x0600;
        self.run();
    }

    // Runs the program loaded into memory.
    pub fn run(&mut self) {
        self.run_with_callback(|_| {});
    }

    // Runs the program loaded into memory, and executes the callback function
    // before each opcode iteration.
    pub fn run_with_callback<F>(&mut self, mut callback: F)
    where
        F: FnMut(&mut CPU),
    {
        let ref opcodes: HashMap<u8, &'static instructions::OpCode> = *instructions::OPCODES;

        loop {
            callback(self);

            // Get the opcode at the program counter.
            let code = self.mem_read_byte(self.pc);
            self.pc += 1;
            let current_pc = self.pc;

            // Lookup the full opcode details.
            let opcode = opcodes
                .get(&code)
                .expect(&format!("OpCode {:x} is not recognized", code));

            match opcode.code {
                // Official opcodes.
                0x00 => return,

                // ADC.
                0x69 | 0x65 | 0x75 | 0x6D | 0x7D | 0x79 | 0x61 | 0x71 => {
                    self.adc(&opcode.mode);
                }

                // AND.
                0x29 | 0x25 | 0x35 | 0x2D | 0x3D | 0x39 | 0x21 | 0x31 => {
                    self.and(&opcode.mode);
                }

                // ASL.
                0x0A => self.asl_implied(),
                0x06 | 0x16 | 0x0E | 0x1E => {
                    self.asl(&opcode.mode);
                }

                // BCC.
                0x90 => self.bcc(),

                // BCS.
                0xB0 => self.bcs(),

                // BEQ.
                0xF0 => self.beq(),

                // BIT.
                0x24 | 0x2C => self.bit(&opcode.mode),

                // BMI.
                0x30 => self.bmi(),

                // BNE.
                0xD0 => self.bne(),

                // BPL.
                0x10 => self.bpl(),

                // BVC.
                0x50 => self.bvc(),

                // BVS.
                0x70 => self.bvs(),

                // CLC.
                0x18 => self.clc(),

                // CLD.
                0xD8 => self.cld(),

                // CLI.
                0x58 => self.cli(),

                // CLV.
                0xB8 => self.clv(),

                // CMP.
                0xC9 | 0xC5 | 0xD5 | 0xCD | 0xDD | 0xD9 | 0xC1 | 0xD1 => {
                    self.cmp(&opcode.mode);
                }

                // CMPX.
                0xE0 | 0xE4 | 0xEC => self.cmpx(&opcode.mode),

                // CMPY.
                0xC0 | 0xC4 | 0xCC => self.cmpy(&opcode.mode),

                // DEC.
                0xC6 | 0xD6 | 0xCE | 0xDE => self.dec(&opcode.mode),

                // DECX.
                0xCA => self.decx(),

                // DECY.
                0x88 => self.decy(),

                // EOR.
                0x49 | 0x45 | 0x55 | 0x4D | 0x5D | 0x59 | 0x41 | 0x51 => {
                    self.eor(&opcode.mode);
                }

                // INC.
                0xE6 | 0xF6 | 0xEE | 0xFE => {
                    self.inc(&opcode.mode);
                }

                // INX.
                0xE8 => self.inx(),

                // INY.
                0xC8 => self.iny(),

                // JMP.
                0x4c => {
                    let addr = self.mem_read_word(self.pc);
                    self.pc = addr;
                }
                0x6c => {
                    self.jmp_indirect();
                }

                // JSR.
                0x20 => self.jsr(),

                // LDA.
                0xA9 | 0xA5 | 0xB5 | 0xAD | 0xBD | 0xB9 | 0xA1 | 0xB1 => {
                    self.lda(&opcode.mode);
                }

                // LDX.
                0xA2 | 0xA6 | 0xB6 | 0xAE | 0xBE => self.ldx(&opcode.mode),

                // LDY.
                0xA0 | 0xA4 | 0xB4 | 0xAC | 0xBC => self.ldy(&opcode.mode),

                // LSR.
                0x4A => self.lsr_accumulator(),
                0x46 | 0x56 | 0x4E | 0x5E => {
                    self.lsr(&opcode.mode);
                }

                // NOP.
                0xEA => {}

                // ORA.
                0x09 | 0x05 | 0x15 | 0x0D | 0x1D | 0x19 | 0x01 | 0x11 => {
                    self.ora(&opcode.mode);
                }

                // PHA.
                0x48 => self.pha(),

                // PHP.
                0x08 => self.php(),

                // PLA.
                0x68 => self.pla(),

                // PLP.
                0x28 => self.plp(),

                // ROL.
                0x2A => self.rol_accumulator(),
                0x26 | 0x36 | 0x2E | 0x3E => {
                    self.rol(&opcode.mode);
                }

                // ROR.
                0x6A => self.ror_accumulator(),
                0x66 | 0x76 | 0x6E | 0x7E => {
                    self.ror(&opcode.mode);
                }

                // RTI.
                0x40 => self.rti(),

                // RTS.
                0x60 => self.rts(),

                // SBC.
                0xE9 | 0xE5 | 0xF5 | 0xED | 0xFD | 0xF9 | 0xE1 | 0xF1 => {
                    self.sbc(&opcode.mode);
                }

                // SEC.
                0x38 => self.sec(),

                // SED.
                0xF8 => self.sed(),

                // SEI.
                0x78 => self.sei(),

                // STA.
                0x85 | 0x95 | 0x8D | 0x9D | 0x99 | 0x81 | 0x91 => {
                    self.sta(&opcode.mode);
                }

                // STX.
                0x86 | 0x96 | 0x8E => self.stx(&opcode.mode),

                // STY.
                0x84 | 0x94 | 0x8C => self.sty(&opcode.mode),

                // TAX.
                0xAA => self.tax(),

                // TAY.
                0xA8 => self.tay(),

                // TSX.
                0xBA => self.tsx(),

                // TXA.
                0x8A => self.txa(),

                // TXS.
                0x9A => self.txs(),

                // TYA.
                0x98 => self.tya(),

                // Unofficial/undocumented opcodes.

                // AAR.
                0x6B => self.aar(),

                // ASR.
                0x4B => self.asr(),

                // ANC.
                0x0B | 0x2B => self.anc(),

                // DCP.
                0xC7 | 0xD7 | 0xCF | 0xDF | 0xDB | 0xD3 | 0xC3 => {
                    self.dcp(&opcode.mode);
                }

                // ISB.
                0xe7 | 0xf7 | 0xef | 0xff | 0xfb | 0xe3 | 0xf3 => {
                    self.isb(&opcode.mode);
                }

                // HLT.
                0x02 | 0x12 | 0x22 | 0x32 | 0x42 | 0x52 | 0x62 | 0x72 | 0x92 | 0xB2 | 0xD2
                | 0xF2 => return,

                // LAS.
                0xBB => self.las(&opcode.mode),

                // LAX.
                0xA7 | 0xB7 | 0xAF | 0xBF | 0xA3 | 0xB3 => self.lax(&opcode.mode),

                // LXA.
                0xAB => self.lxa(),

                // NOP (IGN).
                0x04 | 0x44 | 0x64 | 0x14 | 0x34 | 0x54 | 0x74 | 0xD4 | 0xF4 | 0x0C | 0x1C
                | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC => self.ign(&opcode.mode),

                // NOP (unofficial).
                0x1A | 0x3A | 0x5A | 0x7A | 0xDA | 0xFA => {}

                // NOP (SKB).
                0x80 | 0x82 | 0x89 | 0xC2 | 0xE2 => self.skb(),

                // RLA
                0x27 | 0x37 | 0x2F | 0x3F | 0x3B | 0x23 | 0x33 => self.rla(&opcode.mode),

                // RRA.
                0x67 | 0x77 | 0x6F | 0x7F | 0x7B | 0x63 | 0x73 => self.rra(&opcode.mode),

                // SAX.
                0x83 | 0x87 | 0x8F | 0x97 => self.sax(&opcode.mode),

                // SBC (unofficial).
                0xEB => self.sbc(&opcode.mode),

                // SBX.
                0xCB => self.sbx(),

                // SHA.
                0x93 | 0x9F => self.sha(&opcode.mode),

                // SLO.
                0x07 | 0x17 | 0x0F | 0x1F | 0x1B | 0x03 | 0x13 => {
                    self.slo(&opcode.mode);
                }

                // SRE.
                0x47 | 0x57 | 0x4F | 0x5F | 0x5B | 0x43 | 0x53 => {
                    self.sre(&opcode.mode);
                }

                // SHX.
                0x9E => self.shx(&opcode.mode),

                // SHY.
                0x9C => self.shy(&opcode.mode),

                // XAA.
                0x8B => self.xaa(&opcode.mode),

                // TAS.
                0x9B => self.tas(&opcode.mode),

                _ => todo!("{:02x} {}", opcode.code, opcode.mnemonic),
            }

            // Inform the bus the number of CPU cycles for this operation in
            // order for the other components to process as appropriate.
            self.bus.tick(opcode.cycles);

            // Program counter needs to be incremented by the number of bytes
            // used in the opcode, if not done so elsewhere.
            if current_pc == self.pc {
                self.pc += (opcode.len - 1) as u16;
            }
        }
    }
    // Returns the address of the operand for a given non-immediate addressing
    // mode.
    pub fn get_operand_mode_address(&mut self, mode: &AddressingMode, operand: u16) -> u16 {
        match mode {
            AddressingMode::Immediate => operand,

            AddressingMode::ZeroPage => self.mem_read_byte(operand) as u16,

            AddressingMode::Absolute => self.mem_read_word(operand),

            AddressingMode::ZeroPageX => {
                let pos = self.mem_read_byte(operand);
                let addr = pos.wrapping_add(self.x) as u16;
                addr
            }
            AddressingMode::ZeroPageY => {
                let pos = self.mem_read_byte(operand);
                let addr = pos.wrapping_add(self.y) as u16;
                addr
            }

            AddressingMode::AbsoluteX => {
                let base = self.mem_read_word(operand);
                let addr = base.wrapping_add(self.x as u16);
                addr
            }
            AddressingMode::AbsoluteY => {
                let base = self.mem_read_word(operand);
                let addr = base.wrapping_add(self.y as u16);
                addr
            }

            AddressingMode::IndirectX => {
                let base = self.mem_read_byte(operand);

                let ptr: u8 = (base as u8).wrapping_add(self.x);
                let lo = self.mem_read_byte(ptr as u16);
                let hi = self.mem_read_byte(ptr.wrapping_add(1) as u16);

                u16::from_le_bytes([lo, hi])
            }
            AddressingMode::IndirectY => {
                let base = self.mem_read_byte(operand);

                let lo = self.mem_read_byte(base as u16);
                let hi = self.mem_read_byte((base as u8).wrapping_add(1) as u16);

                let deref_base = u16::from_le_bytes([lo, hi]);
                let deref = deref_base.wrapping_add(self.y as u16);
                deref
            }

            AddressingMode::Implied => {
                panic!("mode {:?} is not supported", mode);
            }
        }
    }

    // Returns the address of the operand for a given addressing mode.
    fn get_operand_address(&mut self, mode: &AddressingMode) -> u16 {
        match mode {
            AddressingMode::Immediate => self.pc,
            _ => self.get_operand_mode_address(mode, self.pc),
        }
    }

    // ADC: Add with carry.
    //
    // This instruction adds the contents of a memory location to the
    // accumulator together with the carry bit. If overflow occurs the carry bit
    // is set, this enables multiple byte addition to be performed.
    fn adc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);

        let param = self.mem_read_byte(addr);

        self.add_to_accumulator(param);
    }

    // AND - Logical AND.
    //
    // A logical AND is performed, bit by bit, on the accumulator contents using
    // the contents of a byte of memory.
    fn and(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);

        let param = self.mem_read_byte(addr);

        self.set_accumulator(self.a & param);
    }

    // ASL: Arithmetic Shift Left
    //
    // This operation shifts all the bits of the accumulator contents one bit
    // left. Bit 0 is set to 0 and bit 7 is placed in the carry flag. The effect
    // of this operation is to multiply the memory contents by 2 (ignoring 2's
    // complement considerations), setting the carry if the result will not fit
    // in 8 bits.
    fn asl_implied(&mut self) {
        let mut data = self.a;

        if data >> 7 == 1 {
            self.set_carry_flag();
        } else {
            self.unset_carry_flag();
        }

        data = data << 1;

        self.set_accumulator(data)
    }

    // ASL: Arithmetic Shift Left
    //
    // This operation shifts all the bits of the memory contents one bit left.
    // Bit 0 is set to 0 and bit 7 is placed in the carry flag. The effect of
    // this operation is to multiply the memory contents by 2 (ignoring 2's
    // complement considerations), setting the carry if the result will not fit
    // in 8 bits.
    fn asl(&mut self, mode: &AddressingMode) -> u8 {
        let addr = self.get_operand_address(mode);

        let mut data = self.mem_read_byte(addr);

        if data >> 7 == 1 {
            self.set_carry_flag();
        } else {
            self.unset_carry_flag();
        }

        data = data << 1;
        self.mem_write_byte(addr, data);

        self.update_zero_and_negative_flags(data);

        data
    }

    // BCC: Branch if Carry Clear.
    //
    // If the carry flag is clear then add the relative displacement to the
    // program counter to cause a branch to a new location.
    fn bcc(&mut self) {
        let carry_clear = self.status & 0b00000001 == 0;
        self.branch(carry_clear);
    }

    // BCS: Branch if Carry Set.
    //
    // If the carry flag is set then add the relative displacement to the
    // program counter to cause a branch to a new location.
    fn bcs(&mut self) {
        let carry_set = self.status & 0b00000001 != 0;
        self.branch(carry_set);
    }

    // BEQ: Branch if Equal.
    //
    // If the zero flag is set then add the relative displacement to the program
    // counter to cause a branch to a new location.
    fn beq(&mut self) {
        let zero_set = self.status & 0b00000010 != 0;
        self.branch(zero_set);
    }

    // BIT: Bit Test.
    //
    // This instructions is used to test if one or more bits are set in a target
    // memory location. The mask pattern in A is ANDed with the value in memory
    // to set or clear the zero flag, but the result is not kept. Bits 7 and 6
    // of the value from memory are copied into the N and V flags.
    fn bit(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);

        let param = self.mem_read_byte(addr);

        // Update zero flag.
        if param & self.a == 0 {
            self.status |= 0b00000010;
        } else {
            self.status &= 0b11111101;
        }

        // Copy to negative flag.
        if param & 0b10000000 > 0 {
            self.status |= 0b10000000;
        } else {
            self.status &= 0b01111111;
        }

        // Copy to overflow flag.
        if param & 0b01000000 > 0 {
            self.status |= 0b01000000;
        } else {
            self.status &= 0b10111111;
        }
    }

    // BMI: Branch if Minus.
    //
    // If the negative flag is set then add the relative displacement to the
    // program counter to cause a branch to a new location.
    fn bmi(&mut self) {
        let negative_set = self.status & 0b10000000 != 0;
        self.branch(negative_set);
    }

    // BNE: Branch if Not Equal.
    //
    // If the zero flag is clear then add the relative displacement to the
    // program counter to cause a branch to a new location.
    fn bne(&mut self) {
        let zero_clear = self.status & 0b00000010 == 0;
        self.branch(zero_clear);
    }

    // BPL: Branch if Positive
    //
    // If the negative flag is clear then add the relative displacement to the
    // program counter to cause a branch to a new location.
    fn bpl(&mut self) {
        let negative_clear = self.status & 0b10000000 == 0;
        self.branch(negative_clear);
    }

    // BVC: Branch if Overflow Clear
    //
    // If the overflow flag is clear then add the relative displacement to the
    // program counter to cause a branch to a new location.
    fn bvc(&mut self) {
        let overflow_clear = self.status & 0b01000000 == 0;
        self.branch(overflow_clear);
    }

    // BVS: Branch if Overflow Set
    //
    // If the overflow flag is set then add the relative displacement to the
    // program counter to cause a branch to a new location.
    fn bvs(&mut self) {
        let overflow_set = self.status & 0b01000000 != 0;
        self.branch(overflow_set);
    }

    // CLC: Clear Carry Flag.
    //
    // Set the carry flag to zero.
    fn clc(&mut self) {
        self.unset_carry_flag();
    }

    // CLD: Clear Decimal Mode
    //
    // Sets the decimal mode flag to zero.
    fn cld(&mut self) {
        self.status &= 0b11110111;
    }

    // CLI: Clear Interrupt Disable
    //
    // Clears the interrupt disable flag allowing normal interrupt requests to
    // be serviced.
    fn cli(&mut self) {
        self.status &= 0b11111011;
    }

    // CLV: Clear Overflow Flag
    //
    // Clears the overflow flag.
    fn clv(&mut self) {
        self.status &= 0b10111111;
    }

    // CMP: Compare
    //
    // This instruction compares the contents of the accumulator with another
    // memory held value and sets the zero and carry flags as appropriate.
    fn cmp(&mut self, mode: &AddressingMode) {
        self.compare(mode, self.a);
    }

    // CMPX: Compare X Register
    //
    // This instruction compares the contents of the X register with another
    // memory held value and sets the zero and carry flags as appropriate.
    fn cmpx(&mut self, mode: &AddressingMode) {
        self.compare(mode, self.x);
    }

    // CMPY: Compare Y Register
    //
    // This instruction compares the contents of the Y register with another
    // memory held value and sets the zero and carry flags as appropriate.
    fn cmpy(&mut self, mode: &AddressingMode) {
        self.compare(mode, self.y);
    }

    // DEC: Decrement Memory.
    //
    // Subtracts one from the value held at a specified memory location setting
    // the zero and negative flags as appropriate.
    fn dec(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);

        let param = self.mem_read_byte(addr);

        let result = param.wrapping_sub(1);
        self.mem_write_byte(addr, result);

        self.update_zero_and_negative_flags(result);
    }

    // DECX: Decrement X Register.
    //
    // Subtracts one from the X register setting the zero and negative flags as
    // appropriate.
    fn decx(&mut self) {
        self.x = self.x.wrapping_sub(1);

        self.update_zero_and_negative_flags(self.x);
    }

    // DECY: Decrement Y Register.
    //
    // Subtracts one from the Y register setting the zero and negative flags as
    // appropriate.
    fn decy(&mut self) {
        self.y = self.y.wrapping_sub(1);

        self.update_zero_and_negative_flags(self.y);
    }

    // EOR: Exclusive OR
    //
    // An exclusive OR is performed, bit by bit, on the accumulator contents
    // using the contents of a byte of memory.
    fn eor(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);

        let param = self.mem_read_byte(addr);

        self.set_accumulator(self.a ^ param)
    }

    // INC: Increment Memory
    //
    // Adds one to the value held at a specified memory location setting the
    // zero and negative flags as appropriate.
    fn inc(&mut self, mode: &AddressingMode) -> u8 {
        let addr = self.get_operand_address(mode);

        let param = self.mem_read_byte(addr);

        let result = param.wrapping_add(1);
        self.mem_write_byte(addr, result);

        self.update_zero_and_negative_flags(result);

        result
    }

    // INX: Increment X register.
    //
    // Adds one to the X register setting the zero and negative flags as
    // appropriate.
    fn inx(&mut self) {
        self.x = self.x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.x);
    }

    // INY: Increment Y register.
    //
    // Adds one to the Y register setting the zero and negative flags as
    // appropriate.
    fn iny(&mut self) {
        self.y = self.y.wrapping_add(1);
        self.update_zero_and_negative_flags(self.y);
    }

    // JSR: Jump to Subroutine
    //
    // The JSR instruction pushes the address (minus one) of the return point on
    // to the stack and then sets the program counter to the target memory
    // address.
    fn jsr(&mut self) {
        self.stack_push_word(self.pc + 1);

        let addr = self.mem_read_word(self.pc);

        self.pc = addr;
    }

    // LDA: Load Accumulator.
    //
    // Loads a byte of memory into the accumulator setting the zero and
    // negative flags as appropriate.
    fn lda(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read_byte(addr);

        self.set_accumulator(data);
    }

    // LDX: Load X Register
    //
    // Loads a byte of memory into the X register setting the zero and negative
    // flags as appropriate.
    fn ldx(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);

        let param = self.mem_read_byte(addr);
        self.x = param;

        self.update_zero_and_negative_flags(self.x);
    }

    // LDY: Load Y Register
    //
    // Loads a byte of memory into the Y register setting the zero and negative
    // flags as appropriate.
    fn ldy(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);

        let param = self.mem_read_byte(addr);
        self.y = param;

        self.update_zero_and_negative_flags(self.y);
    }

    // LSR: Logical Shift Right
    //
    // Each of the bits in the accumulator is shifted one place to the right.
    // The bit that was in bit 0 is shifted into the carry flag. Bit 7 is set to
    // zero.
    fn lsr_accumulator(&mut self) {
        let mut data = self.a;

        if data & 0b00000001 == 1 {
            self.set_carry_flag();
        } else {
            self.unset_carry_flag();
        }

        data = data >> 1;

        self.set_accumulator(data);
    }

    // LSR: Logical Shift Right
    //
    // Each of the bits in memory is shifted one place to the right. The bit
    // that was in bit 0 is shifted into the carry flag. Bit 7 is set to zero.
    fn lsr(&mut self, mode: &AddressingMode) -> u8 {
        let addr = self.get_operand_address(mode);

        let mut data = self.mem_read_byte(addr);

        if data & 0b00000001 == 1 {
            self.set_carry_flag();
        } else {
            self.unset_carry_flag();
        }

        data = data >> 1;

        self.mem_write_byte(addr, data);
        self.update_zero_and_negative_flags(data);

        data
    }

    // ORA: Logical Inclusive OR
    //
    // An inclusive OR is performed, bit by bit, on the accumulator contents
    // using the contents of a byte of memory.
    fn ora(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);

        let param = self.mem_read_byte(addr);

        self.set_accumulator(self.a | param)
    }

    // PHA: Push Accumulator
    //
    // Pushes a copy of the accumulator on to the stack.
    fn pha(&mut self) {
        self.stack_push_byte(self.a);
    }

    // PHP: Push Processor Status
    //
    // Pushes a copy of the status flags on to the stack.
    fn php(&mut self) {
        // Set the break flags.
        let mut status = self.status;

        status |= 0b00010000;
        status |= 0b00100000;

        self.stack_push_byte(status);
    }

    // PLA: Pull Accumulator
    //
    // Pulls an 8 bit value from the stack and into the accumulator. The zero
    // and negative flags are set as appropriate.
    fn pla(&mut self) {
        let data = self.stack_pop_byte();
        self.set_accumulator(data);
    }

    // PLP: Pull Processor Status
    //
    // Pulls an 8 bit value from the stack and into the processor flags. The
    // flags will take on new states as determined by the value pulled.
    fn plp(&mut self) {
        let data = self.stack_pop_byte();
        self.status = data;

        // Set the break flags.
        self.status &= 0b11101111;
        self.status |= 0b00100000;
    }

    // ROL: Rotate Left
    //
    // Move each of the bits in the accumulator one place to the left. Bit 0 is
    // filled with the current value of the carry flag whilst the old bit 7
    // becomes the new carry flag value.
    fn rol_accumulator(&mut self) {
        let mut data = self.a;
        let carry_set = self.status & 0b00000001 != 0;

        if data >> 7 == 1 {
            self.set_carry_flag();
        } else {
            self.unset_carry_flag();
        }

        data = data << 1;
        if carry_set {
            data |= 1;
        }

        self.set_accumulator(data);
    }

    // ROL: Rotate Left
    //
    // Move each of the bits in the memory value one place to the left. Bit 0 is
    // filled with the current value of the carry flag whilst the old bit 7
    // becomes the new carry flag value.
    fn rol(&mut self, mode: &AddressingMode) -> u8 {
        let addr = self.get_operand_address(mode);
        let mut data = self.mem_read_byte(addr);

        let carry_set = self.status & 0b00000001 != 0;

        if data >> 7 == 1 {
            self.set_carry_flag();
        } else {
            self.unset_carry_flag();
        }

        data = data << 1;
        if carry_set {
            data |= 0b00000001;
        }

        self.mem_write_byte(addr, data);

        self.update_zero_and_negative_flags(data);

        data
    }

    // ROR: Rotate Right
    //
    // Move each of the bits in the accumulator one place to the right. Bit 7 is
    // filled with the current value of the carry flag whilst the old bit 0
    // becomes the new carry flag value.
    fn ror_accumulator(&mut self) {
        let mut data = self.a;
        let carry_set = self.status & 0b00000001 != 0;

        if data & 0b00000001 == 1 {
            self.set_carry_flag();
        } else {
            self.unset_carry_flag();
        }

        data = data >> 1;
        if carry_set {
            data |= 0b10000000;
        }

        self.set_accumulator(data);
    }

    // ROR: Rotate Right.
    //
    // Move each of the bits in the memory value one place to the right. Bit 7
    // is filled with the current value of the carry flag whilst the old bit 0
    // becomes the new carry flag value.
    fn ror(&mut self, mode: &AddressingMode) -> u8 {
        let addr = self.get_operand_address(mode);
        let mut data = self.mem_read_byte(addr);

        let carry_set = self.status & 0b00000001 != 0;

        if data & 0b00000001 == 1 {
            self.set_carry_flag();
        } else {
            self.unset_carry_flag();
        }

        data = data >> 1;
        if carry_set {
            data |= 0b10000000;
        }

        self.mem_write_byte(addr, data);

        self.update_zero_and_negative_flags(data);

        data
    }

    // RTI: Return from Interrupt
    //
    // The RTI instruction is used at the end of an interrupt processing
    // routine. It pulls the processor flags from the stack followed by the
    // program counter.
    fn rti(&mut self) {
        self.status = self.stack_pop_byte();

        // Set the break flags.
        self.status &= 0b11101111;
        self.status |= 0b00100000;

        self.pc = self.stack_pop_word();
    }

    // RTS: Return from Subroutine
    // The RTS instruction is used at the end of a subroutine to return to the
    // calling routine. It pulls the program counter (minus one) from the stack.
    fn rts(&mut self) {
        self.pc = self.stack_pop_word().wrapping_add(1);
    }

    // SBC: Subtract with Carry
    //
    // This instruction subtracts the contents of a memory location to the
    // accumulator together with the not of the carry bit. If overflow occurs
    // the carry bit is clear, this enables multiple byte subtraction to be
    // performed.
    fn sbc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);

        let param = self.mem_read_byte(addr);

        self.add_to_accumulator(param.wrapping_neg().wrapping_sub(1));
    }

    // SEC: Set Carry Flag.
    //
    // Set the carry flag to one.
    fn sec(&mut self) {
        self.set_carry_flag();
    }

    // SED: Set Decimal Flag.
    //
    // Set the decimal mode flag to one.
    fn sed(&mut self) {
        self.status |= 0b00001000;
    }

    // SEI: Set Interrupt Disable
    //
    // Set the interrupt disable flag to one.
    fn sei(&mut self) {
        self.status |= 0b00000100;
    }

    // STA: Store Accumulator
    //
    // Stores the contents of the accumulator into memory.
    fn sta(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write_byte(addr, self.a)
    }

    // STX: Store X Register
    //
    // Stores the contents of the X register into memory.
    fn stx(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write_byte(addr, self.x)
    }

    // STY: Store Y Register
    //
    // Stores the contents of the Y register into memory.
    fn sty(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write_byte(addr, self.y)
    }

    // TAX: Transfer Accumulator to X.
    //
    // Copies the current contents of the accumulator into the X register and
    // sets the zero and negative flags as appropriate.
    fn tax(&mut self) {
        self.x = self.a;
        self.update_zero_and_negative_flags(self.x);
    }

    // TAY: Transfer Accumulator to Y.
    //
    // Copies the current contents of the accumulator into the Y register and
    // sets the zero and negative flags as appropriate.
    fn tay(&mut self) {
        self.y = self.a;
        self.update_zero_and_negative_flags(self.y);
    }

    // TSX: Transfer Stack Pointer to X
    //
    // Copies the current contents of the stack register into the X register and
    // sets the zero and negative flags as appropriate.
    fn tsx(&mut self) {
        self.x = self.sp;
        self.update_zero_and_negative_flags(self.x);
    }

    // TXA: Transfer X to Accumulator
    //
    // Copies the current contents of the X register into the accumulator and
    // sets the zero and negative flags as appropriate.
    fn txa(&mut self) {
        self.set_accumulator(self.x);
    }

    // TXS: Transfer X to Stack Pointer
    //
    // Copies the current contents of the X register into the stack register.
    fn txs(&mut self) {
        self.sp = self.x;
    }

    // TYA: Transfer Y to Accumulator
    //
    // Copies the current contents of the Y register into the accumulator and
    // sets the zero and negative flags as appropriate.
    fn tya(&mut self) {
        self.set_accumulator(self.y);
    }

    // AAR: AND accumulator rotate.
    //
    // AND byte with accumulator, then rotate one bit right in accumulator.
    fn aar(&mut self) {
        let data = self.mem_read_byte(self.pc);

        self.set_accumulator(data & self.a);
        self.ror_accumulator();

        let acc = self.a;
        let bit_five_set = acc & 0b00010000 != 0;
        let bit_six_set = acc & 0b00100000 != 0;

        // If both bits are 1: set C, clear V.
        // If both bits are 0: clear C and V.
        // If only bit 5 is 1: set V, clear C.
        // If only bit 6 is 1: set C and V.
        if bit_five_set && bit_six_set {
            self.set_carry_flag();
            self.status &= 0b1011111;
        } else if !bit_five_set && !bit_six_set {
            self.unset_carry_flag();
            self.status &= 0b1011111;
        } else if bit_five_set && !bit_six_set {
            self.unset_carry_flag();
            self.status |= 0b01000000;
        } else if !bit_five_set && bit_six_set {
            self.set_carry_flag();
            self.status |= 0b01000000;
        }

        self.update_zero_and_negative_flags(acc);
    }

    // ASR: AND accumulator shift-right.
    //
    // AND byte with accumulator, then shift right one bit in accumulator.
    fn asr(&mut self) {
        let data = self.mem_read_byte(self.pc);

        self.set_accumulator(data & self.a);
        self.lsr_accumulator();
    }

    // ANC: AND
    //
    // AND byte with accumulator. If result is negative then carry is set.
    fn anc(&mut self) {
        let data = self.mem_read_byte(self.pc);
        self.set_accumulator(data & self.a);

        if self.status & 0b10000000 != 0 {
            self.set_carry_flag();
        } else {
            self.unset_carry_flag();
        }
    }

    // DCP: Decrement.
    //
    // Subtract 1 from memory (without borrow).
    fn dcp(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let mut data = self.mem_read_byte(addr);

        data = data.wrapping_sub(1);
        self.mem_write_byte(addr, data);

        if data <= self.a {
            self.set_carry_flag();
        }

        self.update_zero_and_negative_flags(self.a.wrapping_sub(data));
    }

    // IGN: Ignore.
    //
    // Reads from memory at the specified address and ignores the value. Affects
    // no register nor flags
    fn ign(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_read_byte(addr);
    }

    // ISB.
    //
    // Increase memory by one, then subtract memory from accumulator (with
    // borrow).
    fn isb(&mut self, mode: &AddressingMode) {
        let data = self.inc(mode);
        self.add_to_accumulator(((data as i8).wrapping_neg().wrapping_sub(1)) as u8);
    }

    // LAS: AND stack pointer.
    //
    // AND memory with stack pointer, transfer result to accumulator, X register
    // and stack pointer.
    fn las(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let mut data = self.mem_read_byte(addr);

        data &= self.sp;
        self.a = data;
        self.x = data;
        self.sp = data;

        self.update_zero_and_negative_flags(data);
    }

    // LAX: Load accumulator and X register with memory.
    fn lax(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read_byte(addr);

        self.set_accumulator(data);
        self.x = data;
    }

    // LXA: AND accumulator load X.
    //
    // AND byte with accumulator, then transfer accumulator to X register.
    fn lxa(&mut self) {
        let data = self.mem_read_byte(self.pc);
        self.set_accumulator(data & self.a);

        self.tax();
    }

    // SKB: Skip byte.
    //
    // Reads an immediate byte and skips it.
    fn skb(&mut self) {
        self.mem_read_byte(self.pc);
    }

    // RLA: Rotate left AND.
    //
    // Rotate one bit left in memory, then AND accumulator with memory
    fn rla(&mut self, mode: &AddressingMode) {
        let data = self.rol(mode);
        self.set_accumulator(data & self.a);
    }

    // RLA: Rotate right AND.
    //
    // Rotate one bit right in memory, then add memory to accumulator (with
    // carry).
    fn rra(&mut self, mode: &AddressingMode) {
        let data = self.ror(mode);
        self.add_to_accumulator(data);
    }

    // SAX: Store X AND accumulator.
    //
    // AND X register with accumulator and store result in memory.
    fn sax(&mut self, mode: &AddressingMode) {
        let data = self.a & self.x;
        let addr = self.get_operand_address(mode);
        self.mem_write_byte(addr, data);
    }

    // SBX: Subtract X.
    //
    // AND X register with accumulator and store result in X register, then
    // subtract byte from X register (without borrow).
    fn sbx(&mut self) {
        let data = self.mem_read_byte(self.pc);

        let mut result = self.a & self.x;
        result = result.wrapping_sub(data);

        if data <= result {
            self.set_carry_flag();
        }
        self.update_zero_and_negative_flags(result);

        self.x = result;
    }

    // SHA.
    //
    // AND X register with accumulator then AND result with 7 and store in
    // memory.
    fn sha(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);

        let mut data = self.a & self.x;
        data &= 7;

        self.mem_write_byte(addr, data);
    }

    // SLO.
    //
    // Shift left one bit in memory, then OR accumulator with memory.
    fn slo(&mut self, mode: &AddressingMode) {
        let data = self.asl(mode);
        self.set_accumulator(data | self.a);
    }

    // SRE.
    //
    // Shift right one bit in memory, then EOR accumulator with memory.
    fn sre(&mut self, mode: &AddressingMode) {
        let data = self.lsr(mode);
        self.set_accumulator(data ^ self.a);
    }

    // SHX.
    //
    // AND X register with the high byte of the target address of the argument
    // + 1. Store the result in memory.
    fn shx(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let bytes = addr.to_le_bytes();

        let result = self.x & bytes[0].wrapping_add(1);
        self.mem_write_byte(addr, result);
    }

    // SHY.
    //
    // AND Y register with the high byte of the target address of the argument
    // + 1. Store the result in memory.
    fn shy(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let bytes = addr.to_le_bytes();

        let result = self.y & bytes[0].wrapping_add(1);
        self.mem_write_byte(addr, result);
    }

    // XAA.
    //
    // The real 6502 has unpredictable behaviour for this opcode, because it
    // both reads and writes the accumulator (which the 6502 was not designed
    // to do).
    //
    // More or less does A = (A | magic) & X & imm. "magic" defines which bits
    // of A "shine through".
    fn xaa(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read_byte(addr);

        let result = self.a & self.x & data;
        self.set_accumulator(result)
    }

    // TAS.
    //
    // AND X register with accumulator and store result in stack pointer, then
    // AND stack pointer with the high byte of the target address of the
    // argument + 1. Store result in memory.
    fn tas(&mut self, mode: &AddressingMode) {
        self.sp = self.a & self.x;

        let addr = self.get_operand_address(mode);
        let bytes = addr.to_le_bytes();

        let result = bytes[0].wrapping_add(1) & self.sp;
        self.mem_write_byte(addr, result);
    }

    // Adds data to the accumulator and sets the CPU status accordingly.
    fn add_to_accumulator(&mut self, data: u8) {
        let carry = self.status & 0x01;

        let sum = self.a as u16 + data as u16 + carry as u16;

        // Set the carry bit if there is an overflow.
        if sum > 0xFF {
            self.set_carry_flag();
        } else {
            self.unset_carry_flag();
        }

        let result = sum as u8;

        // Set the overflow flag if the sign bit is incorrect.
        if (data ^ result) & (result ^ self.a) & 0x80 != 0 {
            self.status |= 0b01000000;
        } else {
            self.status &= 0b10111111;
        }

        self.set_accumulator(result);
    }

    // Sets the accumulator value and updates the CPU status.
    fn set_accumulator(&mut self, value: u8) {
        self.a = value;
        self.update_zero_and_negative_flags(self.a);
    }

    // Compares the given data with an item read from memory, then sets the
    // appropriate status flags.
    fn compare(&mut self, mode: &AddressingMode, data: u8) {
        let addr = self.get_operand_address(mode);

        let param = self.mem_read_byte(addr);

        if param <= data {
            self.set_carry_flag();
        } else {
            self.unset_carry_flag();
        }

        self.update_zero_and_negative_flags(data.wrapping_sub(param))
    }

    // Sets the Z (zero) and N (negative) flags on the CPU status based on the
    // given result.
    fn update_zero_and_negative_flags(&mut self, result: u8) {
        // Zero flag should be set if the result is 0.
        if result == 0 {
            self.status |= 0b00000010;
        } else {
            self.status &= 0b11111101;
        }

        // Negative flag should be set if bit 7 of the result is set.
        if result >> 7 == 1 {
            self.status |= 0b10000000;
        } else {
            self.status &= 0b01111111;
        }
    }

    // Sets the carry flag on the CPU status.
    fn set_carry_flag(&mut self) {
        self.status |= 0b00000001;
    }

    // Unsets the carry flag on the CPU status.
    fn unset_carry_flag(&mut self) {
        self.status &= 0b11111110;
    }

    // Sets the program counter to an indirect address.
    //
    // An original 6502 has does not correctly fetch the target address if
    // the indirect vector falls on a page boundary (e.g. $xxFF where xx is
    // any value from $00 to $FF). In this case fetches the LSB from $xxFF
    // as expected but takes the MSB from $xx00.
    fn jmp_indirect(&mut self) {
        let addr = self.mem_read_word(self.pc);

        let mut jump_addr = self.mem_read_word(addr);

        // Example:
        //
        // Assume a memory layout of:
        //
        // $3000 = $40
        // $30FF = $80
        // $3100 = $50
        //
        // Expressed as JMP (address), you would expect JMP ($30FF) would first
        // fetch the target address from $30FF (low byte) and $3100 (high byte),
        // then jump to that address ($5080)
        //
        // However, 6502 will fetch the high byte from $3000, resulting in a
        // jump to $4080 instead!
        if addr & 0x00FF == 0x00FF {
            let lo = self.mem_read_byte(addr);
            let hi = self.mem_read_byte(addr & 0xFF00);

            jump_addr = u16::from_le_bytes([lo, hi]);
        }

        self.pc = jump_addr;
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::cartridge::test;
    use crate::cartridge::Rom;
    use crate::trace::trace;
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    #[test]
    fn test_0xa9_lda_immediate_load_data() {
        let bus = Bus::new(test::test_rom());
        let mut cpu = CPU::new(bus);
        cpu.load_and_run(vec![0xa9, 0x05, 0x00]);

        assert_eq!(cpu.a, 0x05);
        assert_eq!(cpu.status & 0b00000010, 0b00);
        assert_eq!(cpu.status & 0b1, 0);
    }

    #[test]
    fn test_0xa9_lda_zero_flag() {
        let bus = Bus::new(test::test_rom());
        let mut cpu = CPU::new(bus);
        cpu.load_and_run(vec![0xa9, 0x00, 0x00]);

        assert_eq!(cpu.status & 0b00000010, 0b10);
    }

    #[test]
    fn test_lda_from_memory() {
        let bus = Bus::new(test::test_rom());
        let mut cpu = CPU::new(bus);
        cpu.mem_write_byte(0x10, 0x55);

        cpu.load_and_run(vec![0xa5, 0x10, 0x00]);

        assert_eq!(cpu.a, 0x55);
    }

    #[test]
    fn test_sta() {
        let bus = Bus::new(test::test_rom());
        let mut cpu = CPU::new(bus);
        cpu.load_and_run(vec![0xa9, 0x05, 0x85, 0x20, 0x00]);

        assert_eq!(cpu.a, 0x05);
        assert_eq!(cpu.mem_read_byte(0x20), 0x05)
    }

    #[test]
    fn test_0xaa_tax_move_a_to_x() {
        let bus = Bus::new(test::test_rom());
        let mut cpu = CPU::new(bus);
        cpu.load(vec![0xaa, 0x00]);
        cpu.reset();
        cpu.pc = 0x0600;
        cpu.a = 10;

        cpu.run();
        assert_eq!(cpu.x, 10)
    }

    #[test]
    fn test_0xe8_inx_increment_x() {
        let bus = Bus::new(test::test_rom());
        let mut cpu = CPU::new(bus);
        cpu.load(vec![0xe8, 0x00]);
        cpu.reset();
        cpu.pc = 0x0600;
        cpu.x = 1;

        cpu.run();
        assert_eq!(cpu.x, 2)
    }

    #[test]
    fn test_inx_overflow() {
        let bus = Bus::new(test::test_rom());
        let mut cpu = CPU::new(bus);
        cpu.load(vec![0xe8, 0xe8, 0x00]);
        cpu.reset();

        cpu.x = 0xff;
        cpu.pc = 0x0600;
        cpu.run();

        assert_eq!(cpu.x, 1)
    }

    #[test]
    fn test_5_ops_working_together() {
        let bus = Bus::new(test::test_rom());
        let mut cpu = CPU::new(bus);
        cpu.load_and_run(vec![0xa9, 0xc0, 0xaa, 0xe8, 0x00]);

        assert_eq!(cpu.x, 0xc1)
    }

    #[test]
    fn test_compare_nestest_rom() {
        // Run test ROM to collect the trace output.
        let bytes: Vec<u8> = std::fs::read("nestest.nes").unwrap();
        let rom = Rom::new(&bytes).unwrap();

        let bus = Bus::new(rom);
        let mut cpu = CPU::new(bus);
        cpu.reset();
        cpu.pc = 0xC000;

        let mut result: Vec<String> = vec![];
        cpu.run_with_callback(|cpu| {
            result.push(trace(cpu));
        });

        // Compare the trace output with the golden output, line-by-line.
        let golden_file = File::open("nestest_no_cycle.log").expect("no such file");
        let reader = BufReader::new(golden_file);

        for (i, line) in reader.lines().enumerate() {
            let line_str = line.expect("could not read line");
            assert_eq!(result[i], line_str);
        }
    }
}
