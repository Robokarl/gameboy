use std::path::Path;

mod no_mbc;
mod mbc1;
mod mbc2;
mod mbc3;
mod mbc5;

pub use no_mbc::NoMbc as NoMbc;
pub use mbc1::Mbc1 as Mbc1;
pub use mbc2::Mbc2 as Mbc2;
pub use mbc3::Mbc3 as Mbc3;
pub use mbc5::Mbc5 as Mbc5;

pub trait Mbc {
    fn write(&mut self, address: usize, value: u8);
    
    fn read(&self, address: usize) -> u8;

    fn update_rtc(&mut self, _micros: u64) {
        // Default is no RTC
    }

    fn save(&self, path: &Path);
}
