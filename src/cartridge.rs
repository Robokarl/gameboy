use super::mbc::*;
use std::path::{Path, PathBuf};

pub struct Cartridge {
    mbc: Box<dyn Mbc>,
    save_path: PathBuf,
}


impl Cartridge {

    pub fn new<P: AsRef<Path>>(rom_path: P) -> Self {
        let mut save_path = (*rom_path.as_ref()).to_path_buf();
        let mut save_extension = save_path.extension().unwrap().to_owned();
        save_extension.push("save");
        save_path.set_extension(save_extension);

        let rom = std::fs::read(rom_path).unwrap();
        let mut battery = false;
        let mut rtc = false;
        let mut load_data = vec![];
        
        let mbc: Box<dyn Mbc> = match rom[0x147] {
            0x0 | 0x8 | 0x9 => Box::new(NoMbc::new(rom)),
            0x1 | 0x2 => Box::new(Mbc1::new(rom, &load_data, battery)),
            0x3 => {
                load_data = load_save_file(&save_path, 0x8000);
                battery = true;
                Box::new(Mbc1::new(rom, &load_data, battery))
            }
            0x5 => Box::new(Mbc2::new(rom, &load_data, battery)),
            0x6 => {
                load_data = load_save_file(&save_path, 0x200);
                battery = true;
                Box::new(Mbc2::new(rom, &load_data, battery))
            }
            0x0f => {
                load_data = load_save_file(&save_path, 0x8000);
                rtc = true;
                Box::new(Mbc3::new(rom, &load_data, battery, rtc))
            }
            0x10 => {
                load_data = load_save_file(&save_path, 0x8000);
                battery = true;
                rtc = true;
                Box::new(Mbc3::new(rom, &load_data, battery, rtc))
            }
            0x11 | 0x12 => Box::new(Mbc3::new(rom, &load_data, battery, rtc)),
            0x13 => {
                load_data = load_save_file(&save_path, 0x8000);
                battery = true;
                Box::new(Mbc3::new(rom, &load_data, battery, rtc))
            }
            0x19 | 0x1a | 0x1c | 0x1d => Box::new(Mbc5::new(rom, &load_data, battery)),
            0x1b | 0x1e => {
                load_data = load_save_file(&save_path, 0x2_0000);
                battery = true;
                Box::new(Mbc5::new(rom, &load_data, battery))
            }
            _ => unimplemented!("Unsupported Cartridge Type: {:02x}", rom[0x147]),
        };
        
        Cartridge {
            mbc,
            save_path,
        }
    }

    pub fn save(&mut self) {
        self.mbc.save(&self.save_path);
    }

    pub fn read(&self, address: usize) -> u8 {
        self.mbc.read(address)
    }

    pub fn write(&mut self, address: usize, value: u8) {
        self.mbc.write(address, value);
    }

    pub fn update_rtc(&mut self, millis: u64) {
        self.mbc.update_rtc(millis);
    }
}

fn load_save_file<P: AsRef<Path>>(save_path: P, size: usize) -> Vec<u8> {
    let mut load_data = Vec::with_capacity(size);
    if let Ok(save_data) = std::fs::read(save_path) {
        for byte in save_data.iter() {
            load_data.push(*byte);
        }
    }
    load_data
}

