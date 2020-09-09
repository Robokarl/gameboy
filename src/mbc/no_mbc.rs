use super::MBC;
use std::path::PathBuf;

pub struct NoMBC {
    rom: [u8; 0x8000],
    ram: [u8; 0x2000],
}

impl NoMBC {
    pub fn new(rom: Vec<u8>) -> Self {
        let mut mbc = NoMBC {
            rom: [0; 0x8000],
            ram: [0; 0x2000],
        };

        mbc.rom[0..rom.len()].copy_from_slice(&rom);

        mbc
    }
}

impl MBC for NoMBC {
    fn read(&self, address: usize) -> u8 {
        if address < 0x8000 {
            self.rom[address]
        } else if address >= 0xa000 && address < 0xc000 {
            self.ram[address - 0xa000]
        } else {
            panic!("Invalid read from ROM.  Address = {:04x}", address);
        }
    }

    fn write(&mut self, address: usize, value: u8) {
        if address < 0x8000 {
            // cannot write to ROM
        } else if address >= 0xa000 && address < 0xc000 {
            self.ram[address - 0xa000] = value;
        } else {
            panic!("Invalid write to ROM.  Address = {:04x}", address);
        }
    }

    fn save(&self, _path: &PathBuf) {
    }
}

