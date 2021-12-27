use super::Mbc;
use std::path::Path;
use std::io::{BufWriter, Write};
use std::fs::File;

pub struct Mbc1 {
    rom: Vec<u8>,
    ram: [u8; 0x8000],
    ram_enabled: bool,
    bank1: usize,
    bank2: usize,
    mode: bool,
    rom_size: u8,
    ram_size: u8,
    has_battery: bool,
}

impl Mbc1 {
    pub fn new(rom: Vec<u8>, ram: &[u8], battery: bool) -> Self {
        let mut mbc = Mbc1 {
            rom: vec![0; 0x20_0000],
            ram: [0; 0x8000],
            ram_enabled: false,
            bank1: 1,
            bank2: 0,
            mode: false,
            rom_size: rom[0x148],
            ram_size: rom[0x149],
            has_battery: battery,
        };

        mbc.rom[0..rom.len()].copy_from_slice(&rom);
        mbc.ram[0..ram.len()].copy_from_slice(ram);

        mbc
    }
}

impl Mbc for Mbc1 {
    fn read(&self, address: usize) -> u8 {
        if address < 0x4000 {
            if self.mode && self.rom_size > 4 {
                let rom_address = (self.bank2 << 5) * 0x4000 + address;
                self.rom[rom_address]
            } else {
                self.rom[address]
            }
        } else if address < 0x8000 {
            let rom_bank = if self.rom_size > 4 {
                ((self.bank2 << 5) + self.bank1) % (2 << self.rom_size)
            } else {
                self.bank1 % (2 << self.rom_size)
            };
            let rom_address = rom_bank * 0x4000 + (address - 0x4000);
            self.rom[rom_address]
        } else if (0xa000..0xc000).contains(&address) {
            if self.ram_enabled {
                if self.mode && self.ram_size > 2 {
                    let ram_address = self.bank2 * 0x2000 + (address - 0xa000);
                    self.ram[ram_address]
                } else {
                    let ram_address = address - 0xa000;
                    self.ram[ram_address]
                }
            } else {
                0xff
            }
        } else {
            panic!("Invalid read from ROM.  Address = {:04x}", address);
        }
    }

    fn write(&mut self, address: usize, value: u8) {
        if address < 0x2000 {
            self.ram_enabled = value & 0x0f == 0x0a;
        } else if address < 0x4000 {
            self.bank1 = if value & 0x1f == 0 { 1 } else { value & 0x1f } as usize;
        } else if address < 0x6000 {
            self.bank2 = (value as usize) & 0x3;
        } else if address < 0x8000 {
            self.mode = value & 0x1 == 0x1;
        } else if (0xa000..0xc000).contains(&address) {
            if self.ram_enabled {
                if self.mode && self.ram_size > 2 {
                    let ram_address = self.bank2 * 0x2000 + (address - 0xa000);
                    self.ram[ram_address] = value;
                } else {
                    let ram_address = address - 0xa000;
                    self.ram[ram_address] = value;
                }
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_mbc1() {
        let mut rom = vec![0; 0x20_0000];
        for (i, e) in rom.iter_mut().enumerate() {
            *e = (i / 0x4000) as u8;
        }
        rom[0x148] = 6;
        let ram = [0; 0x8000];
        let mut mbc = Mbc1::new(rom, &ram, false);
        mbc.write(0, 0xa); // enable RAM
        assert_eq!(mbc.read(0x0000), 0);
        assert_eq!(mbc.read(0x4000), 1);
        assert_eq!(mbc.read(0x7fff), 1);
        mbc.write(0x2000, 0);
        assert_eq!(mbc.read(0x4000), 1);
        assert_eq!(mbc.read(0x7fff), 1);
        mbc.write(0x2000, 1);
        assert_eq!(mbc.read(0x4000), 1);
        assert_eq!(mbc.read(0x7fff), 1);
        mbc.write(0x2000, 2);
        assert_eq!(mbc.read(0x4000), 2);
        assert_eq!(mbc.read(0x7fff), 2);

        mbc.write(0x4000, 2);
        assert_eq!(mbc.read(0x4000), 0x42);
        assert_eq!(mbc.read(0x7fff), 0x42);

        mbc.write(0x6000, 1);
        assert_eq!(mbc.read(0xa000), 0x00);

        mbc.write(0x6000, 0);
        assert_eq!(mbc.read(0x4000), 0x42);
        assert_eq!(mbc.read(0x7fff), 0x42);

        mbc.write(0x2000, 5);
        assert_eq!(mbc.read(0x4000), 0x45);
        assert_eq!(mbc.read(0x7fff), 0x45);
    }
}
