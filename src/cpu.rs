use sdl2::render::TextureCreator;
use sdl2::video::WindowContext;
use super::registers::Registers;
use super::{Display, InterruptState, DEBUG, Mmu};
use std::path::Path;

pub struct Cpu<'a> {
    pub mmu: Mmu<'a>,
    pub pc: usize,
    debug_pc: usize,
    pub sp: usize,
    pub registers: Registers,
    pub cycles: u8,
    pub halt: bool,
    pub mem_read: Vec<u8>,
    pub opcode1: u8,
    pub opcode2: u8,
    pending_interrupt: bool,
    interrupt_dest: usize,
}

impl<'a> Cpu<'a> {
    pub fn new<P: AsRef<Path>>(
        rom_path: P,
        sdl: &sdl2::Sdl,
        display: Display,
        texture_creator: &'a TextureCreator<WindowContext>,
        dmg_mode: bool,
    ) -> Self {
        Cpu {
            mmu: Mmu::new(rom_path, sdl, display, texture_creator, dmg_mode),
            pc: 0,
            debug_pc: 0,
            sp: 0,
            registers: Registers::new(),
            cycles: 1,
            halt: false,
            mem_read: vec![],
            opcode1: 0,
            opcode2: 0,
            pending_interrupt: false,
            interrupt_dest: 0,
        }
    }

    pub fn next_byte(&mut self) -> u8 {
        let byte = self.mmu.read_byte(self.pc);
        self.pc += 1;
        byte
    }

    pub fn write_reg8(&mut self, reg_id: u8, value: u8) {
        if reg_id == 6 {
            self.mmu.write_byte(self.registers.get_hl() as usize, value);
        } else {
            self.registers.set_reg8_by_id(reg_id, value);
        }
    }

    pub fn read_reg8(&self, reg_id: u8) -> u8 {
        if reg_id == 6 {
            self.mmu.read_byte(self.registers.get_hl() as usize)
        } else {
            self.registers.get_reg8_by_id(reg_id)
        }
    }

    pub fn write_reg16(&mut self, reg_id: u8, value: u16) {
        if reg_id == 3 {
            self.sp = value as usize;
        } else {
            self.registers.set_reg16_by_id(reg_id, value);
        }
    }

    pub fn read_reg16(&self, reg_id: u8) -> u16 {
        if reg_id == 3 {
            self.sp as u16
        } else {
            self.registers.get_reg16_by_id(reg_id)
        }
    }

    pub fn push_stack(&mut self, value: u8) {
        self.sp = (self.sp - 1) & 0xffff;
        self.mmu.write_byte(self.sp, value);
    }

    pub fn pop_stack(&mut self) -> u8 {
        let value = self.mmu.read_byte(self.sp);
        self.sp = (self.sp + 1) & 0xffff;
        value
    }

    fn process_interrupts(&mut self) {
        if self.halt && self.mmu.interrupt_controller.state != InterruptState::Enabled {
            self.halt = !self.mmu.interrupt_controller.poll_interrupts();
            self.pending_interrupt = false;
        } else if let Some(dest) = self.mmu.interrupt_controller.pending_interrupts() {
            self.pending_interrupt = true;
            self.halt = false;
            self.interrupt_dest = dest;
        }

        if self.pending_interrupt {
            match self.cycles {
                0 => self.cycles = 5,
                4 => {}
                3 => {
                    let pc_h = ((self.pc & 0xff00) >> 8) as u8;
                    self.push_stack(pc_h);
                }
                2 => {
                    let pc_l = self.pc as u8;
                    self.push_stack(pc_l);
                }
                _ => {
                    self.pc = self.interrupt_dest;
                }
            }
        }
    }

    pub fn execute_cycle(&mut self) {
        if self.mmu.dma_config.cpu_halted(self.mmu.ppu.lcd_status.mode) {
            return;
        }

        if !self.pending_interrupt {
            match self.opcode1 {
                0x00 => self.cycles = 1,
                0x01 | 0x11 | 0x21 | 0x31 => self.ld_reg16_d16(),
                0x02 => self.ld_bc_a(),
                0x03 | 0x13 | 0x23 | 0x33 => self.inc_reg16(),
                0x04 | 0x0C | 0x14 | 0x1C | 0x24 | 0x2C | 0x3C => self.inc_reg8(),
                0x05 | 0x0D | 0x15 | 0x1D | 0x25 | 0x2D | 0x3D => self.dec_reg8(),
                0x06 | 0x0E | 0x16 | 0x1E | 0x26 | 0x2E | 0x3E => self.ld_reg_d8(),
                0x07 => self.rlc_a(),
                0x08 => self.ld_a16_sp(),
                0x09 | 0x19 | 0x29 | 0x39 => self.add_hl_reg16(),
                0x0A => self.ld_a_bc(),
                0x0B | 0x1B | 0x2B | 0x3B => self.dec_reg16(),
                0x0F => self.rrc_a(),
                0x10 => self.stop(),
                0x12 => self.ld_de_a(),
                0x17 => self.rl_a(),
                0x18 => self.jump_r8(),
                0x1A => self.ld_a_de(),
                0x1F => self.rr_a(),
                0x20 | 0x28 | 0x30 | 0x38 => self.jump_cc_r8(),
                0x22 => self.ld_hli_a(),
                0x27 => self.daa(),
                0x2A => self.ld_a_hli(),
                0x2F => self.cpl(),
                0x32 => self.ld_hld_a(),
                0x34 => self.inc_hl(),
                0x35 => self.dec_hl(),
                0x36 => self.ld_hl_d8(),
                0x37 => self.scf(),
                0x3A => self.ld_a_hld(),
                0x3F => self.ccf(),
                0x40..=0x75 => self.ld_reg_reg(),
                0x76 => self.halt(),
                0x77..=0x7F => self.ld_reg_reg(),
                0x80..=0x87 => self.add(),
                0x88..=0x8F => self.adc(),
                0x90..=0x97 => self.sub(),
                0x98..=0x9F => self.sbc(),
                0xA0..=0xA7 => self.and_reg(),
                0xA8..=0xAF => self.xor_reg(),
                0xB0..=0xB7 => self.or_reg(),
                0xB8..=0xBF => self.cp_reg(),
                0xC0 | 0xC8 | 0xD0 | 0xD8 => self.ret_cc(),
                0xC1 | 0xD1 | 0xE1 | 0xF1 => self.pop_reg16(),
                0xC2 | 0xCA | 0xD2 | 0xDA => self.jump_cc_a16(),
                0xC3 => self.jump_a16(),
                0xC4 | 0xCC | 0xD4 | 0xDC => self.call_cc_a16(),
                0xC5 | 0xD5 | 0xE5 | 0xF5 => self.push_reg16(),
                0xC6 => self.add_d8(),
                0xC7 | 0xCF | 0xD7 | 0xDF | 0xE7 | 0xEF | 0xF7 | 0xFF => self.rst(),
                0xC9 => self.ret(),
                0xCD => self.call_a16(),
                0xCE => self.adc_d8(),
                0xD6 => self.sub_d8(),
                0xD9 => self.reti(),
                0xDE => self.sbc_d8(),
                0xE0 => self.ld_a8_a(),
                0xE2 => self.ld_ff_c_a(),
                0xE6 => self.and_d8(),
                0xE8 => self.add_sp_r8(),
                0xE9 => self.jump_hl(),
                0xEA => self.ld_a16_a(),
                0xEE => self.xor_d8(),
                0xF0 => self.ld_a_a8(),
                0xF2 => self.ld_a_ff_c(),
                0xF3 => self.disable_interrupts(),
                0xF6 => self.or_d8(),
                0xF8 => self.ldhl_sp_r8(),
                0xF9 => self.ld_sp_hl(),
                0xFA => self.ld_a_a16(),
                0xFB => self.enable_interrupts(),
                0xFE => self.cp_d8(),
                0xCB => {
                    if self.cycles == 0 {
                        self.opcode2 = self.next_byte();
                    }
                    match self.opcode2 {
                        0x00..=0x05 | 0x07 => self.rlc(),
                        0x06 => self.rlc_hl(),
                        0x08..=0x0D | 0x0F => self.rrc(),
                        0x0E => self.rrc_hl(),
                        0x10..=0x15 | 0x17 => self.rl(),
                        0x16 => self.rl_hl(),
                        0x18..=0x1D | 0x1F => self.rr(),
                        0x1E => self.rr_hl(),
                        0x20..=0x25 | 0x27 => self.sla(),
                        0x26 => self.sla_hl(),
                        0x28..=0x2D | 0x2F => self.sra(),
                        0x2E => self.sra_hl(),
                        0x30..=0x35 | 0x37 => self.swap(),
                        0x36 => self.swap_hl(),
                        0x38..=0x3D | 0x3F => self.srl(),
                        0x3E => self.srl_hl(),
                        0x46 | 0x4E | 0x56 | 0x5E | 0x66 | 0x6E | 0x76 | 0x7E => self.bit_hl(),
                        0x40..=0x7F => self.bit(),
                        0x86 | 0x8E | 0x96 | 0x9E | 0xA6 | 0xAE | 0xB6 | 0xBE => self.res_hl(),
                        0x80..=0xBF => self.res(),
                        0xC6 | 0xCE | 0xD6 | 0xDE | 0xE6 | 0xEE | 0xF6 | 0xFE => self.set_hl(),
                        0xC0..=0xFF => self.set(),
                    }
                }
                _ => panic!("Invalid opcode {:02x}", self.opcode1),
            }
        }

        if DEBUG && !self.halt && self.cycles == 1 && !self.pending_interrupt {
            let debug_opcode = if self.opcode1 == 0xcb {
                (self.opcode1 as u16) << 8 | self.opcode2 as u16
            } else {
                self.opcode1 as u16
            };
            println!(
                "PC: {:04x}, Op: {:02x} SP: {:04x} {:2x}",
                self.debug_pc, debug_opcode, self.sp, self.registers
            );
        }

        self.cycles = self.cycles.saturating_sub(1);

        // Pre-fetch next instruction, or handle pending interrupt
        if self.cycles == 0 {
            self.mem_read = vec![];
            self.pending_interrupt = false;
        }

        if self.cycles == 0 || self.pending_interrupt {
            self.process_interrupts();
            if self.pending_interrupt || self.halt {
                return;
            }
        }

        if self.cycles == 0 {
            self.debug_pc = self.pc;
            self.opcode1 = self.next_byte();
        }
    }
}
