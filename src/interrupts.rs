
#[derive(PartialEq)]
pub enum InterruptState{
    Disabled,
    Scheduled,
    Enabled,
}

pub struct InterruptController {
    pub state: InterruptState,
    pub interrupt_enable: u8,
    pub interrupt_flag: u8,
}

impl InterruptController {
    pub fn new() -> Self {
        InterruptController {
            state: InterruptState::Disabled,
            interrupt_enable: 0,
            interrupt_flag: 0,
        }
    }

    pub fn write(&mut self, address: usize, value: u8) {
        match address {
            0xffff => {
                if self.interrupt_enable & 0x10 == 0 && value & 0x10 != 0 {
                    println!("joypad interrupt enabled (unimplemented)");
                }
                self.interrupt_enable = value;
            }
            0xff0f => self.interrupt_flag = value,
            _ => panic!("Invalid write to Interrupt Controller, address: {:02x}", address),
        }
    }

    pub fn read(&self, address: usize) -> u8 {
        match address {
            0xffff => self.interrupt_enable | 0xe0,
            0xff0f => self.interrupt_flag | 0xe0,
            _ => panic!("Invalid read from Interrupt Controller, address: {:02x}", address),
        }
    }

    pub fn poll_interrupts(&self) -> bool {
        self.interrupt_enable & self.interrupt_flag != 0
    }

    pub fn pending_interrupts(&mut self) -> Option<usize> {
        match self.state {
            InterruptState::Enabled => {
                let pending = self.interrupt_enable & self.interrupt_flag;
                if pending & 0x01 != 0 { // vblank interrupt
                    self.state = InterruptState::Disabled;
                    self.interrupt_flag &= !0x01;
                    Some(0x0040)
                } else if pending & 0x02 != 0 { // LCD STAT
                    self.state = InterruptState::Disabled;
                    self.interrupt_flag &= !0x02;
                    Some(0x0048)
                } else if pending & 0x04 != 0 { // Timer
                    self.state = InterruptState::Disabled;
                    self.interrupt_flag &= !0x04;
                    Some(0x0050)
                } else if pending & 0x08 != 0 { // Serial
                    self.state = InterruptState::Disabled;
                    self.interrupt_flag &= !0x08;
                    Some(0x0058)
                } else if pending & 0x10 != 0 { // Joypad
                    self.state = InterruptState::Disabled;
                    self.interrupt_flag &= !0x10;
                    Some(0x0060)
                } else {
                    None
                }
            }
            InterruptState::Scheduled => {
                self.state = InterruptState::Enabled;
                None
            }
            InterruptState::Disabled => {
                None
            }
        }
    }
}

