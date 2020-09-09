use super::{SCALE_FACTOR, SCREEN_HEIGHT, SCREEN_WIDTH};
use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;

pub struct Display {
    pub canvas: Canvas<Window>,
}

impl Display {
    pub fn new(sdl: &sdl2::Sdl) -> Self {
        let sdl_video = sdl.video().unwrap();
        let window = sdl_video
            .window(
                "Gameboy Emulator",
                SCREEN_WIDTH as u32 * SCALE_FACTOR,
                SCREEN_HEIGHT as u32 * SCALE_FACTOR,
            )
            .build()
            .unwrap();
        let mut canvas = window.into_canvas().accelerated().build().unwrap();
        canvas.clear();
        canvas.present();

        Display { canvas }
    }

    pub fn render(&mut self, texture: &Texture) {
        self.canvas
            .copy(texture, None, None)
            .expect("Failed to copy texture");
        self.canvas.present();
    }
}
