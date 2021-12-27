use super::Mbc;
use std::path::Path;

pub struct NoMbc {
    rom: [u8; 0x8000],
    ram: [u8; 0x2000],
}

impl NoMbc {
    pub fn new(rom: Vec<u8>) -> Self {
        let mut mbc = NoMbc {
            rom: [0; 0x8000],
            ram: [0; 0x2000],
        };

        mbc.rom[0..rom.len()].copy_from_slice(&rom);

        mbc
    }
}

impl Mbc for NoMbc {
    fn read(&self, address: usize) -> u8 {
        if address < 0x8000 {
            self.rom[address]
        } else if (0xa000..0xc000).contains(&address) {
            self.ram[address - 0xa000]
        } else {
            panic!("Invalid read from ROM.  Address = {:04x}", address);
        }
    }

    fn write(&mut self, address: usize, value: u8) {
        if address < 0x8000 {
            // cannot write to ROM
        } else if (0xa000..0xc000).contains(&address) {
            self.ram[address - 0xa000] = value;
        } else {
            panic!("Invalid write to ROM.  Address = {:04x}", address);
        }
    }

    fn save(&self, _path: &Path) {
    }
}

