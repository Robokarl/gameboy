
pub struct SerialLink {
    byte_to_transfer: u8,
    shift_clock: u8,
}

impl SerialLink {
    pub fn new() -> Self {
        SerialLink {
            byte_to_transfer: 0,
            shift_clock: 0,
        }
    }

    pub fn read(&self, address: usize) -> u8 {
        if address == 0xff01 {
            self.byte_to_transfer
        } else if address == 0xff02 {
            0x7e | self.shift_clock
        } else {
            0xff
        }
    }

    pub fn write(&mut self, address: usize, value: u8) {
        if address == 0xff01 {
            self.byte_to_transfer = value;
        } else if address == 0xff02 {
            if value & 0x80 == 0x80 {
                //print!("{}", self.byte_to_transfer as char);
            }
            self.shift_clock = value & 0x01;
        }
    }
}
