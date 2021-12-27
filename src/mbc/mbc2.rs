use super::Mbc;
use std::path::Path;
use std::io::{BufWriter, Write};
use std::fs::File;

pub struct Mbc2 {
    rom: Vec<u8>,
    ram: [u8; 0x200],
    ram_enabled: bool,
    rom_bank: u8,
    rom_size: u8,
    has_battery: bool,
}

impl Mbc2 {
    pub fn new(rom: Vec<u8>, ram: &[u8], battery: bool) -> Self {
        let mut mbc = Mbc2 {
            rom: vec![0; 0x4_0000],
            ram: [0; 0x200],
            ram_enabled: false,
            rom_bank: 1,
            rom_size: rom[0x148],
            has_battery: battery,
        };

        if mbc.rom_size > 3 {
            panic!("Mbc2 - Invalid ROM size: {}", mbc.rom_size);
        }

        mbc.rom[0..rom.len()].copy_from_slice(&rom);
        mbc.ram[0..ram.len()].copy_from_slice(ram);

        mbc
    }
}

impl Mbc for Mbc2 {
    fn read(&self, address: usize) -> u8 {
        if address < 0x4000 {
            self.rom[address]
        } else if address < 0x8000 {
            let rom_bank = self.rom_bank % (2 << self.rom_size);
            let rom_address = rom_bank as usize * 0x4000 + (address - 0x4000);
            self.rom[rom_address]
        } else if (0xa000..0xc000).contains(&address) {
            if self.ram_enabled {
                let ram_address = (address - 0xa000) % 0x200;
                self.ram[ram_address] | 0xf0
            } else {
                0xff
            }
        } else {
            panic!("Invalid read from ROM.  Address = {:04x}", address);
        }
    }

    fn write(&mut self, address: usize, value: u8) {
        if address < 0x4000 {
            if address & 0x0100 == 0x0000 {
                self.ram_enabled = value & 0x0f == 0x0a;
            } else {
                self.rom_bank = if value & 0x0f == 0 { 1 } else { value & 0x0f };
            }
        } else if address < 0x8000 {
            // No effect
        } else if (0xa000..0xc000).contains(&address) {
            if self.ram_enabled {
                let ram_address = (address - 0xa000) % 0x200;
                self.ram[ram_address] = value;
            }
        } else {
            panic!("Invalid write to ROM.  Address = {:04x}", address);
        }
    }

    fn save(&self, path: &Path) {
        if self.has_battery {
            let mut buffer = BufWriter::new(File::create(path).expect("Cannot open save file"));
            buffer.write_all(&self.ram).expect("Failed to save");
        }
    }
}
