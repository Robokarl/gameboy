
use sdl2::EventPump;
use sdl2::keyboard::Scancode;

pub struct Joypad {
    event_pump: EventPump,
    select_buttons: bool,
    select_directions: bool,
    buttons_state: u8,
    directions_state: u8,
}

impl Joypad {
    pub fn new(sdl: &sdl2::Sdl) -> Self {
        Joypad {
            event_pump: sdl.event_pump().unwrap(),
            select_buttons: true,
            select_directions: true,
            buttons_state: 0x0f,
            directions_state: 0x0f,
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

    pub fn poll_inputs(&mut self) -> bool {
        for event in self.event_pump.poll_iter() {
            if let sdl2::event::Event::Quit { .. } = event {
                return true;
            }
        }

        let keyboard_state = self.event_pump.keyboard_state();
        let scancodes = keyboard_state.pressed_scancodes();
        self.directions_state = 0x00;
        self.buttons_state = 0x00;
        for code in scancodes {
            match code {
                Scancode::Down  => self.directions_state |= 0x08,
                Scancode::Up    => self.directions_state |= 0x04,
                Scancode::Left  => self.directions_state |= 0x02,
                Scancode::Right => self.directions_state |= 0x01,

                Scancode::Return => self.buttons_state |= 0x08,
                Scancode::Space  => self.buttons_state |= 0x04,
                Scancode::A      => self.buttons_state |= 0x02,
                Scancode::S      => self.buttons_state |= 0x01,
                _ => {}
            }
        }

        false
    }
}
