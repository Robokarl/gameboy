use super::Joypad;
use sdl2::EventPump;
use sdl2::keyboard::{Scancode, Keycode};
use sdl2::event::{Event, WindowEvent};


pub struct Input {
    event_pump: EventPump,
    pub quit: bool,
    pub pause: bool,
    pub mute: bool,
    pub run_2x: bool,
}

impl Input {
    pub fn new(event_pump: EventPump) -> Self {
        Self {
            event_pump,
            quit: false,
            pause: false,
            mute: false,
            run_2x: false,
        }
    }

    pub fn poll_inputs(&mut self, joypad: &mut Joypad) {
        let (mut buttons_state, mut directions_state) = joypad.get_values();
        for event in self.event_pump.poll_iter() {
            match event {
                Event::KeyDown { scancode: Some(Scancode::Down),  .. } => directions_state |= 0x08,
                Event::KeyDown { scancode: Some(Scancode::Up),    .. } => directions_state |= 0x04,
                Event::KeyDown { scancode: Some(Scancode::Left),  .. } => directions_state |= 0x02,
                Event::KeyDown { scancode: Some(Scancode::Right), .. } => directions_state |= 0x01,

                Event::KeyDown { scancode: Some(Scancode::Return), .. } => buttons_state |= 0x08,
                Event::KeyDown { scancode: Some(Scancode::Space),  .. } => buttons_state |= 0x04,
                Event::KeyDown { scancode: Some(Scancode::Q),      .. } => buttons_state |= 0x02,
                Event::KeyDown { scancode: Some(Scancode::W),      .. } => buttons_state |= 0x01,

                Event::KeyUp { scancode: Some(Scancode::Down),  .. } => directions_state &= !0x08,
                Event::KeyUp { scancode: Some(Scancode::Up),    .. } => directions_state &= !0x04,
                Event::KeyUp { scancode: Some(Scancode::Left),  .. } => directions_state &= !0x02,
                Event::KeyUp { scancode: Some(Scancode::Right), .. } => directions_state &= !0x01,

                Event::KeyUp { scancode: Some(Scancode::Return), .. } => buttons_state &= !0x08,
                Event::KeyUp { scancode: Some(Scancode::Space),  .. } => buttons_state &= !0x04,
                Event::KeyUp { scancode: Some(Scancode::Q),      .. } => buttons_state &= !0x02,
                Event::KeyUp { scancode: Some(Scancode::W),      .. } => buttons_state &= !0x01,

                Event::KeyDown { keycode: Some(Keycode::P), .. } => {
                    self.pause = !self.pause;
                    if self.pause { println!("Paused") } else { println!("Unpaused") };
                }
                Event::KeyDown { keycode: Some(Keycode::LShift), .. } => {
                    self.run_2x = !self.run_2x;
                    if self.run_2x { println!("Running at double speed") } else { println!("Running at normal speed") };
                }
                Event::KeyDown { keycode: Some(Keycode::M), .. } => {
                    self.mute = !self.mute;
                    if self.mute { println!("Muted") } else { println!("Unmuted") };
                }
                Event::Window { win_event: WindowEvent::Close, .. }  => self.quit = true,
                Event::Quit { .. } => self.quit = true,
                _ => {}
            }
        }

        joypad.set_values(buttons_state, directions_state);

    }
}
