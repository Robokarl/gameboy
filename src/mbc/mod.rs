use std::path::PathBuf;

mod no_mbc;
mod mbc1;
mod mbc2;
mod mbc3;
mod mbc5;

pub use no_mbc::NoMBC as NoMBC;
pub use mbc1::MBC1 as MBC1;
pub use mbc2::MBC2 as MBC2;
pub use mbc3::MBC3 as MBC3;
pub use mbc5::MBC5 as MBC5;

pub trait MBC {
    fn write(&mut self, address: usize, value: u8);
    
    fn read(&self, address: usize) -> u8;

    fn update_rtc(&mut self, _micros: u64) {
        // Default is no RTC
    }

    fn save(&self, path: &PathBuf);
}
