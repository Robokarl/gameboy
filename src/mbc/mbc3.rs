use super::MBC;
use std::path::PathBuf;
use std::io::{BufWriter, Write};
use std::fs::File;
use std::time::{SystemTime, Duration};
use nanoserde::{DeBin, SerBin};

#[derive(Clone, Default, SerBin, DeBin)]
struct RtcTime {
    micros: u32,
    seconds: u8,
    minutes: u8,
    hours: u8,
    days: u16,
    day_carry: bool,
}

#[derive(Clone, Default, SerBin, DeBin)]
struct Rtc {
    live_time: RtcTime,
    latched_time: RtcTime,
    latched: bool,
    halt: bool,
}


pub struct MBC3 {
    rom: Vec<u8>,
    ram: [u8; 0x8000],
    rom_bank: usize,
    ram_timer_enabled: bool,
    ram_timer_select: u8,
    rom_size: u8,
    rtc: Rtc,
    has_battery: bool,
    has_rtc: bool,
}

impl MBC3 {
    pub fn new(rom: Vec<u8>, load_data: &[u8], battery: bool, rtc: bool) -> Self {
        let mut mbc = MBC3 {
            rom: vec![0; 0x20_0000],
            ram: [0; 0x8000],
            rom_bank: 1,
            ram_timer_enabled: false,
            ram_timer_select: 0,
            rom_size: rom[0x148],
            rtc: Rtc::default(),
            has_battery: battery,
            has_rtc: rtc,
        };

        mbc.rom[0..rom.len()].copy_from_slice(&rom);

        if !load_data.is_empty() {
            let mut load_index = 0;
            
            if battery {
                mbc.ram.copy_from_slice(&load_data[0..0x8000]);
                load_index += 0x8000;
            }

            if rtc {
                mbc.rtc = DeBin::deserialize_bin(&load_data[load_index..load_index+22]).unwrap();
                load_index += 22;
                let last_save_millis: u64 = DeBin::deserialize_bin(&load_data[load_index..load_index+8]).unwrap();
                let last_save = SystemTime::UNIX_EPOCH + Duration::from_millis(last_save_millis);
                let duration_since_last_save = SystemTime::now().duration_since(last_save);
                let micros_since_last_save = match duration_since_last_save {
                    Ok(duration) => duration.as_micros() as u64,
                    Err(_) => 0,
                };
                mbc.update_rtc(micros_since_last_save);
            }
        }

        mbc
    }

}

impl MBC for MBC3 {
    fn read(&self, address: usize) -> u8 {
        if address < 0x4000 {
            self.rom[address]
        } else if address < 0x8000 {
            let rom_bank = self.rom_bank % (2 << self.rom_size);
            let rom_address = rom_bank * 0x4000 + (address - 0x4000);
            self.rom[rom_address]
        } else if address >= 0xa000 && address < 0xc000 {
            if self.ram_timer_select < 4 && self.ram_timer_enabled {
                let ram_address = self.ram_timer_select as usize * 0x2000 + (address - 0xa000);
                self.ram[ram_address]
            } else if self.ram_timer_enabled {
                let read_time = if self.rtc.latched { &self.rtc.latched_time } else { &self.rtc.live_time };
                match self.ram_timer_select {
                    0x8 => read_time.seconds,
                    0x9 => read_time.minutes,
                    0xa => read_time.hours,
                    0xb => read_time.days as u8,
                    0xc => {
                        let mut result = 0x00;
                        result |= if read_time.day_carry { 0x80 } else { 0x00 };
                        result |= if self.rtc.halt { 0x40 } else { 0x00 };
                        result |= if read_time.days & 0x0100 != 0 { 0x01 } else { 0x00 };
                        result
                    }
                    _   => 0xff,
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
            self.ram_timer_enabled = value & 0x0f == 0x0a;
        } else if address < 0x4000 {
            self.rom_bank = if value & 0x7f == 0 { 1 } else { value & 0x7f } as usize;
        } else if address < 0x6000 {
            self.ram_timer_select = value & 0x0f;
            if (self.ram_timer_select >= 4 && self.ram_timer_select < 8) || self.ram_timer_select > 0xc {
                println!("Invalid ram / RTC select: 0x{:x}", self.ram_timer_select);
            }
        } else if address < 0x8000 {
            let new_rtc_latched = value & 0x01 != 0x00;
            if !self.rtc.latched && new_rtc_latched {
                self.rtc.latched_time = self.rtc.live_time.clone();
            }
            self.rtc.latched = new_rtc_latched;
        } else if address >= 0xa000 && address < 0xc000 {
            if self.ram_timer_select < 4 && self.ram_timer_enabled {
                let ram_address = self.ram_timer_select as usize * 0x2000 + (address - 0xa000);
                self.ram[ram_address] = value;
            } else if self.ram_timer_enabled {
                #[allow(clippy::single_match)]
                match self.ram_timer_select {
                    0x08 => self.rtc.live_time.seconds = value % 0x40,
                    0x09 => self.rtc.live_time.minutes = value % 0x40,
                    0x0a => self.rtc.live_time.hours = value % 0x20,
                    0x0b => self.rtc.live_time.days = (self.rtc.live_time.days & 0xff00) | value as u16,
                    0x0c => {
                        let day_high = (value as u16 & 0x01) << 8;
                        self.rtc.live_time.days = (self.rtc.live_time.days & 0x00ff) | day_high;

                        self.rtc.halt = value & 0x40 != 0;
                        self.rtc.live_time.day_carry = value & 0x80 != 0;
                    }
                    _   => { println!("Invalid write to RTC registers") },
                }
            }
        } else {
            panic!("Invalid write to ROM.  Address = {:04x}", address);
        }
    }

    fn save(&self, path: &PathBuf) {
        if !self.has_battery && !self.has_rtc {
            return;
        }

        let mut buffer = BufWriter::new(File::create(path).expect("Cannot open save file"));
        if self.has_battery {
            buffer.write_all(&self.ram).expect("Failed to save");
        }
        if self.has_rtc {
            buffer.write_all(&SerBin::serialize_bin(&self.rtc)).expect("Failed to save");
            let now_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
            let now_millis = now_time.as_millis() as u64;
            buffer.write_all(&SerBin::serialize_bin(&now_millis)).expect("Failed to save");
        }
    }
    
    fn update_rtc(&mut self, micros: u64) {
        if !self.rtc.halt {
            let new_micros = self.rtc.live_time.micros as u64 + micros;
            self.rtc.live_time.micros = (new_micros % 1_000_000) as u32;
            let seconds_cin = new_micros / 1_000_000;

            let minutes_cin = if seconds_cin > 0 {
                let new_seconds = if self.rtc.live_time.seconds > 59 { 59 } else { self.rtc.live_time.seconds } as u64 + seconds_cin;
                self.rtc.live_time.seconds = (new_seconds % 60) as u8;
                new_seconds / 60
            } else {
                0
            };

            let hours_cin = if minutes_cin > 0 {
                let new_minutes = if self.rtc.live_time.minutes > 59 { 59 } else { self.rtc.live_time.minutes } as u64 + minutes_cin;
                self.rtc.live_time.minutes = (new_minutes % 60) as u8;
                new_minutes / 60
            } else {
                0
            };

            let days_cin = if hours_cin > 0 {
                let new_hours = if self.rtc.live_time.hours > 23 { 23 } else { self.rtc.live_time.hours } as u64 + hours_cin;
                self.rtc.live_time.hours = (new_hours % 24) as u8;
                new_hours / 24
            } else {
                0
            };

            if days_cin > 0 {
                let new_days = self.rtc.live_time.days as u64 + days_cin;
                self.rtc.live_time.days = (new_days % 512) as u16;
                if new_days > 511 {
                    self.rtc.live_time.day_carry = true;
                }
            }
        }
    }
}
