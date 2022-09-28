use crate::cpu::AddressingMode;
use lazy_static::lazy_static;
use std::collections::HashMap;

// Represents an operation that can be processed by the NES CPU.
pub struct OpCode {
    pub code: u8,
    pub mnemonic: &'static str,
    pub len: u8,
    pub cycles: u8,
    pub mode: AddressingMode,
}

impl OpCode {
    // Returns an instantiated OpCode.
    fn new(code: u8, mnemonic: &'static str, len: u8, cycles: u8, mode: AddressingMode) -> Self {
        OpCode {
            code,
            mnemonic,
            len,
            cycles,
            mode,
        }
    }
}

lazy_static! {
    static ref CPU_OPCODES: Vec<OpCode> = vec![
        OpCode::new(0x69, "ADC", 2, 2, AddressingMode::Immediate),
        OpCode::new(0x65, "ADC", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x75, "ADC", 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0x6D, "ADC", 3, 4, AddressingMode::Absolute),
        OpCode::new(0x7D, "ADC", 3, 4, AddressingMode::AbsoluteX),
        OpCode::new(0x79, "ADC", 3, 4, AddressingMode::AbsoluteY),
        OpCode::new(0x61, "ADC", 2, 6, AddressingMode::IndirectX),
        OpCode::new(0x11, "ADC", 2, 5, AddressingMode::IndirectY),
        OpCode::new(0x29, "AND", 2, 2, AddressingMode::Immediate),
        OpCode::new(0x25, "AND", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x35, "AND", 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0x2D, "AND", 3, 4, AddressingMode::Absolute),
        OpCode::new(0x3D, "AND", 3, 4, AddressingMode::AbsoluteX),
        OpCode::new(0x39, "AND", 3, 4, AddressingMode::AbsoluteY),
        OpCode::new(0x21, "AND", 2, 6, AddressingMode::IndirectX),
        OpCode::new(0x31, "AND", 2, 5, AddressingMode::IndirectY),
        OpCode::new(0x0A, "ASL", 1, 2, AddressingMode::Implied),
        OpCode::new(0x06, "ASL", 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0x16, "ASL", 2, 6, AddressingMode::ZeroPageX),
        OpCode::new(0x0E, "ASL", 3, 6, AddressingMode::Absolute),
        OpCode::new(0x1E, "ASL", 3, 7, AddressingMode::AbsoluteX),
        OpCode::new(0x90, "BCC", 2, 2, AddressingMode::Implied),
        OpCode::new(0xB0, "BCS", 2, 2, AddressingMode::Implied),
        OpCode::new(0xF0, "BEQ", 2, 2, AddressingMode::Implied),
        OpCode::new(0x24, "BIT", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x2C, "BIT", 2, 4, AddressingMode::Absolute),
        OpCode::new(0x30, "BMI", 2, 2, AddressingMode::Implied),
        OpCode::new(0xD0, "BNE", 2, 2, AddressingMode::Implied),
        OpCode::new(0x10, "BPL", 2, 2, AddressingMode::Implied),
        OpCode::new(0x00, "BRK", 1, 7, AddressingMode::Implied),
        OpCode::new(0x50, "BVC", 2, 2, AddressingMode::Implied),
        OpCode::new(0x70, "BVS", 2, 2, AddressingMode::Implied),
        OpCode::new(0x18, "CLC", 1, 2, AddressingMode::Implied),
        OpCode::new(0xD8, "CLD", 1, 2, AddressingMode::Implied),
        OpCode::new(0x58, "CLI", 1, 2, AddressingMode::Implied),
        OpCode::new(0xB8, "CLV", 1, 2, AddressingMode::Implied),
        OpCode::new(0xC9, "CMP", 2, 2, AddressingMode::Immediate),
        OpCode::new(0xC5, "CMP", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xD5, "CMP", 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0xCD, "CMP", 3, 4, AddressingMode::Absolute),
        OpCode::new(0xDD, "CMP", 3, 4, AddressingMode::AbsoluteX),
        OpCode::new(0xD9, "CMP", 3, 4, AddressingMode::AbsoluteY),
        OpCode::new(0xC1, "CMP", 2, 6, AddressingMode::IndirectX),
        OpCode::new(0xD1, "CMP", 2, 5, AddressingMode::IndirectY),
        OpCode::new(0xE0, "CMPX", 2, 2, AddressingMode::Immediate),
        OpCode::new(0xE4, "CMPX", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xEC, "CMPX", 2, 4, AddressingMode::Absolute),
        OpCode::new(0xC0, "CMPY", 2, 2, AddressingMode::Immediate),
        OpCode::new(0xC4, "CMPY", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xCC, "CMPY", 2, 4, AddressingMode::Absolute),
        OpCode::new(0xC6, "DEC", 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0xD6, "DEC", 2, 6, AddressingMode::ZeroPageX),
        OpCode::new(0xCE, "DEC", 3, 6, AddressingMode::Absolute),
        OpCode::new(0xDE, "DEC", 3, 7, AddressingMode::AbsoluteX),
        OpCode::new(0xCA, "DECX", 1, 2, AddressingMode::Implied),
        OpCode::new(0x88, "DECY", 1, 2, AddressingMode::Implied),
        OpCode::new(0xE8, "INX", 1, 2, AddressingMode::Implied),
        OpCode::new(0xA9, "LDA", 2, 2, AddressingMode::Immediate),
        OpCode::new(0xA5, "LDA", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xB5, "LDA", 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0xAD, "LDA", 3, 4, AddressingMode::Absolute),
        OpCode::new(0xBD, "LDA", 3, 4, AddressingMode::AbsoluteX),
        OpCode::new(0xB9, "LDA", 3, 4, AddressingMode::AbsoluteY),
        OpCode::new(0xA1, "LDA", 2, 6, AddressingMode::IndirectX),
        OpCode::new(0xB1, "LDA", 2, 5, AddressingMode::IndirectY),
        OpCode::new(0x85, "STA", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x95, "STA", 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0x8D, "STA", 3, 4, AddressingMode::Absolute),
        OpCode::new(0x9D, "STA", 3, 5, AddressingMode::AbsoluteX),
        OpCode::new(0x99, "STA", 3, 5, AddressingMode::AbsoluteY),
        OpCode::new(0x81, "STA", 2, 6, AddressingMode::IndirectX),
        OpCode::new(0x91, "STA", 2, 6, AddressingMode::IndirectY),
        OpCode::new(0xAA, "TAX", 1, 2, AddressingMode::Implied),
    ];
    pub static ref OPCODES: HashMap<u8, &'static OpCode> = {
        let mut map = HashMap::new();
        for opc in &*CPU_OPCODES {
            map.insert(opc.code, opc);
        }
        map
    };
}
