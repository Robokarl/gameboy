use super::MBC;
use std::path::PathBuf;
use std::io::{BufWriter, Write};
use std::fs::File;

pub struct MBC5 {
    rom: Vec<u8>,
    ram: [u8; 0x2_0000],
    ram_enabled: bool,
    rom_bank_l: u8,
    rom_bank_h: u8,
    ram_bank: u8,
    rom_size: u8,
    has_battery: bool,
}

impl MBC5 {
    pub fn new(rom: Vec<u8>, ram: &[u8], battery: bool) -> Self {
        let mut mbc = MBC5 {
            rom: vec![0; 0x80_0000],
            ram: [0; 0x2_0000],
            ram_enabled: false,
            rom_bank_l: 1,
            rom_bank_h: 0,
            ram_bank: 0,
            rom_size: rom[0x148],
            has_battery: battery,
        };

        mbc.rom[0..rom.len()].copy_from_slice(&rom);
        mbc.ram[0..ram.len()].copy_from_slice(&ram);

        mbc
    }
}

impl MBC for MBC5 {
    fn write(&mut self, address: usize, value: u8) {
        match address {
            0x0000..=0x1fff => self.ram_enabled = value == 0x0a,
            0x2000..=0x2fff => self.rom_bank_l = value,
            0x3000..=0x3fff => self.rom_bank_h = value & 0x01,
            0x4000..=0x5fff => self.ram_bank = value & 0x0f,
            0x6000..=0x7fff => {}  // No function
            0xa000..=0xbfff => {
                if self.ram_enabled {
                    let ram_address = (self.ram_bank as usize) * 0x2000 + address - 0xa000;
                    self.ram[ram_address] = value;
                }
            }
            _ => panic!("Invalid ROM write, address: {:04x}, data: {:02x}", address, value),
        }
    }
    
    fn read(&self, address: usize) -> u8 {
        match address {
            0x0000..=0x3fff => self.rom[address],
            0x4000..=0x7fff => {
                let rom_bank = ((self.rom_bank_h as usize) << 8 | self.rom_bank_l as usize) % (2 << self.rom_size as usize);
                let rom_address = (rom_bank * 0x4000) + address - 0x4000;
                self.rom[rom_address]
            }
            0xa000..=0xbfff => {
                if self.ram_enabled {
                    let ram_address = (self.ram_bank as usize) * 0x2000 + address - 0xa000;
                    self.ram[ram_address]
                } else {
                    0xff
                }
            }
            _ => panic!("Invalid ROM read, address: {:04x}", address),
        }
    }

    fn save(&self, path: &PathBuf) {
        if self.has_battery {
            let mut buffer = BufWriter::new(File::create(path).expect("Cannot open save file"));
            buffer.write_all(&self.ram).expect("Failed to save");
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_mbc5() {
        let mut rom = vec![0; 0x4000];
        rom[0x148] = 0x1b;
        let mut ram = [0; 0x2_0000];
        for (i, byte) in ram.iter_mut().enumerate() {
            *byte = (i / 0x2000) as u8;
        }
        let mut mbc = MBC5::new(rom, &ram);
        for bank in 0x00..0x10 {
            mbc.write(0x4000, bank);
            assert_eq!(mbc.read(0xa000), 0xff);
        }

        mbc.write(0, 0x0a);
        mbc.write(0x4000, 0);
        assert_eq!(mbc.read(0xa000), 0x00);
        
        for bank in 0x00..0x10 {
            mbc.write(0x4000, bank);
            assert_eq!(mbc.read(0xa000), bank);
        }
    }
}
