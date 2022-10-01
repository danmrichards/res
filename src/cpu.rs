use crate::instructions;
use std::collections::HashMap;
use std::ops::Add;

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

trait Memory {
    // Returns the byte at the given address in memory.
    fn mem_read_byte(&self, addr: u16) -> u8;

    // Writes the data at the given address in memory.
    fn mem_write_byte(&mut self, addr: u16, data: u8);

    // Returns a word from memory, merged from the two bytes at pos and pos + 1.
    fn mem_read_word(&self, pos: u16) -> u16 {
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
}

// Stack is located from $0100-$01FF.
const STACK: u16 = 0x0100;
const STACK_RESET: u8 = 0xFD;

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

    // Memory stores the data available to the CPU.
    memory: [u8; 0xFFFF],
}

impl Memory for CPU {
    // Returns the byte at the given address in memory.
    fn mem_read_byte(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    // Writes the data at the given address in memory.
    fn mem_write_byte(&mut self, addr: u16, data: u8) {
        self.memory[addr as usize] = data
    }
}

impl CPU {
    // Returns an instantiated CPU.
    pub fn new() -> Self {
        CPU {
            a: 0,
            x: 0,
            y: 0,
            status: 0,
            pc: 0,
            sp: STACK_RESET,
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
        self.y = 0;
        self.sp = STACK_RESET;
        self.status = 0;

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

        self.stack_push_byte(bytes[0]);
        self.stack_push_byte(bytes[1]);
    }

    // Pops two bytes from the stack and returns a word.
    fn stack_pop_word(&mut self) -> u16 {
        let lo = self.stack_pop_byte();
        let hi = self.stack_pop_byte();

        u16::from_le_bytes([hi, lo])
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
        let ref opcodes: HashMap<u8, &'static instructions::OpCode> = *instructions::OPCODES;

        loop {
            // Get the opcode at the program counter.
            let code = self.mem_read_byte(self.pc);
            self.pc += 1;

            // Lookup the full opcode details.
            let opcode = opcodes
                .get(&code)
                .expect(&format!("OpCode {:x} is not recognized", code));

            match opcode.code {
                0x00 => return,

                // ADC.
                0x69 | 0x65 | 0x75 | 0x6D | 0x7D | 0x79 | 0x61 | 0x11 => {
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
                0x24 | 0x2C => {
                    self.bit(&opcode.mode);
                }

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
                0x17 => self.clc(),

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
                0xE0 | 0xE4 | 0xEC => {
                    self.cmpx(&opcode.mode);
                }

                // CMPY.
                0xC0 | 0xC4 | 0xCC => {
                    self.cmpy(&opcode.mode);
                }

                // DEC.
                0xC6 | 0xD6 | 0xCE | 0xDE => {
                    self.dec(&opcode.mode);
                }

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
                0x4c | 0x6c => {
                    self.jmp(&opcode.mode);
                }

                // JSR.
                0x20 => {
                    self.jsr(&opcode.mode);
                }

                // LDA.
                0xA9 | 0xA5 | 0xB5 | 0xAD | 0xBD | 0xB9 | 0xA1 | 0xB1 => {
                    self.lda(&opcode.mode);
                }

                // LDX.
                0xA2 | 0xA6 | 0xB6 | 0xAE | 0xBE => {
                    self.ldx(&opcode.mode);
                }

                // LDY.
                0xA0 | 0xA4 | 0xB4 | 0xAC | 0xBC => {
                    self.ldy(&opcode.mode);
                }

                // LSR.
                0x4A => self.lsr_accumulator(),
                0x46 | 0x56 | 0x4E | 0x5E => {
                    self.lsr(&opcode.mode);
                }

                // NOP.
                0xEA => {},

                // STA.
                0x85 | 0x95 | 0x8D | 0x9D | 0x99 | 0x81 | 0x91 => {
                    self.sta(&opcode.mode);
                }

                // TAX.
                0xAA => self.tax(),

                _ => todo!(""),
            }

            // Program counter needs to be incremented by the number of bytes
            // used in the opcode.
            self.pc += (opcode.len - 1) as u16;
        }
    }

    // Returns the address of the operand for a given addressing mode.
    fn get_operand_address(&self, mode: &AddressingMode) -> u16 {
        match mode {
            AddressingMode::Immediate => self.pc,

            AddressingMode::ZeroPage => self.mem_read_byte(self.pc) as u16,

            AddressingMode::Absolute => self.mem_read_word(self.pc),

            AddressingMode::ZeroPageX => {
                let pos = self.mem_read_byte(self.pc);
                let addr = pos.wrapping_add(self.x) as u16;
                addr
            }
            AddressingMode::ZeroPageY => {
                let pos = self.mem_read_byte(self.pc);
                let addr = pos.wrapping_add(self.y) as u16;
                addr
            }

            AddressingMode::AbsoluteX => {
                let base = self.mem_read_word(self.pc);
                let addr = base.wrapping_add(self.x as u16);
                addr
            }
            AddressingMode::AbsoluteY => {
                let base = self.mem_read_word(self.pc);
                let addr = base.wrapping_add(self.y as u16);
                addr
            }

            AddressingMode::IndirectX => {
                let base = self.mem_read_byte(self.pc);

                let ptr: u8 = (base as u8).wrapping_add(self.x);
                let lo = self.mem_read_byte(ptr as u16);
                let hi = self.mem_read_byte(ptr.wrapping_add(1) as u16);

                u16::from_le_bytes([hi, lo])
            }
            AddressingMode::IndirectY => {
                let base = self.mem_read_byte(self.pc);

                let lo = self.mem_read_byte(base as u16);
                let hi = self.mem_read_byte((base as u8).wrapping_add(1) as u16);

                let deref_base = u16::from_le_bytes([hi, lo]);
                let deref = deref_base.wrapping_add(self.y as u16);
                deref
            }

            AddressingMode::Implied => {
                panic!("mode {:?} is not supported", mode);
            }
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

        self.set_register_a(self.a & param);
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

        self.set_register_a(data)
    }

    // ASL: Arithmetic Shift Left
    //
    // This operation shifts all the bits of the memory contents one bit left.
    // Bit 0 is set to 0 and bit 7 is placed in the carry flag. The effect of
    // this operation is to multiply the memory contents by 2 (ignoring 2's
    // complement considerations), setting the carry if the result will not fit
    // in 8 bits.
    fn asl(&mut self, mode: &AddressingMode) {
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
            self.status = self.status | 0b00000010;
        } else {
            self.status = self.status & 0b11111101;
        }

        // Copy to negative flag.
        if param & 0b1000000 != 0 {
            self.status = self.status | 0b10000000;
        } else {
            self.status = self.status & 0b01111111;
        }

        // Copy to overflow flag.
        if param & 0b0100000 != 0 {
            self.status = self.status | 0b01000000;
        } else {
            self.status = self.status & 0b10111111;
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
        self.status = self.status & 0b11110111;
    }

    // CLI: Clear Interrupt Disable
    //
    // Clears the interrupt disable flag allowing normal interrupt requests to
    // be serviced.
    fn cli(&mut self) {
        self.status = self.status & 0b11111011;
    }

    // CLV: Clear Overflow Flag
    //
    // Clears the overflow flag.
    fn clv(&mut self) {
        self.status = self.status & 0b10111111;
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

        self.set_register_a(self.a ^ param)
    }

    // INC: Increment Memory
    //
    // Adds one to the value held at a specified memory location setting the
    // zero and negative flags as appropriate.
    fn inc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);

        let param = self.mem_read_byte(addr);

        let result = param.wrapping_add(1);
        self.mem_write_byte(addr, result);

        self.update_zero_and_negative_flags(result);
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

    // JMP: Jump
    //
    // Sets the program counter to the address specified by the operand.
    fn jmp(&mut self, mode: &AddressingMode) {
        match mode {
            // Implied mode has to be handled separately due to a hardware bug
            // on the real 6502. We need to recreate that bug here for absolute
            // compatibility!
            AddressingMode::Implied => self.jump_indirect(),

            _ => {
                self.pc = self.get_operand_address(mode);
            }
        };
    }

    // JSR: Jump to Subroutine
    //
    // The JSR instruction pushes the address (minus one) of the return point on
    // to the stack and then sets the program counter to the target memory
    // address.
    fn jsr(&mut self, mode: &AddressingMode) {
        self.stack_push_word(self.pc + 1);

        let addr = self.get_operand_address(mode);

        self.pc = addr;
    }

    // LDA: Load Accumulator.
    //
    // Loads a byte of memory into the accumulator setting the zero and
    // negative flags as appropriate.
    fn lda(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);

        let param = self.mem_read_byte(addr);
        self.a = param;

        self.update_zero_and_negative_flags(self.a);
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
        let mut data = self.register_a;

        if data & 1 == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }

        data = data >> 1;

        self.set_register_a(data);
    }

    // LSR: Logical Shift Right
    //
    // Each of the bits in memory is shifted one place to the right. The bit
    // that was in bit 0 is shifted into the carry flag. Bit 7 is set to zero.
    fn lsr(&mut self, mode: &AddressingMode){
        let addr = self.get_operand_address(mode);

        let mut data = self.mem_read_byte(addr);

        if data & 1 == 1 {
            self.set_carry_flag();
        } else {
            self.unset_carry_flag();
        }

        data = data >> 1;

        self.mem_write_byte(addr, data);
        self.update_zero_and_negative_flags(data);
    }

    // TAX: Transfer Accumulator to X.
    //
    // Copies the current contents of the accumulator into the X register and
    // sets the zero and negative flags as appropriate.
    fn tax(&mut self) {
        self.x = self.a;
        self.update_zero_and_negative_flags(self.x);
    }

    // Stores the contents of the accumulator into memory.
    fn sta(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write_byte(addr, self.a)
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
            self.status = self.status | 0b01000000;
        } else {
            self.status = self.status & 0b10111111;
        }

        self.set_register_a(result);
    }

    fn set_register_a(&mut self, value: u8) {
        self.a = value;
        self.update_zero_and_negative_flags(self.a);
    }

    // Compares the given data with an item read from memory, then sets the
    // appropriate status flags.
    fn compare(&mut self, mode: &AddressingMode, data: u8) {
        let addr = self.get_operand_address(mode);

        let param = self.mem_read_byte(addr);

        if data >= param {
            self.set_carry_flag();
        }

        self.update_zero_and_negative_flags(data.wrapping_sub(param))
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
        if result & 0b10000000 != 0 {
            self.status = self.status | 0b10000000;
        } else {
            self.status = self.status & 0b01111111;
        }
    }

    // Sets the carry flag on the CPU status.
    fn set_carry_flag(&mut self) {
        self.status = self.status | 0b00000001;
    }

    // Unsets the carry flag on the CPU status.
    fn unset_carry_flag(&mut self) {
        self.status = self.status & 0b11111110;
    }

    // Sets the program counter to an indirect address.
    //
    // An original 6502 has does not correctly fetch the target address if
    // the indirect vector falls on a page boundary (e.g. $xxFF where xx is
    // any value from $00 to $FF). In this case fetches the LSB from $xxFF
    // as expected but takes the MSB from $xx00.
    fn jump_indirect(&mut self) {
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

            jump_addr = u16::from_le_bytes([hi, lo]);
        }

        self.pc = jump_addr;
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
    fn test_lda_from_memory() {
        let mut cpu = CPU::new();
        cpu.mem_write_byte(0x10, 0x55);

        cpu.load_and_run(vec![0xa5, 0x10, 0x00]);

        assert_eq!(cpu.a, 0x55);
    }

    #[test]
    fn test_sta() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x05, 0x85, 0x20, 0x00]);

        assert_eq!(cpu.a, 0x05);
        assert_eq!(cpu.mem_read_byte(0x20), 0x05)
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
