use super::CPU;
use super::InterruptState;
use super::registers::Flags;

impl<'a> CPU<'a> {
    pub fn ld_a16_sp(&mut self) {
        match self.cycles {
            0 => {
                let mem_read = self.next_byte();
                self.mem_read.push(mem_read);
                self.cycles = 5;
            }
            4 => {
                let mem_read = self.next_byte();
                self.mem_read.push(mem_read);
            }
            3 => {
                let address_msb = self.mem_read[1] as usize;
                let address_lsb = self.mem_read[0] as usize;
                let address = (address_msb << 8) | address_lsb;
                let sp_l = self.sp & 0x00ff;
                self.mmu.write_byte(address, sp_l as u8);
            }
            2 => {
                let address_msb = self.mem_read[1] as usize;
                let address_lsb = self.mem_read[0] as usize;
                let address = (address_msb << 8) | address_lsb;
                let sp_h = (self.sp & 0xff00) >> 8;
                self.mmu.write_byte(address + 1, sp_h as u8);
            }
            _ => {},
        }
    }

    pub fn ld_sp_hl(&mut self) {
        if self.cycles == 0 {
            self.cycles = 2;
            self.sp = self.registers.get_hl() as usize;
        }
    }

    pub fn ldhl_sp_r8(&mut self) {
        match self.cycles {
            0 => {
                self.cycles = 3;
                let mem_read = self.next_byte();
                self.mem_read.push(mem_read);
            }
            2 => {
                let offset = self.mem_read[0];
                let offset16 = if offset & 0x80 != 0 { offset as u16 | 0xff00 } else { offset as u16 };
                let (result, flags) = add_16bit(self.sp as u16, offset16);
                self.registers.set_hl(result);
                self.registers.f = flags;
            }
            _ => {},
        }
    }

    pub fn ld_hli_a(&mut self) {
        if self.cycles == 0 {
            let hl = self.registers.get_hl();
            self.mmu.write_byte(hl as usize, self.registers.a);
            self.registers.set_hl(hl.wrapping_add(1));
            self.cycles = 2;
        }
    }

    pub fn ld_hld_a(&mut self) {
        if self.cycles == 0 {
            let hl = self.registers.get_hl();
            self.mmu.write_byte(hl as usize, self.registers.a);
            self.registers.set_hl(hl.wrapping_sub(1));
            self.cycles = 2;
        }
    }

    pub fn ld_a_hld(&mut self) {
        if self.cycles == 0 {
            let hl = self.registers.get_hl();
            self.registers.a = self.mmu.read_byte(hl as usize);
            self.registers.set_hl(hl.wrapping_sub(1));
            self.cycles = 2;
        }
    }

    pub fn ld_a_hli(&mut self) {
        if self.cycles == 0 {
            let hl = self.registers.get_hl();
            self.registers.a = self.mmu.read_byte(hl as usize);
            self.registers.set_hl(hl.wrapping_add(1));
            self.cycles = 2;
        }
    }

    pub fn ld_ff_c_a(&mut self) {
        if self.cycles == 0 {
            let address = 0xff00 + self.registers.c as usize;
            self.mmu.write_byte(address, self.registers.a);
            self.cycles = 2;
        }
    }

    pub fn ld_a_ff_c(&mut self) {
        if self.cycles == 0 {
            let address = 0xff00 + self.registers.c as usize;
            self.registers.a = self.mmu.read_byte(address);
            self.cycles = 2;
        }
    }

    pub fn ld_a8_a(&mut self) {
        match self.cycles {
            0 => {
                self.cycles = 3;
                let mem_read = self.next_byte();
                self.mem_read.push(mem_read);
            }
            2 => {
                let address = 0xff00 + self.mem_read[0] as usize;
                self.mmu.write_byte(address, self.registers.a);
            }
            _ => {}
        }
    }

    pub fn ld_a_a8(&mut self) {
        match self.cycles {
            0 => {
                self.cycles = 3;
                let mem_read = self.next_byte();
                self.mem_read.push(mem_read);
            }
            2 => {
                let address = 0xff00 + self.mem_read[0] as usize;
                let mem_read = self.mmu.read_byte(address);
                self.mem_read.push(mem_read);
            }
            _ => self.registers.a = self.mem_read[1],
        }
    }

    pub fn ld_reg16_d16(&mut self) {
        match self.cycles {
            0 => {
                let mem_read = self.next_byte();
                self.mem_read.push(mem_read);
                self.cycles = 3;
            }
            2 => {
                let mem_read = self.next_byte();
                self.mem_read.push(mem_read);
            }
            _ => {
                let dest_id = (self.opcode1 & 0x30) >> 4;
                let value = ((self.mem_read[1] as u16) << 8) | (self.mem_read[0] as u16);
                self.write_reg16(dest_id, value);
            }
        }
    }

    pub fn ld_a_bc(&mut self) {
        if self.cycles == 0 {
            let address = self.registers.get_bc() as usize;
            self.registers.a = self.mmu.read_byte(address);
            self.cycles = 2;
        }
    }

    pub fn ld_a_de(&mut self) {
        if self.cycles == 0 {
            let address = self.registers.get_de() as usize;
            self.registers.a = self.mmu.read_byte(address);
            self.cycles = 2;
        }
    }

    pub fn ld_bc_a(&mut self) {
        if self.cycles == 0 {
            let address = self.registers.get_bc() as usize;
            self.mmu.write_byte(address, self.registers.a);
            self.cycles = 2;
        }
    }

    pub fn ld_de_a(&mut self) {
        if self.cycles == 0 {
            let address = self.registers.get_de() as usize;
            self.mmu.write_byte(address, self.registers.a);
            self.cycles = 2;
        }
    }

    pub fn ld_a16_a(&mut self) {
        match self.cycles {
            0 => {
                self.cycles = 4;
                let mem_read = self.next_byte();
                self.mem_read.push(mem_read);
            }
            3 => {
                let mem_read = self.next_byte();
                self.mem_read.push(mem_read);
            }
            2 => {
                let address = (self.mem_read[1] as usize) << 8 | self.mem_read[0] as usize;
                self.mmu.write_byte(address, self.registers.a);
            }
            _ => {}
        }
    }

    pub fn ld_a_a16(&mut self) {
        match self.cycles {
            0 => {
                self.cycles = 4;
                let mem_read = self.next_byte();
                self.mem_read.push(mem_read);
            }
            3 => {
                let mem_read = self.next_byte();
                self.mem_read.push(mem_read);
            }
            2 => {
                let address = (self.mem_read[1] as usize) << 8 | self.mem_read[0] as usize;
                let mem_read = self.mmu.read_byte(address);
                self.mem_read.push(mem_read);
            }
            _ => self.registers.a = self.mem_read[2],
        }
    }

    pub fn ld_hl_d8(&mut self) {
        match self.cycles {
            0 => {
                self.cycles = 3;
                let mem_read = self.next_byte();
                self.mem_read.push(mem_read);
            }
            2 => {
                let address = self.registers.get_hl() as usize;
                self.mmu.write_byte(address, self.mem_read[0]);
            }
            _ => {}
        }
    }

    pub fn ld_reg_d8(&mut self) {
        if self.cycles == 0 {
            let dest_id = (self.opcode1 & 0x38) >> 3;
            let value = self.next_byte();
            self.registers.set_reg8_by_id(dest_id, value);
            self.cycles = 2;
        }
    }

    pub fn ld_reg_reg(&mut self) {
        if self.cycles == 0 {
            let dest_id = (self.opcode1 & 0x38) >> 3;
            let source_id = self.opcode1 & 0x07;

            let value = self.read_reg8(source_id);
            self.write_reg8(dest_id, value);

            self.cycles = if source_id == 6 || dest_id == 6 { 2 } else { 1 };
        }
    }

    ///////////////////////////////////////////
    // Arithmetic Operations

    pub fn add(&mut self) {
        if self.cycles == 0 {
            let reg_id = self.opcode1 & 0x07;
            let value = self.read_reg8(reg_id);

            let (result, flags) = add_8bit(self.registers.a, value, 0);
            self.registers.f = flags;
            self.registers.a = result;
            self.cycles = if reg_id == 6 { 2 } else { 1 };
        }
    }

    pub fn adc(&mut self) {
        if self.cycles == 0 {
            let reg_id = self.opcode1 & 0x07;
            let value = self.read_reg8(reg_id);

            let carry = if self.registers.f.c { 1 } else { 0 };
            let (result, flags) = add_8bit(self.registers.a, value, carry);
            self.registers.f = flags;
            self.registers.a = result;
            self.cycles = if reg_id == 6 { 2 } else { 1 };
        }
    }

    pub fn sub(&mut self) {
        if self.cycles == 0 {
            let reg_id = self.opcode1 & 0x07;
            let value = self.read_reg8(reg_id);

            let (result, flags) = sub_8bit(self.registers.a, value, 0);
            self.registers.f = flags;
            self.registers.a = result;
            self.cycles = if reg_id == 6 { 2 } else { 1 };
        }
    }

    pub fn sbc(&mut self) {
        if self.cycles == 0 {
            let reg_id = self.opcode1 & 0x07;
            let value = self.read_reg8(reg_id);

            let carry = if self.registers.f.c { 1 } else { 0 };
            let (result, flags) = sub_8bit(self.registers.a, value, carry);
            self.registers.f = flags;
            self.registers.a = result;
            self.cycles = if reg_id == 6 { 2 } else { 1 };
        }
    }

    pub fn add_d8(&mut self) {
        if self.cycles == 0 {
            let value = self.next_byte();
            let (result, flags) = add_8bit(self.registers.a, value, 0);
            self.registers.f = flags;
            self.registers.a = result;
            self.cycles = 2;
        }
    }

    pub fn adc_d8(&mut self) {
        if self.cycles == 0 {
            let value = self.next_byte();
            let carry = if self.registers.f.c { 1 } else { 0 };
            let (result, flags) = add_8bit(self.registers.a, value, carry);
            self.registers.f = flags;
            self.registers.a = result;
            self.cycles = 2;
        }
    }

    pub fn sub_d8(&mut self) {
        if self.cycles == 0 {
            let value = self.next_byte();
            let (result, flags) = sub_8bit(self.registers.a, value, 0);
            self.registers.f = flags;
            self.registers.a = result;
            self.cycles = 2;
        }
    }

    pub fn sbc_d8(&mut self) {
        if self.cycles == 0 {
            let value = self.next_byte();
            let carry = if self.registers.f.c { 1 } else { 0 };
            let (result, flags) = sub_8bit(self.registers.a, value, carry);
            self.registers.f = flags;
            self.registers.a = result;
            self.cycles = 2;
        }
    }

    pub fn cp_reg(&mut self) {
        if self.cycles == 0 {
            let reg_id = self.opcode1 & 0x7;
            let value = self.read_reg8(reg_id);

            let (_result, flags) = sub_8bit(self.registers.a, value, 0);
            self.registers.f = flags;
            self.cycles = if reg_id == 6 { 2 } else { 1 };
        }
    }

    pub fn cp_d8(&mut self) {
        if self.cycles == 0 {
            let value = self.next_byte();
            let (_result, flags) = sub_8bit(self.registers.a, value, 0);
            self.registers.f = flags;
            self.cycles = 2;
        }
    }

    pub fn add_hl_reg16(&mut self) {
        if self.cycles == 0 {
            let reg_id = (self.opcode1 & 0x30) >> 4;
            let value = self.read_reg16(reg_id);

            let hl = self.registers.get_hl();
            let mut flags = Flags::default();
            let result32 = hl as u32 + value as u32;
            let result = result32 as u16;
            let result12 = (hl & 0x0fff) + (value & 0x0fff);
            flags.h = result12 > 0x0fff;
            flags.c = result32 > 0xffff;
            self.registers.set_hl(result);

            flags.z = self.registers.f.z;
            self.registers.f = flags;
            self.cycles = 2;
        }
    }

    pub fn add_sp_r8(&mut self) {
        match self.cycles {
            0 => {
                self.cycles = 4;
                let mem_read = self.next_byte();
                self.mem_read.push(mem_read);
            }
            3 => {
                let offset = self.mem_read[0];
                let offset16 = if offset & 0x80 != 0 { offset as u16 | 0xff00 } else { offset as u16 };
                let (result, flags) = add_16bit(self.sp as u16, offset16);
                self.sp = result as usize;
                self.registers.f = flags;
            }
            _ => {}
        }
    }

    // Arithmetic Operations
    ///////////////////////////////////////////


    ///////////////////////////////////////////
    // Logical operations
    pub fn and_reg(&mut self) {
        if self.cycles == 0 {
            let reg_id = self.opcode1 & 0x07;
            let value = self.read_reg8(reg_id);
            self.registers.a &= value;
            self.registers.f.z = self.registers.a == 0;
            self.registers.f.n = false;
            self.registers.f.h = true;
            self.registers.f.c = false;
            self.cycles = if reg_id == 6 { 2 } else { 1 };
        }
    }

    pub fn and_d8(&mut self) {
        if self.cycles == 0 {
            let value = self.next_byte();
            self.registers.a &= value;
            self.registers.f.z = self.registers.a == 0;
            self.registers.f.n = false;
            self.registers.f.h = true;
            self.registers.f.c = false;
            self.cycles = 2;
        }
    }

    pub fn or_reg(&mut self) {
        if self.cycles == 0 {
            let reg_id = self.opcode1 & 0x07;
            let value = self.read_reg8(reg_id);
            self.registers.a |= value;
            self.registers.f.z = self.registers.a == 0;
            self.registers.f.n = false;
            self.registers.f.h = false;
            self.registers.f.c = false;
            self.cycles = if reg_id == 6 { 2 } else { 1 };
        }
    }

    pub fn or_d8(&mut self) {
        if self.cycles == 0 {
            let value = self.next_byte();
            self.registers.a |= value;
            self.registers.f.z = self.registers.a == 0;
            self.registers.f.n = false;
            self.registers.f.h = false;
            self.registers.f.c = false;
            self.cycles = 2;
        }
    }

    pub fn xor_reg(&mut self) {
        if self.cycles == 0 {
            let reg_id = self.opcode1 & 0x07;
            let value = self.read_reg8(reg_id);
            self.registers.a ^= value;
            self.registers.f.z = self.registers.a == 0;
            self.registers.f.n = false;
            self.registers.f.h = false;
            self.registers.f.c = false;
            self.cycles = if reg_id == 6 { 2 } else { 1 };
        }
    }

    pub fn xor_d8(&mut self) {
        if self.cycles == 0 {
            let value = self.next_byte();
            self.registers.a ^= value;
            self.registers.f.z = self.registers.a == 0;
            self.registers.f.n = false;
            self.registers.f.h = false;
            self.registers.f.c = false;
            self.cycles = 2;
        }
    }
    // Logical operations
    ///////////////////////////////////////////

    pub fn push_reg16(&mut self) {
        match self.cycles {
            0 => self.cycles = 4,
            3 => {
                let reg_id = (self.opcode1 & 0x30) >> 4;
                let value = self.registers.get_reg16_by_id(reg_id);
                let value_h = (value & 0xff00) >> 8;
                self.push_stack(value_h as u8);
            }
            2 => {
                let reg_id = (self.opcode1 & 0x30) >> 4;
                let value = self.registers.get_reg16_by_id(reg_id);
                let value_l = value as u8;
                self.push_stack(value_l);
            }
            _ => {}
        }
    }

    pub fn pop_reg16(&mut self) {
        match self.cycles {
            0 => {
                let mem_read = self.pop_stack();
                self.mem_read.push(mem_read);
                self.cycles = 3;
            }
            2 => {
                let mem_read = self.pop_stack();
                self.mem_read.push(mem_read);
            }
            _ => {
                let reg_id = (self.opcode1 & 0x30) >> 4;
                let value_l = self.mem_read[0] as u16;
                let value_h = self.mem_read[1] as u16;
                let value = (value_h << 8) | value_l;
                self.registers.set_reg16_by_id(reg_id, value);
            }
        }
    }

    ///////////////////////////////////////////
    // Call / Return
    pub fn call_a16(&mut self) {
        match self.cycles {
            0 => {
                self.cycles = 6;
                let mem_read = self.next_byte();
                self.mem_read.push(mem_read);
            }
            5 => {
                let mem_read = self.next_byte();
                self.mem_read.push(mem_read);
            }
            4 => {}
            3 => {
                let pc_h = (self.pc & 0xff00) >> 8;
                self.push_stack(pc_h as u8);
            }
            2 => {
                let pc_l = self.pc as u8;
                self.push_stack(pc_l);
            }
            _ => {
                let address = ((self.mem_read[1] as usize) << 8) | self.mem_read[0] as usize;
                self.pc = address;
            }
        }
    }

    pub fn call_cc_a16(&mut self) {
        let condition_id = (self.opcode1 & 0x18) >> 3;
        let condition = self.registers.get_condition_by_id(condition_id);
        if condition {
            self.call_a16();
        } else {
            match self.cycles {
                0 => {
                    self.cycles = 3;
                    let _ = self.next_byte();
                }
                2 => {
                    let _ = self.next_byte();
                }
                _ => {}
            }
        }
    }

    pub fn ret(&mut self) {
        match self.cycles {
            0 => {
                let mem_read = self.pop_stack();
                self.mem_read.push(mem_read);
                self.cycles = 4;
            }
            3 => {
                let mem_read = self.pop_stack();
                self.mem_read.push(mem_read);
            }
            2 => {
                let address_l = self.mem_read[0] as usize;
                let address_h = self.mem_read[1] as usize;
                self.pc = (address_h << 8) | address_l;
            }
            _ => {}
        }
    }

    pub fn ret_cc(&mut self) {
        let condition_id = (self.opcode1 & 0x18) >> 3;
        let condition = self.registers.get_condition_by_id(condition_id);
        if condition {
            // One extra cycle for ret_cc compared to ret
            match self.cycles {
                0 => self.cycles = 5,
                4 => {
                    let mem_read = self.pop_stack();
                    self.mem_read.push(mem_read);
                }
                3 => {
                    let mem_read = self.pop_stack();
                    self.mem_read.push(mem_read);
                }
                2 => {
                    let address_l = self.mem_read[0] as usize;
                    let address_h = self.mem_read[1] as usize;
                    self.pc = (address_h << 8) | address_l;
                }
                _ => {}
            }
        } else if self.cycles == 0 {
            self.cycles = 2;
        }
    }

    pub fn reti(&mut self) {
        self.ret();
        self.mmu.interrupt_controller.state = InterruptState::Enabled;
    }
    // Call / Return
    ///////////////////////////////////////////

    pub fn inc_hl(&mut self) {
        match self.cycles {
            0 => {
                self.cycles = 3;
                let mem_read = self.mmu.read_byte(self.registers.get_hl() as usize);
                self.mem_read.push(mem_read);
            }
            2 => {
                let carry = self.registers.f.c;
                let value = self.mem_read[0];
                let (result, mut flags) = add_8bit(value, 1, 0);
                flags.c = carry;

                self.mmu.write_byte(self.registers.get_hl() as usize, result);
                self.registers.f = flags;
            }
            _ => {}
        }
    }

    pub fn inc_reg8(&mut self) {
        if self.cycles == 0 {
            let reg_id = (self.opcode1 & 0x38) >> 3;
            let carry = self.registers.f.c;
            let value = self.read_reg8(reg_id);
            let (result, mut flags) = add_8bit(value, 1, 0);
            flags.c = carry;

            self.write_reg8(reg_id, result);
            self.registers.f = flags;
            self.cycles = 1;
        }
    }

    pub fn dec_hl(&mut self) {
        match self.cycles {
            0 => {
                self.cycles = 3;
                let mem_read = self.mmu.read_byte(self.registers.get_hl() as usize);
                self.mem_read.push(mem_read);
            }
            2 => {
                let carry = self.registers.f.c;
                let value = self.mem_read[0];
                let (result, mut flags) = sub_8bit(value, 1, 0);
                flags.c = carry;

                self.mmu.write_byte(self.registers.get_hl() as usize, result);
                self.registers.f = flags;
            }
            _ => {}
        }
    }

    pub fn dec_reg8(&mut self) {
        if self.cycles == 0 {
            let reg_id = (self.opcode1 & 0x38) >> 3;
            let carry = self.registers.f.c;
            let value = self.read_reg8(reg_id);
            let (result, mut flags) = sub_8bit(value, 1, 0);
            flags.c = carry;

            self.write_reg8(reg_id, result);
            self.registers.f = flags;
            self.cycles = 1;
        }
    }

    pub fn inc_reg16(&mut self) {
        if self.cycles == 0 {
            let reg_id = (self.opcode1 & 0x30) >> 4;
            let result = self.read_reg16(reg_id).wrapping_add(1);
            self.write_reg16(reg_id, result);
            self.cycles = 2;
        }
    }

    pub fn dec_reg16(&mut self) {
        if self.cycles == 0 {
            let reg_id = (self.opcode1 & 0x30) >> 4;
            let result = self.read_reg16(reg_id).wrapping_sub(1);
            self.write_reg16(reg_id, result);
            self.cycles = 2;
        }
    }

    ///////////////////////////////////////////
    // Rotate

    pub fn rr_hl(&mut self) {
        match self.cycles {
            0 => self.cycles = 4,
            3 => {
                let mem_read = self.mmu.read_byte(self.registers.get_hl() as usize);
                self.mem_read.push(mem_read);
            }
            2 => {
                let value = self.mem_read[0];
                let (result, flags) = rotate_right(value, self.registers.f.c);

                self.mmu.write_byte(self.registers.get_hl() as usize, result);
                self.registers.f = flags;
            }
            _ => {}
        }
    }

    pub fn rr(&mut self) {
        if self.cycles == 0 {
            let reg_id = self.opcode2 & 0x07;
            let value = self.read_reg8(reg_id);
            let (result, flags) = rotate_right(value, self.registers.f.c);
            self.write_reg8(reg_id, result);
            self.registers.f = flags;
            self.cycles = 2;
        }
    }

    pub fn rl_hl(&mut self) {
        match self.cycles {
            0 => self.cycles = 4,
            3 => {
                let mem_read = self.mmu.read_byte(self.registers.get_hl() as usize);
                self.mem_read.push(mem_read);
            }
            2 => {
                let value = self.mem_read[0];
                let (result, flags) = rotate_left(value, self.registers.f.c);

                self.mmu.write_byte(self.registers.get_hl() as usize, result);
                self.registers.f = flags;
            }
            _ => {}
        }
    }

    pub fn rl(&mut self) {
        if self.cycles == 0 {
            let reg_id = self.opcode2 & 0x07;
            let value = self.read_reg8(reg_id);
            let (result, flags) = rotate_left(value, self.registers.f.c);
            self.write_reg8(reg_id, result);
            self.registers.f = flags;
            self.cycles = 2;
        }
    }

    pub fn rlc_hl(&mut self) {
        match self.cycles {
            0 => self.cycles = 4,
            3 => {
                let mem_read = self.mmu.read_byte(self.registers.get_hl() as usize);
                self.mem_read.push(mem_read);
            }
            2 => {
                let value = self.mem_read[0];
                let shift_in = value & 0x80 != 0;
                let (result, flags) = rotate_left(value, shift_in);

                self.mmu.write_byte(self.registers.get_hl() as usize, result);
                self.registers.f = flags;
            }
            _ => {}
        }
    }

    pub fn rlc(&mut self) {
        if self.cycles == 0 {
            let reg_id = self.opcode2 & 0x07;
            let value = self.read_reg8(reg_id);

            let shift_in = value & 0x80 != 0;
            let (result, flags) = rotate_left(value, shift_in);

            self.write_reg8(reg_id, result);
            self.registers.f = flags;
            self.cycles = 2;
        }
    }

    pub fn rrc_hl(&mut self) {
        match self.cycles {
            0 => self.cycles = 4,
            3 => {
                let mem_read = self.mmu.read_byte(self.registers.get_hl() as usize);
                self.mem_read.push(mem_read);
            }
            2 => {
                let value = self.mem_read[0];
                let shift_in = value & 0x01 != 0;
                let (result, flags) = rotate_right(value, shift_in);

                self.mmu.write_byte(self.registers.get_hl() as usize, result);
                self.registers.f = flags;
            }
            _ => {}
        }
    }

    pub fn rrc(&mut self) {
        if self.cycles == 0 {
            let reg_id = self.opcode2 & 0x07;
            let value = self.read_reg8(reg_id);

            let shift_in = value & 0x01 != 0;
            let (result, flags) = rotate_right(value, shift_in);

            self.write_reg8(reg_id, result);
            self.registers.f = flags;
            self.cycles = 2;
        }
    }

    pub fn rl_a(&mut self) {
        let (result, flags) = rotate_left(self.registers.a, self.registers.f.c);
        self.registers.a = result;
        self.registers.f = flags;
        self.registers.f.z = false;
        self.cycles = 1;
    }

    pub fn rlc_a(&mut self) {
        let shift_in = self.registers.a & 0x80 != 0;
        let (result, flags) = rotate_left(self.registers.a, shift_in);
        self.registers.a = result;
        self.registers.f = flags;
        self.registers.f.z = false;
        self.cycles = 1;
    }

    pub fn rr_a(&mut self) {
        let (result, flags) = rotate_right(self.registers.a, self.registers.f.c);
        self.registers.a = result;
        self.registers.f = flags;
        self.registers.f.z = false;
        self.cycles = 1;
    }

    pub fn rrc_a(&mut self) {
        let shift_in = self.registers.a & 0x01 != 0;
        let (result, flags) = rotate_right(self.registers.a, shift_in);
        self.registers.a = result;
        self.registers.f = flags;
        self.registers.f.z = false;
        self.cycles = 1;
    }
    // Rotate
    ///////////////////////////////////////////

    ///////////////////////////////////////////
    // Shift
    pub fn sla_hl(&mut self) {
        match self.cycles {
            0 => self.cycles = 4,
            3 => {
                let mem_read = self.mmu.read_byte(self.registers.get_hl() as usize);
                self.mem_read.push(mem_read);
            }
            2 => {
                let value = self.mem_read[0];
                let result = value << 1;

                self.mmu.write_byte(self.registers.get_hl() as usize, result);
                self.registers.f.z = result == 0;
                self.registers.f.n = false;
                self.registers.f.h = false;
                self.registers.f.c = value & 0x80 == 0x80;
            }
            _ => {}
        }
    }

    pub fn sla(&mut self) {
        if self.cycles == 0 {
            let reg_id = self.opcode2 & 0x07;
            let value = self.read_reg8(reg_id);

            let result = value << 1;
            self.write_reg8(reg_id, result);

            self.registers.f.z = result == 0;
            self.registers.f.n = false;
            self.registers.f.h = false;
            self.registers.f.c = value & 0x80 == 0x80;
            self.cycles = 2;
        }
    }

    pub fn sra_hl(&mut self) {
        match self.cycles {
            0 => self.cycles = 4,
            3 => {
                let mem_read = self.mmu.read_byte(self.registers.get_hl() as usize);
                self.mem_read.push(mem_read);
            }
            2 => {
                let value = self.mem_read[0];
                let msb = value & 0x80;
                let result = (value >> 1) | msb;

                self.mmu.write_byte(self.registers.get_hl() as usize, result);
                self.registers.f.z = result == 0;
                self.registers.f.n = false;
                self.registers.f.h = false;
                self.registers.f.c = value & 0x01 == 0x01;
            }
            _ => {}
        }
    }

    pub fn sra(&mut self) {
        if self.cycles == 0 {
            let reg_id = self.opcode2 & 0x07;
            let value = self.read_reg8(reg_id);

            let msb = value & 0x80;
            let result = (value >> 1) | msb;
            self.write_reg8(reg_id, result);

            self.registers.f.z = result == 0;
            self.registers.f.n = false;
            self.registers.f.h = false;
            self.registers.f.c = value & 0x01 == 0x01;
            self.cycles = 2;
        }
    }

    pub fn srl_hl(&mut self) {
        match self.cycles {
            0 => self.cycles = 4,
            3 => {
                let mem_read = self.mmu.read_byte(self.registers.get_hl() as usize);
                self.mem_read.push(mem_read);
            }
            2 => {
                let value = self.mem_read[0];
                let result = value >> 1;

                self.mmu.write_byte(self.registers.get_hl() as usize, result);
                self.registers.f.z = result == 0;
                self.registers.f.n = false;
                self.registers.f.h = false;
                self.registers.f.c = value & 0x01 == 0x01;
            }
            _ => {}
        }
    }

    pub fn srl(&mut self) {
        if self.cycles == 0 {
            let reg_id = self.opcode2 & 0x07;
            let value = self.read_reg8(reg_id);

            let result = value >> 1;
            self.write_reg8(reg_id, result);

            self.registers.f.z = result == 0;
            self.registers.f.n = false;
            self.registers.f.h = false;
            self.registers.f.c = value & 0x01 == 0x01;
            self.cycles = 2;
        }
    }
    // Shift
    ///////////////////////////////////////////

    pub fn jump_cc_r8(&mut self) {
        if self.cycles == 0 {
            let condition_id = (self.opcode1 & 0x18) >> 3;
            let condition = self.registers.get_condition_by_id(condition_id);
            if condition {
                self.jump_r8();
                self.cycles = 3;
            } else {
                self.next_byte();
                self.cycles = 2;
            }
        }
    }

    pub fn jump_r8(&mut self) {
        if self.cycles == 0 {
            let offset = self.next_byte() as i8;
            self.pc = self.pc.wrapping_add(offset as usize);
            self.cycles = 3;
        }
    }

    pub fn jump_cc_a16(&mut self) {
        let condition_id = (self.opcode1 & 0x18) >> 3;
        let condition = self.registers.get_condition_by_id(condition_id);

        if condition {
            self.jump_a16();
        } else {
            match self.cycles {
                0 => {
                    let _ = self.next_byte();
                    self.cycles = 3;
                }
                2 => {
                    let _ = self.next_byte();
                }
                _ => {}
            }
        }
    }

    pub fn jump_a16(&mut self) {
        match self.cycles {
            0 => {
                let mem_read = self.next_byte();
                self.mem_read.push(mem_read);
                self.cycles = 4;
            }
            3 => {
                let mem_read = self.next_byte();
                self.mem_read.push(mem_read);
            }
            2 => {
                let dest = ((self.mem_read[1] as usize) << 8) | self.mem_read[0] as usize;
                self.pc = dest;
            }
            _ => {}
        }
    }

    pub fn jump_hl(&mut self) {
        self.pc = self.registers.get_hl() as usize;
        self.cycles = 1;
    }

    pub fn enable_interrupts(&mut self) {
        if self.mmu.interrupt_controller.state == InterruptState::Disabled {
            self.mmu.interrupt_controller.state = InterruptState::Scheduled;
        }
        self.cycles = 1;
    }

    pub fn disable_interrupts(&mut self) {
        self.mmu.interrupt_controller.state = InterruptState::Disabled;
        self.cycles = 1;
    }

    pub fn rst(&mut self) {
        if self.cycles == 0 {
            let dest = self.opcode1 as usize & 0x38;
            let pc_h = ((self.pc & 0xff00) >> 8) as u8;
            let pc_l = (self.pc & 0x00ff) as u8;
            self.push_stack(pc_h);
            self.push_stack(pc_l);
            self.pc = dest;
            self.cycles = 4;
        }
    }

    pub fn cpl(&mut self) {
        self.registers.a = !self.registers.a;
        self.registers.f.n = true;
        self.registers.f.h = true;

        self.cycles = 1;
    }

    pub fn scf(&mut self) {
        self.registers.f.c = true;
        self.registers.f.h = false;
        self.registers.f.n = false;
        self.cycles = 1;
    }

    pub fn ccf(&mut self) {
        self.registers.f.c = !self.registers.f.c;
        self.registers.f.h = false;
        self.registers.f.n = false;
        self.cycles = 1;
    }

    pub fn swap_hl(&mut self) {
        match self.cycles {
            0 => self.cycles = 4,
            3 => {
                let mem_read = self.mmu.read_byte(self.registers.get_hl() as usize);
                self.mem_read.push(mem_read);
            }
            2 => {
                let value = self.mem_read[0];
                let temp = (value & 0xf0) >> 4;
                let result = (value << 4) | temp;
                self.mmu.write_byte(self.registers.get_hl() as usize, result);

                self.registers.f.z = result == 0;
                self.registers.f.n = false;
                self.registers.f.h = false;
                self.registers.f.c = false;
            }
            _ => {}
        }
    }

    pub fn swap(&mut self) {
        if self.cycles == 0 {
            let reg_id = self.opcode2 & 0x7;
            let value = self.read_reg8(reg_id);

            let temp = (value & 0xf0) >> 4;
            let result = (value << 4) | temp;
            self.write_reg8(reg_id, result);

            self.registers.f.z = result == 0;
            self.registers.f.n = false;
            self.registers.f.h = false;
            self.registers.f.c = false;
            self.cycles = 2;
        }
    }

    pub fn bit_hl(&mut self) {
        match self.cycles {
            0 => self.cycles = 3,
            2 => {
                let mem_read = self.mmu.read_byte(self.registers.get_hl() as usize);
                self.mem_read.push(mem_read);
            }
            _ => {
                let bit_index = (self.opcode2 & 0x38) >> 3;
                let mask = 0x01 << bit_index;
                let result = self.mem_read[0] & mask;

                self.registers.f.z = result == 0;
                self.registers.f.n = false;
                self.registers.f.h = true;
            }
        }
    }

    pub fn bit(&mut self) {
        if self.cycles == 0 {
            let reg_id = self.opcode2 & 0x07;
            let value = self.read_reg8(reg_id);

            let bit_index = (self.opcode2 & 0x38) >> 3;
            let mask = 0x01 << bit_index;
            let result = value & mask;

            self.registers.f.z = result == 0;
            self.registers.f.n = false;
            self.registers.f.h = true;
            self.cycles = 2;
        }
    }

    pub fn res_hl(&mut self) {
        match self.cycles {
            0 => self.cycles = 4,
            3 => {
                let mem_read = self.mmu.read_byte(self.registers.get_hl() as usize);
                self.mem_read.push(mem_read);
            }
            2 => {
                let bit_index = (self.opcode2 & 0x38) >> 3;
                let mask = !(0x01 << bit_index);
                let result = self.mem_read[0] & mask;

                self.mmu.write_byte(self.registers.get_hl() as usize, result);
            }
            _ => {}
        }
    }

    pub fn res(&mut self) {
        if self.cycles == 0 {
            let reg_id = self.opcode2 & 0x07;
            let value = self.read_reg8(reg_id);

            let bit_index = (self.opcode2 & 0x38) >> 3;
            let mask = !(0x01 << bit_index);
            let result = value & mask;

            self.write_reg8(reg_id, result);
            self.cycles = 2;
        }
    }

    pub fn set_hl(&mut self) {
        match self.cycles {
            0 => self.cycles = 4,
            3 => {
                let mem_read = self.mmu.read_byte(self.registers.get_hl() as usize);
                self.mem_read.push(mem_read);
            }
            2 => {
                let bit_index = (self.opcode2 & 0x38) >> 3;
                let mask = 0x01 << bit_index;
                let result = self.mem_read[0] | mask;

                self.mmu.write_byte(self.registers.get_hl() as usize, result);
            }
            _ => {}
        }
    }
                

    pub fn set(&mut self) {
        if self.cycles == 0 {
            let reg_id = self.opcode2 & 0x07;
            let value = self.read_reg8(reg_id);

            let bit_index = (self.opcode2 & 0x38) >> 3;
            let mask = 0x01 << bit_index;
            let result = value | mask;

            self.write_reg8(reg_id, result);
            self.cycles = 2;
        }
    }

    pub fn daa(&mut self) {
        if !self.registers.f.n {  // after an addition, adjust if (half-)carry occurred or if result is out of bounds
            if self.registers.f.c || self.registers.a > 0x99 { 
                self.registers.a = self.registers.a.wrapping_add(0x60);
                self.registers.f.c = true;
            }
            if self.registers.f.h || (self.registers.a & 0x0f) > 0x09 { 
                self.registers.a = self.registers.a.wrapping_add(0x6);
            }
        } else {  // after a subtraction, only adjust if (half-)carry occurred
            if self.registers.f.c {
                self.registers.a = self.registers.a.wrapping_sub(0x60);
            }
            if self.registers.f.h {
                self.registers.a = self.registers.a.wrapping_sub(0x6);
            }
        }
        
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.h = false;
        self.cycles = 1;
    }

    pub fn halt(&mut self) {
        self.halt = true;
        self.cycles = 1;
    }

    pub fn stop(&mut self) {
        self.mmu.switch_speed();
        self.cycles = 1;
    }
}

fn add_8bit(value1: u8, value2: u8, carry_in: u8) -> (u8, Flags) {
    let mut flags = Flags::default();
    let result16 = value1 as u16 + value2 as u16 + carry_in as u16;
    let result = result16 as u8;
    flags.z = result == 0;
    flags.c = result16 > 0xff;

    let result4 = (value1 & 0x0f) + (value2 & 0x0f) + (carry_in & 0x01);
    flags.h = result4 > 0x0f;

    (result, flags)
}

fn sub_8bit(value1: u8, value2: u8, carry_in: u8) -> (u8, Flags) {
    let mut flags = Flags::default();
    let result = value1.wrapping_sub(value2).wrapping_sub(carry_in);
    flags.z = result == 0;
    flags.n = true;
    flags.h = (value2 & 0x0F) + carry_in > (value1 & 0xF);
    flags.c = (value2 as u16) + (carry_in as u16) > value1 as u16;
    (result, flags)
}

fn add_16bit(value1: u16, value2: u16) -> (u16, Flags) {
    let mut flags = Flags::default();
    let result = value1.wrapping_add(value2);
    flags.h = (value1 & 0x000f) + (value2 & 0x000f) > 0x000f;
    flags.c = (value1 & 0x00ff) + (value2 & 0x00ff) > 0x00ff;
    (result, flags)
}

fn rotate_left(value: u8, shift_in: bool) -> (u8, Flags) {
    let mut flags = Flags::default();
    let mut result = value << 1;
    result |= if shift_in { 0x01 } else { 0x00 };

    flags.z = result == 0;
    flags.c = value & 0x80 == 0x80;
    (result, flags)
}

fn rotate_right(value: u8, shift_in: bool) -> (u8, Flags) {
    let mut flags = Flags::default();
    let mut result = value >> 1;
    result |= if shift_in { 0x80 } else { 0x00 };

    flags.z = result == 0;
    flags.c = value & 0x01 == 0x01;
    (result, flags)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn add() {
        let (result, flags) = add_8bit(0x3a, 0xc6, 0);
        assert_eq!(result, 0);
        assert_eq!(flags.as_u8(), 0b10110000);
        let (result, flags) = add_8bit(0x3c, 0xff, 0);
        assert_eq!(result, 0x3b);
        assert_eq!(flags.as_u8(), 0b00110000);
        let (result, flags) = add_8bit(0x3c, 0x12, 0);
        assert_eq!(result, 0x4e);
        assert_eq!(flags.as_u8(), 0);
    }

    #[test]
    pub fn add_carry() {
        let (result, flags) = add_8bit(0xe1, 0x0f, 1);
        assert_eq!(result, 0xf1);
        assert_eq!(flags.as_u8(), 0b00100000);
        let (result, flags) = add_8bit(0xe1, 0x3b, 1);
        assert_eq!(result, 0x1d);
        assert_eq!(flags.as_u8(), 0b00010000);
        let(result, flags) = add_8bit(0xe1, 0x1e, 1);
        assert_eq!(result, 0x00);
        assert_eq!(flags.as_u8(), 0b10110000);
    }

    #[test]
    pub fn sub() {
        let (result, flags) = sub_8bit(0x3e, 0x3e, 0);
        assert_eq!(result, 0);
        assert_eq!(flags.as_u8(), 0b11000000);
        let (result, flags) = sub_8bit(0x3e, 0x0f, 0);
        assert_eq!(result, 0x2f);
        assert_eq!(flags.as_u8(), 0b01100000);
        let (result, flags) = sub_8bit(0x3e, 0x40, 0);
        assert_eq!(result, 0xfe);
        assert_eq!(flags.as_u8(), 0b01010000);
    }

    #[test]
    pub fn sub_carry() {
        let (result, flags) = sub_8bit(0x3b, 0x2a, 1);
        assert_eq!(result, 0x10);
        assert_eq!(flags.as_u8(), 0b01000000);
        let (result, flags) = sub_8bit(0x3b, 0x3a, 1);
        assert_eq!(result, 0x00);
        assert_eq!(flags.as_u8(), 0b11000000);
        let (result, flags) = sub_8bit(0x3b, 0x4f, 1);
        assert_eq!(result, 0xeb);
        assert_eq!(flags.as_u8(), 0b01110000);
    }

    #[test]
    pub fn add16() {
        let (result, flags) = add_16bit(0x8a23, 0x060f);
        assert_eq!(result, 0x9032);
        assert_eq!(flags.as_u8(), 0x20);
        let (result, flags) = add_16bit(0x8a23, 0x8af3);
        assert_eq!(result, 0x1516);
        assert_eq!(flags.as_u8(), 0x10);
        let (result, flags) = add_16bit(0x8a23, 0x0001);
        assert_eq!(result, 0x8a24);
        assert_eq!(flags.as_u8(), 0x00);
    }

}
