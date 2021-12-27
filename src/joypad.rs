
pub struct Joypad {
    select_buttons: bool,
    select_directions: bool,
    buttons_state: u8,
    directions_state: u8,
}

impl Joypad {
    pub fn new() -> Self {
        Joypad {
            select_buttons: true,
            select_directions: true,
            buttons_state: 0x00,
            directions_state: 0x00,
        }
    }

    pub fn write(&mut self, value: u8) {
        self.select_buttons = value & 0x20 == 0;
        self.select_directions = value & 0x10 == 0;
    }

    pub fn read(&self) -> u8 {
        let mut value = 0xff;
        if self.select_buttons { value &= 0xdf }
        if self.select_directions { value &= 0xef }
        if self.select_buttons {
            value &= !self.buttons_state;
        } else if self.select_directions {
            value &= !self.directions_state;
        }

        value
    }

    pub fn set_values(&mut self, buttons: u8, directions: u8) {
        self.buttons_state = buttons;
        self.directions_state = directions;
    }

    pub fn get_values(&self) -> (u8, u8) {
        (self.buttons_state, self.directions_state)
    }
}
