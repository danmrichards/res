use crate::cpu::AddressingMode;
use crate::cpu::Memory;
use crate::cpu::CPU;
use crate::instructions;
use std::collections::HashMap;

pub fn trace(cpu: &CPU) -> String {
    let ref opcodes: HashMap<u8, &'static instructions::OpCode> = *instructions::OPCODES;

    // Get the current opcode.
    let code = cpu.mem_read_byte(cpu.pc);
    let op = opcodes.get(&code).unwrap();

    let begin = cpu.pc;
    let mut hex_dump = vec![];
    hex_dump.push(code);

    // Get the operands and memory used by the current opcode.
    let (mem_addr, stored_value) = match op.mode {
        AddressingMode::Immediate | AddressingMode::Implied => (0, 0),
        _ => {
            let addr = cpu.get_operand_mode_address(&op.mode, begin + 1);
            (addr, cpu.mem_read_byte(addr))
        }
    };

    // Build an assembly string representation of the operation.
    let asm_op = match op.len {
        1 => match op.code {
            0x0A | 0x4A | 0x2A | 0x6A => format!("A "),
            _ => String::from(""),
        },
        2 => {
            let address: u8 = cpu.mem_read_byte(begin + 1);
            hex_dump.push(address);

            match op.mode {
                AddressingMode::Immediate => format!("#${:02x}", address),
                AddressingMode::ZeroPage => format!("${:02x} = {:02x}", mem_addr, stored_value),
                AddressingMode::ZeroPageX => format!(
                    "${:02x},X @ {:02x} = {:02x}",
                    address, mem_addr, stored_value
                ),
                AddressingMode::ZeroPageY => format!(
                    "${:02x},Y @ {:02x} = {:02x}",
                    address, mem_addr, stored_value
                ),
                AddressingMode::IndirectX => format!(
                    "(${:02x},X) @ {:02x} = {:04x} = {:02x}",
                    address,
                    (address.wrapping_add(cpu.x)),
                    mem_addr,
                    stored_value
                ),
                AddressingMode::IndirectY => format!(
                    "(${:02x}),Y = {:04x} @ {:04x} = {:02x}",
                    address,
                    (mem_addr.wrapping_sub(cpu.y as u16)),
                    mem_addr,
                    stored_value
                ),
                AddressingMode::Implied => {
                    let address: usize =
                        (begin as usize + 2).wrapping_add((address as i8) as usize);
                    format!("${:04x}", address)
                }

                _ => panic!(
                    "unexpected addressing mode {:?} has op-len 2. code {:02x}",
                    op.mode, op.code
                ),
            }
        }
        3 => {
            let address_lo = cpu.mem_read_byte(begin + 1);
            let address_hi = cpu.mem_read_byte(begin + 2);
            hex_dump.push(address_lo);
            hex_dump.push(address_hi);

            let address = cpu.mem_read_word(begin + 1);

            match op.mode {
                AddressingMode::Implied => {
                    if op.code == 0x6c {
                        let jmp_addr = if address & 0x00FF == 0x00FF {
                            let lo = cpu.mem_read_byte(address);
                            let hi = cpu.mem_read_byte(address & 0xFF00);
                            (hi as u16) << 8 | (lo as u16)
                        } else {
                            cpu.mem_read_word(address)
                        };

                        format!("(${:04x}) = {:04x}", address, jmp_addr)
                    } else {
                        format!("${:04x}", address)
                    }
                }
                AddressingMode::Absolute => format!("${:04x} = {:02x}", mem_addr, stored_value),
                AddressingMode::AbsoluteX => format!(
                    "${:04x},X @ {:04x} = {:02x}",
                    address, mem_addr, stored_value
                ),
                AddressingMode::AbsoluteY => format!(
                    "${:04x},Y @ {:04x} = {:02x}",
                    address, mem_addr, stored_value
                ),
                _ => panic!(
                    "unexpected addressing mode {:?} has op-len 3. code {:02x}",
                    op.mode, op.code
                ),
            }
        }
        _ => String::from(""),
    };

    let hex_str = hex_dump
        .iter()
        .map(|z| format!("{:02x}", z))
        .collect::<Vec<String>>()
        .join(" ");
    let asm_str = format!(
        "{:04x}  {:8} {: >4} {}",
        begin, hex_str, op.mnemonic, asm_op
    )
    .trim()
    .to_string();

    format!(
        "{:47} A:{:02x} X:{:02x} Y:{:02x} P:{:02x} SP:{:02x}",
        asm_str, cpu.a, cpu.x, cpu.y, cpu.status, cpu.sp,
    )
    .to_ascii_uppercase()
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::bus::Bus;
    use crate::cartridge::test::test_rom;

    #[test]
    fn test_format_trace() {
        let mut bus = Bus::new(test_rom());
        bus.mem_write_byte(100, 0xA2);
        bus.mem_write_byte(101, 0x01);
        bus.mem_write_byte(102, 0xCA);
        bus.mem_write_byte(103, 0x88);
        bus.mem_write_byte(104, 0x00);

        let mut cpu = CPU::new(bus);
        cpu.pc = 0x64;
        cpu.a = 1;
        cpu.x = 2;
        cpu.y = 3;

        let mut result: Vec<String> = vec![];
        cpu.run_with_callback(|cpu| {
            result.push(trace(cpu));
        });

        assert_eq!(
            "0064  A2 01     LDX #$01                        A:01 X:02 Y:03 P:24 SP:FD",
            result[0]
        );
        assert_eq!(
            "0066  CA        DEX                             A:01 X:01 Y:03 P:24 SP:FD",
            result[1]
        );
        assert_eq!(
            "0067  88        DEY                             A:01 X:00 Y:03 P:26 SP:FD",
            result[2]
        );
    }

    #[test]
    fn test_format_mem_access() {
        let mut bus = Bus::new(test_rom());
        bus.mem_write_byte(100, 0x11);
        bus.mem_write_byte(101, 0x33);
        bus.mem_write_byte(0x33, 00);
        bus.mem_write_byte(0x34, 04);
        bus.mem_write_byte(0x400, 0xAA);

        let mut cpu = CPU::new(bus);
        cpu.pc = 0x64;
        cpu.y = 0;

        let mut result: Vec<String> = vec![];
        cpu.run_with_callback(|cpu| {
            result.push(trace(cpu));
        });

        assert_eq!(
            "0064  11 33     ORA ($33),Y = 0400 @ 0400 = AA  A:00 X:00 Y:00 P:24 SP:FD",
            result[0]
        );
    }
}
