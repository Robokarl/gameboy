use super::interrupts::*;
use super::{Cartridge, Display, Joypad, SerialLink, SoundController, Timer, DEBUG, Ppu};
use sdl2::render::TextureCreator;
use sdl2::video::WindowContext;
use std::path::Path;

const BOOT_ROM_SIZE: usize = 0x900;
const BOOT_ROM_SIZE_MINUS_1: usize = BOOT_ROM_SIZE - 1;

pub struct Mmu<'a> {
    boot_rom: [u8; BOOT_ROM_SIZE],
    pub cartridge: Cartridge,
    wram: [u8; 0x8000],
    hram: [u8; 0x7f],
    disable_boot_rom: bool,
    pub ppu: Ppu<'a>,
    pub sound_controller: SoundController,
    pub interrupt_controller: super::InterruptController,
    pub timer: Timer,
    pub joypad: Joypad,
    serial_link: SerialLink,
    pub dma_config: DmaConfig,
    wram_bank_sel: u8,
    dmg_mode: bool,
    pub double_speed: bool,
    prepare_speed_switch: bool,
}

#[derive(Copy, Clone, PartialEq)]
pub enum DmaType {
    Oam,
    GeneralPurpose,
    Hblank,
}

impl Default for DmaType {
    fn default() -> Self {
        DmaType::Oam
    }
}

#[derive(Copy, Clone, Default)]
pub struct DmaConfig {
    oam_source_address: usize,
    gp_source_address: usize,
    gp_dest_address: usize,
    count: usize,
    hblank_count: usize,
    pub active: bool,
    length: usize,
    pub dma_type: DmaType,
}

impl DmaConfig {
    pub fn cpu_halted(&self, mode: u8) -> bool {
        (self.dma_type == DmaType::GeneralPurpose && self.active)
            || (self.dma_type == DmaType::Hblank
                && self.active
                && mode == 0
                && self.hblank_count > 0)
    }
}

impl<'a> Mmu<'a> {
    pub fn new<P: AsRef<Path>>(
        rom_path: P,
        sdl: &sdl2::Sdl,
        display: Display,
        texture_creator: &'a TextureCreator<WindowContext>,
        dmg_mode: bool,
    ) -> Self {
        let mut boot_rom = [0; BOOT_ROM_SIZE];
        let boot_rom_filename = if dmg_mode {
            "./DMG_ROM.bin"
        } else {
            "./cgb_bios.bin"
        };
        for (i, byte) in std::fs::read(boot_rom_filename).unwrap().iter().enumerate() {
            boot_rom[i] = *byte;
        }

        Mmu {
            boot_rom,
            cartridge: Cartridge::new(rom_path),
            wram: [0; 0x8000],
            hram: [0; 0x7f],
            disable_boot_rom: false,
            ppu: Ppu::new(display, texture_creator, dmg_mode),
            sound_controller: SoundController::new(sdl),
            interrupt_controller: InterruptController::new(),
            timer: Timer::new(),
            joypad: Joypad::new(),
            serial_link: SerialLink::new(),
            dma_config: DmaConfig::default(),
            wram_bank_sel: 0,
            dmg_mode,
            double_speed: false,
            prepare_speed_switch: false,
        }
    }

    fn oam_dma_cycle(&mut self) {
        match self.dma_config.count {
            0xa1 => {
                self.dma_config.count -= 1;
                if DEBUG {
                    println!("DMA: now active");
                }
            }
            0x01..=0xa0 => {
                let source_address =
                    self.dma_config.oam_source_address + 0xa0 - self.dma_config.count;
                let dest_address = 0xa0 - self.dma_config.count;
                let value = self.read_byte(source_address);
                if DEBUG {
                    println!(
                        "OAM DMA: Write {:02x} from address {:04x} to {:04x}",
                        value,
                        source_address,
                        0xfe00 + dest_address
                    )
                };
                self.ppu.sprite_attribute_table[dest_address] = value;
                self.dma_config.count -= 1;
                self.dma_config.active = true;
            }
            _ => {
                if DEBUG && self.dma_config.active {
                    println!("DMA: now inactive");
                }
                self.dma_config.active = false;
            }
        }
    }

    fn gp_dma_cycle(&mut self) {
        if self.dma_config.count == 0 {
            self.dma_config.active = false;
        } else {
            let source_address =
                self.dma_config.gp_source_address + self.dma_config.length - self.dma_config.count;
            let dest_address =
                self.dma_config.gp_dest_address + self.dma_config.length - self.dma_config.count;
            if DEBUG {
                println!(
                    "GP DMA: Cycle {:03x} / {:03x}",
                    self.dma_config.length - self.dma_config.count,
                    self.dma_config.length
                );
            }
            let value = self.read_byte(source_address);
            self.write_byte(dest_address, value);
            self.dma_config.count -= 1;
        }
    }

    fn hblank_dma_cycle(&mut self) {
        if self.dma_config.count == 0 {
            self.dma_config.active = false;
        } else if self.dma_config.cpu_halted(self.ppu.lcd_status.mode) {
            let source_address =
                self.dma_config.gp_source_address + self.dma_config.length - self.dma_config.count;
            let dest_address =
                self.dma_config.gp_dest_address + self.dma_config.length - self.dma_config.count;
            if DEBUG {
                println!(
                    "HBLANK DMA: Cycle {:03x} / {:03x}",
                    self.dma_config.length - self.dma_config.count,
                    self.dma_config.length
                );
            }
            let value = self.read_byte(source_address);
            self.write_byte(dest_address, value);
            self.dma_config.count -= 1;
            self.dma_config.hblank_count -= 1;
        } else if self.ppu.lcd_status.mode != 0 {
            self.dma_config.hblank_count = 16;
        }
    }

    pub fn execute_cycle(&mut self) {
        if self.dma_config.active || self.dma_config.count > 0 {
            match self.dma_config.dma_type {
                DmaType::Oam => self.oam_dma_cycle(),
                DmaType::GeneralPurpose => self.gp_dma_cycle(),
                DmaType::Hblank => self.hblank_dma_cycle(),
            }
        }
    }

    pub fn read_byte(&self, address: usize) -> u8 {
        let result = match address {
            0x100..=0x1ff => self.cartridge.read(address),
            0x0000..=BOOT_ROM_SIZE_MINUS_1 if !self.disable_boot_rom => self.boot_rom[address],
            0x0000..=0x7fff | 0xa000..=0xbfff => self.cartridge.read(address),
            0x8000..=0x9fff => self.ppu.read_vram(address),
            0xc000..=0xcfff => self.wram[address - 0xc000],
            0xd000..=0xdfff => {
                let wram_bank = if self.wram_bank_sel == 0 {
                    1
                } else {
                    self.wram_bank_sel
                };
                let wram_address = wram_bank as usize * 0x1000 + address - 0xd000;
                self.wram[wram_address]
            }
            0xe000..=0xefff => self.wram[address - 0xe000],
            0xf000..=0xfdff => {
                let wram_bank = if self.wram_bank_sel == 0 {
                    1
                } else {
                    self.wram_bank_sel
                };
                let wram_address = wram_bank as usize * 0x1000 + address - 0xf000;
                self.wram[wram_address]
            }
            0xfe00..=0xfeff => {
                if self.dma_config.active && self.dma_config.dma_type == DmaType::Oam {
                    0xff
                } else {
                    self.ppu.read_oam(address)
                }
            }
            0xff00 => self.joypad.read(),
            0xff01 | 0xff02 => self.serial_link.read(address),
            0xff04..=0xff07 => self.timer.read(address),
            0xff0f | 0xffff => self.interrupt_controller.read(address),
            0xff10..=0xff3f => self.sound_controller.read(address),
            0xff40..=0xff45 => self.ppu.read_register(address),
            0xff46 => (self.dma_config.oam_source_address >> 8) as u8,
            0xff47..=0xff4b => self.ppu.read_register(address),
            0xff4d => {
                let mut result = 0x7e;
                if self.double_speed {
                    result |= 0x80
                };
                if self.prepare_speed_switch {
                    result |= 0x01
                };
                result
            }
            0xff4f if !self.dmg_mode => self.ppu.read_register(address),
            0xff51 if !self.dmg_mode => (self.dma_config.gp_source_address >> 8) as u8,
            0xff52 if !self.dmg_mode => self.dma_config.gp_source_address as u8,
            0xff53 if !self.dmg_mode => (self.dma_config.gp_dest_address >> 8) as u8,
            0xff54 if !self.dmg_mode => self.dma_config.gp_dest_address as u8,
            0xff55 if !self.dmg_mode => match self.dma_config.dma_type {
                DmaType::Oam | DmaType::GeneralPurpose => 0xff,
                DmaType::Hblank => {
                    let mut result = ((self.dma_config.count / 0x10) as u8).wrapping_sub(1);
                    if !self.dma_config.active {
                        result |= 0x80;
                    }
                    result
                }
            },
            0xff68..=0xff6c if !self.dmg_mode => self.ppu.read_register(address),
            0xff70 if !self.dmg_mode => self.wram_bank_sel,
            0xff80..=0xfffe => self.hram[address - 0xff80],
            _ => {
                println!("Unmapped read from address {:04x}", address);
                0xff
            }
        };

        if DEBUG {
            println!("Mmu Read: Address: {:04x}, Data: {:02x}", address, result);
        }

        result
    }

    pub fn write_byte(&mut self, address: usize, value: u8) {
        if DEBUG {
            println!("Mmu Write: Address: {:04x}, Data: {:02x}", address, value);
        }

        match address {
            0x0000..=0x7fff | 0xa000..=0xbfff => self.cartridge.write(address, value),
            0x8000..=0x9fff => self.ppu.write_vram(address, value),
            0xc000..=0xcfff => self.wram[address - 0xc000] = value,
            0xd000..=0xdfff => {
                let wram_bank = if self.wram_bank_sel == 0 {
                    1
                } else {
                    self.wram_bank_sel
                };
                let wram_address = wram_bank as usize * 0x1000 + address - 0xd000;
                self.wram[wram_address] = value;
            }
            0xe000..=0xefff => self.wram[address - 0xe000] = value,
            0xf000..=0xfdff => {
                let wram_bank = if self.wram_bank_sel == 0 {
                    1
                } else {
                    self.wram_bank_sel
                };
                let wram_address = wram_bank as usize * 0x1000 + address - 0xf000;
                self.wram[wram_address] = value;
            }
            0xfe00..=0xfe9f => {
                if !self.dma_config.active {
                    self.ppu.write_oam(address, value);
                }
            }
            0xfea0..=0xfeff => {} // unused memory
            0xff00 => self.joypad.write(value),
            0xff01 | 0xff02 => self.serial_link.write(address, value),
            0xff04..=0xff07 => self.timer.write(address, value),
            0xff0f | 0xffff => self.interrupt_controller.write(address, value),
            0xff10..=0xff3f => self.sound_controller.write(address, value),
            0xff40 => self.ppu.lcdc.set_from_u8(value),
            0xff41 => {
                let prev_interrupt_state = self.ppu.interrupt_state();
                self.ppu.write_register(address, value);
                if self.ppu.interrupt_state() && !prev_interrupt_state {
                    self.interrupt_controller.interrupt_flag |= 0x02;
                    if DEBUG {
                        println!("Setting LCD interrupt");
                    }
                }
            }
            0xff42..=0xff45 => self.ppu.write_register(address, value),
            0xff46 => {
                self.dma_config.oam_source_address = (value as usize) << 8;
                self.dma_config.count = 0xa1;
                self.dma_config.dma_type = DmaType::Oam;
            }
            0xff47..=0xff4b => self.ppu.write_register(address, value),
            0xff4c => {
                if value & 0x80 == 0 {
                    self.dmg_mode = true;
                    self.ppu.dmg_compatibility = true;
                }
            }
            0xff4d => self.prepare_speed_switch = value & 0x01 == 0x01,
            0xff4f => self.ppu.write_register(address, value),
            0xff50 => self.disable_boot_rom = true,
            0xff51 if !self.dmg_mode => {
                self.dma_config.gp_source_address =
                    ((value as usize) << 8) | (self.dma_config.gp_source_address & 0xff)
            }
            0xff52 if !self.dmg_mode => {
                self.dma_config.gp_source_address =
                    (self.dma_config.gp_source_address & 0xff00) | value as usize
            }
            0xff53 if !self.dmg_mode => {
                self.dma_config.gp_dest_address = (((value as usize | 0x80) & 0x9f) << 8)
                    | (self.dma_config.gp_dest_address & 0xff)
            }
            0xff54 if !self.dmg_mode => {
                self.dma_config.gp_dest_address =
                    (self.dma_config.gp_dest_address & 0xff00) | (value as usize & 0xf0)
            }
            0xff55 if !self.dmg_mode => {
                if self.dma_config.dma_type == DmaType::Hblank
                    && value & 0x80 == 0
                    && self.dma_config.active
                {
                    self.dma_config.active = false;
                } else {
                    if value & 0x80 == 0x80 {
                        self.dma_config.dma_type = DmaType::Hblank;
                        self.dma_config.hblank_count = 16;
                    } else {
                        self.dma_config.dma_type = DmaType::GeneralPurpose;
                    }
                    self.dma_config.count = (((value as usize) & 0x7f) + 1) * 0x10;
                    self.dma_config.length = (((value as usize) & 0x7f) + 1) * 0x10;
                    self.dma_config.active = true;
                }
            }
            0xff68..=0xff6c => self.ppu.write_register(address, value),
            0xff70 => self.wram_bank_sel = value & 0x07,
            0xff80..=0xfffe => self.hram[address - 0xff80] = value,
            _ => println!(
                "Unmapped write to address {:04x}: data: {:02x}",
                address, value
            ),
        }
    }

    pub fn switch_speed(&mut self) {
        if self.prepare_speed_switch {
            self.double_speed = !self.double_speed;
            self.prepare_speed_switch = false;
        }
    }

    #[allow(dead_code)]
    pub fn dump_rom(&self) {
        println!("dumping boot rom");
        for (address, byte) in self.boot_rom.iter().enumerate() {
            if address % 0x10 == 0 {
                println!();
                print!("{:03x} ", address);
            }
            print!("{:02x} ", *byte);
        }
        println!();
    }
}
