use super::{DEBUG, InterruptController, SoundController};

pub struct Timer {
    timer_counter: u8,
    timer_modulo: u8,
    timer_enable: bool,
    clock_div: u16,
    divider_tick: u16,
}

impl Timer {
    pub fn new() -> Self {
        Timer {
            timer_counter: 0,
            timer_modulo: 0,
            timer_enable: false,
            clock_div: 1024,
            divider_tick: 0,
        }
    }

    pub fn execute_cycle(&mut self, interrupt_controller: &mut InterruptController, sound_controller: &mut SoundController) {
        if self.divider_tick & 0x1fff == 0x1fff {
            sound_controller.tick_frame_sequencer();
        }
        self.divider_tick = self.divider_tick.wrapping_add(1);

        if self.timer_enable && self.divider_tick % self.clock_div == 0 {
            self.timer_counter = self.timer_counter.wrapping_add(1);
            
            if DEBUG { println!("TAC: {}", self.timer_counter); }
            if self.timer_counter == 0 {
                self.timer_counter = self.timer_modulo;
                interrupt_controller.interrupt_flag |= 0x04;
            }
        }
    }


    pub fn write(&mut self, address: usize, value: u8) {
        match address {
            0xff04 => {
                let mask = self.clock_div / 2;
                if self.divider_tick & mask != 0 {
                    self.timer_counter += 1;
                }
                self.divider_tick = 0;
            }
            0xff05 => self.timer_counter = value,
            0xff06 => self.timer_modulo = value,
            0xff07 => {
                self.timer_enable = value & 0x04 != 0;
                self.clock_div = match value & 0x03 {
                    0 => 1024,
                    1 => 16,
                    2 => 64,
                    _ => 256,
                }
            }
            _ => panic!("Invalid write to Timer, address: {:02x}", address),
        }
    }

    pub fn read(&self, address: usize) -> u8 {
        match address {
            0xff04 => (self.divider_tick >> 8) as u8,
            0xff05 => self.timer_counter,
            0xff06 => self.timer_modulo,
            0xff07 => {
                let mut value = match self.clock_div {
                    1024 => 0,
                    16   => 1,
                    64   => 2,
                    _    => 3, // 256
                };
                if self.timer_enable { value |= 0x04 }
                value
            }
            _ => panic!("Invalid read from Timer, address {:02x}", address),
        }
    }
}
