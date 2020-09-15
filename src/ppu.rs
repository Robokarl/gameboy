use super::{Display, InterruptController, DEBUG};
use super::{SCREEN_HEIGHT, SCREEN_WIDTH};
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::render::{Texture, TextureCreator};
use sdl2::video::WindowContext;

#[derive(Copy, Clone, Default)]
struct BGMapAttributes {
    bg_oam_priority: bool,
    vertical_flip: bool,
    horizontal_flip: bool,
    unused: bool,
    vram_bank: bool,
    palette: u8,
}

impl BGMapAttributes {
    fn set_from_u8(&mut self, value: u8) {
        self.bg_oam_priority = value & 0x80 != 0;
        self.vertical_flip = value & 0x40 != 0;
        self.horizontal_flip = value & 0x20 != 0;
        self.unused = value & 0x10 != 0;
        self.vram_bank = value & 0x08 != 0;
        self.palette = value & 0x07;
    }

    fn as_u8(&self) -> u8 {
        let mut value = 0;
        if self.bg_oam_priority {
            value |= 0x80;
        }
        if self.vertical_flip {
            value |= 0x40;
        }
        if self.horizontal_flip {
            value |= 0x20;
        }
        if self.unused {
            value |= 0x10;
        }
        if self.vram_bank {
            value |= 0x08;
        }
        value | self.palette
    }
}

#[derive(Copy, Clone, Default)]
pub struct LcdControl {
    display_enable: bool,
    window_tile_map_select: bool,
    window_display_enable: bool,
    tile_address_mode: bool,
    bg_tile_map_select: bool,
    sprite_size: bool,
    sprite_display_enable: bool,
    bg_window_display_priority: bool,
}

impl LcdControl {
    pub fn as_u8(&self) -> u8 {
        let mut result = 0;
        if self.display_enable {
            result |= 0x80
        }
        if self.window_tile_map_select {
            result |= 0x40
        }
        if self.window_display_enable {
            result |= 0x20
        }
        if self.tile_address_mode {
            result |= 0x10
        }
        if self.bg_tile_map_select {
            result |= 0x08
        }
        if self.sprite_size {
            result |= 0x04
        }
        if self.sprite_display_enable {
            result |= 0x02
        }
        if self.bg_window_display_priority {
            result |= 0x01
        }
        result
    }

    pub fn set_from_u8(&mut self, value: u8) {
        self.display_enable = value & 0x80 == 0x80;
        self.window_tile_map_select = value & 0x40 == 0x40;
        self.window_display_enable = value & 0x20 == 0x20;
        self.tile_address_mode = value & 0x10 == 0x10;
        self.bg_tile_map_select = value & 0x08 == 0x08;
        self.sprite_size = value & 0x04 == 0x04;
        self.sprite_display_enable = value & 0x02 == 0x02;
        self.bg_window_display_priority = value & 0x01 == 0x01;
    }
}

#[derive(Copy, Clone)]
pub struct LcdStatus {
    coincidence_interrupt_enable: bool,
    oam_interrupt_enable: bool,
    vblank_interrupt_enable: bool,
    hblank_interrupt_enable: bool,
    coincidence_flag: bool,
    pub mode: u8,
}

impl Default for LcdStatus {
    fn default() -> Self {
        LcdStatus {
            coincidence_interrupt_enable: false,
            oam_interrupt_enable: false,
            vblank_interrupt_enable: false,
            hblank_interrupt_enable: false,
            coincidence_flag: false,
            mode: 2,
        }
    }
}

impl LcdStatus {
    pub fn as_u8(&self) -> u8 {
        let mut result = 0x80;
        if self.coincidence_interrupt_enable {
            result |= 0x40;
        }
        if self.oam_interrupt_enable {
            result |= 0x20;
        }
        if self.vblank_interrupt_enable {
            result |= 0x10;
        }
        if self.hblank_interrupt_enable {
            result |= 0x08;
        }
        if self.coincidence_flag {
            result |= 0x04;
        }
        result | self.mode
    }

    pub fn set_from_u8(&mut self, value: u8) {
        self.coincidence_interrupt_enable = value & 0x40 == 0x40;
        self.oam_interrupt_enable = value & 0x20 == 0x20;
        self.vblank_interrupt_enable = value & 0x10 == 0x10;
        self.hblank_interrupt_enable = value & 0x08 == 0x08;
    }
}

struct CgbPalette {
    auto_increment: bool,
    index: u8,
    palette: [u8; 0x40],
}

impl CgbPalette {
    fn new() -> Self {
        CgbPalette {
            auto_increment: false,
            index: 0,
            palette: [0; 0x40],
        }
    }

    fn read_index(&self) -> u8 {
        if self.auto_increment {
            self.index | 0x80
        } else {
            self.index
        }
    }

    fn read_data(&self) -> u8 {
        self.palette[self.index as usize]
    }

    fn write_index(&mut self, value: u8) {
        self.auto_increment = value & 0x80 != 0;
        self.index = value & 0x3f;
    }

    fn write_data(&mut self, value: u8, mode: u8) {
        if mode != 3 {
            if DEBUG {
                println!("Setting palette index {:02x} to {:02x}", self.index, value);
            }
            self.palette[self.index as usize] = value;
        }
        if self.auto_increment {
            self.index = (self.index + 1) & 0x3f;
        }
    }

    fn get_color(&self, palette: u8, shade: u8) -> Color {
        let address = ((shade * 2) + (palette * 8)) as usize;
        let rgb = (self.palette[address + 1] as u16) << 8 | self.palette[address] as u16;
        let red = rgb & 0x001f;
        let green = (rgb & 0x03d0) >> 5;
        let blue = (rgb & 0x7c00) >> 10;
        let red_corrected = (red * 13 + green * 2 + blue) >> 1;
        let green_corrected = (green * 3 + blue) << 1;
        let blue_corrected = (red * 3 + green * 2 + blue * 11) >> 1;
        Color::RGB(
            red_corrected as u8,
            green_corrected as u8,
            blue_corrected as u8,
        )
    }
}

pub struct PPU<'a> {
    texture: Texture<'a>,
    display: Display,
    frame_buffer: [u8; SCREEN_HEIGHT * SCREEN_WIDTH * 3],
    tile_data_bank0: [u8; 0x1800],
    tile_data_bank1: [u8; 0x1800],
    background_tile_map_bank0: [u8; 0x800],
    background_tile_map_bank1: [BGMapAttributes; 0x800],
    pub sprite_attribute_table: [u8; 0xa0],
    cgb_background_palette: CgbPalette,
    cgb_sprite_palette: CgbPalette,
    pub lcdc: LcdControl,
    pub lcd_status: LcdStatus,
    sprites_to_draw: [(u8, Sprite); SCREEN_WIDTH],
    bg_palette: u8,
    obj_palette_0: u8,
    obj_palette_1: u8,
    scroll_x: u8,
    scroll_y: u8,
    window_x: u8,
    window_y: u8,
    tick: u16,
    scanline: u8,
    ly_compare: u8,
    vram_bank_sel: u8,
    pub dmg_compatibility: bool,
    pub dmg_mode: bool,
    object_priority_mode: bool,
    screen_cleared: bool,
}

#[derive(Default, Copy, Clone)]
struct Sprite {
    y: u8,
    x: u8,
    tile_number: u8,
    bg_priority: bool,
    flip_y: bool,
    flip_x: bool,
    dmg_palette: bool,
    vram_bank: bool,
    cgb_palette: u8,
}

impl<'a> PPU<'a> {
    pub fn new(
        display: Display,
        texture_creator: &'a TextureCreator<WindowContext>,
        dmg_mode: bool,
    ) -> Self {
        let texture = texture_creator
            .create_texture_streaming(
                PixelFormatEnum::RGB24,
                SCREEN_WIDTH as u32,
                SCREEN_HEIGHT as u32,
            )
            .expect("Failed to create texture");

        PPU {
            texture,
            display,
            frame_buffer: [0; SCREEN_WIDTH * SCREEN_HEIGHT * 3],
            tile_data_bank0: [0; 0x1800],
            tile_data_bank1: [0; 0x1800],
            background_tile_map_bank0: [0; 0x800],
            background_tile_map_bank1: [BGMapAttributes::default(); 0x800],
            sprite_attribute_table: [0; 0xa0],
            bg_palette: 0,
            obj_palette_0: 0,
            obj_palette_1: 0,
            cgb_background_palette: CgbPalette::new(),
            cgb_sprite_palette: CgbPalette::new(),
            lcdc: LcdControl::default(),
            lcd_status: LcdStatus::default(),
            sprites_to_draw: [(0, Sprite::default()); SCREEN_WIDTH],
            scroll_x: 0,
            scroll_y: 0,
            window_x: 0,
            window_y: 0,
            tick: 0,
            scanline: 0,
            ly_compare: 0,
            vram_bank_sel: 0,
            dmg_mode,
            dmg_compatibility: false,
            object_priority_mode: false,
            screen_cleared: false,
        }
    }

    pub fn execute_cycle(&mut self, interrupt_controller: &mut InterruptController) {
        if !self.lcdc.display_enable {
            if !self.screen_cleared {
                self.frame_buffer = [0xff; SCREEN_WIDTH * SCREEN_HEIGHT * 3];
                self.render_frame();
                self.screen_cleared = true;
            }
            return;
        }

        self.screen_cleared = false;

        if self.lcd_status.mode == 3 && self.tick < SCREEN_WIDTH as u16 {
            self.draw_pixel(self.tick as u8, self.scanline);
        }

        self.tick += 1;

        let mut new_scanline = self.scanline;
        let mut new_mode = self.lcd_status.mode;

        match self.lcd_status.mode {
            0 => {
                // hblank
                if self.tick == 204 {
                    self.tick = 0;
                    new_scanline += 1;
                    if new_scanline == SCREEN_HEIGHT as u8 {
                        self.render_frame();
                        interrupt_controller.interrupt_flag |= 0x01;
                        if DEBUG {
                            println!("PPU: Ending HBLANK, starting VBLANK");
                        }
                        new_mode = 1;
                    } else {
                        if DEBUG {
                            println!(
                                "PPU: Ending HBLANK, starting OAM scan for scanline {}",
                                new_scanline
                            );
                        }
                        new_mode = 2;
                    }
                }
            }
            1 => {
                // vblank
                if self.tick == 456 {
                    self.tick = 0;
                    new_scanline += 1;
                    if new_scanline == 154 {
                        if DEBUG {
                            println!("PPU: Ending VBLANK, Starting OAM scan");
                        }
                        new_scanline = 0;
                        new_mode = 2;
                    } else if DEBUG {
                        println!("PPU: Starting scanline {}", new_scanline);
                    }
                }
            }
            2 => {
                // scan OAM
                if self.tick == 1 {
                    self.populate_sprites();
                } else if self.tick == 80 {
                    if DEBUG {
                        println!("PPU: Ending OAM scan, starting drawing");
                    }
                    self.tick = 0;
                    new_mode = 3;
                }
            }
            _ => {
                // draw line
                if self.tick == 172 {
                    if DEBUG {
                        println!("PPU: Ending drawing, starting hblank");
                    }
                    self.tick = 0;
                    new_mode = 0;
                }
            }
        }

        if self.lcd_status.mode != new_mode || self.scanline != new_scanline {
            let prev_interrupt_state = self.interrupt_state();
            self.lcd_status.mode = new_mode;
            self.scanline = new_scanline;
            self.lcd_status.coincidence_flag = self.scanline == self.ly_compare;
            let interrupt_state = self.interrupt_state();
            if interrupt_state && !prev_interrupt_state {
                interrupt_controller.interrupt_flag |= 0x02;
                if DEBUG {
                    println!("Setting LCD interrupt");
                }
            }
        }
    }

    fn sprite_in_scanline(&self, sprite_y: u8) -> bool {
        if self.lcdc.sprite_size {
            sprite_y <= self.scanline + 16 && sprite_y > self.scanline
        } else {
            sprite_y <= self.scanline + 16 && sprite_y > self.scanline + 8
        }
    }

    fn populate_sprites(&mut self) {
        let mut num_sprites = 0;
        self.sprites_to_draw = [(0, Sprite::default()); SCREEN_WIDTH];
        if self.lcdc.sprite_display_enable {
            let mut sprites = Vec::with_capacity(10);
            for sprite_num in 0..40 {
                if num_sprites == 10 {
                    break; // max 10 sprites per line
                }

                let sprite_addr = sprite_num * 4;
                let y = self.sprite_attribute_table[sprite_addr];
                if self.sprite_in_scanline(y) {
                    let sprite_flags = self.sprite_attribute_table[sprite_addr + 3];
                    sprites.push(Sprite {
                        y,
                        x: self.sprite_attribute_table[sprite_addr + 1],
                        tile_number: self.sprite_attribute_table[sprite_addr + 2],
                        bg_priority: sprite_flags & 0x80 != 0,
                        flip_y: sprite_flags & 0x40 != 0,
                        flip_x: sprite_flags & 0x20 != 0,
                        dmg_palette: sprite_flags & 0x10 != 0,
                        vram_bank: sprite_flags & 0x08 != 0,
                        cgb_palette: sprite_flags & 0x07,
                    });
                    num_sprites += 1;
                }
            }

            if self.object_priority_mode {
                sprites.sort_by_key(|k| k.x);
            }

            for x in 0..SCREEN_WIDTH as u8 {
                for sprite in sprites.iter() {
                    if sprite.x <= x + 8 && sprite.x > x {
                        let mut sprite_x = x + 8 - sprite.x;
                        if sprite.flip_x {
                            sprite_x = 7 - sprite_x;
                        }
                        let mut sprite_y = self.scanline + 16 - sprite.y;
                        if sprite.flip_y {
                            sprite_y = if self.lcdc.sprite_size {
                                15 - sprite_y
                            } else {
                                7 - sprite_y
                            };
                        }
                        let sprite_tile_number = if self.lcdc.sprite_size {
                            sprite.tile_number & 0xfe
                        } else {
                            sprite.tile_number
                        };
                        let sprite_shade = self.get_sprite_tile_shade(
                            sprite_tile_number,
                            sprite_x,
                            sprite_y,
                            sprite.vram_bank,
                        );
                        if sprite_shade != 0 {
                            self.sprites_to_draw[x as usize] = (sprite_shade, *sprite);
                            break;
                        }
                    }
                }
            }
        }
    }

    fn draw_pixel(&mut self, x: u8, y: u8) {
        let bg_tile_x;
        let bg_tile_y;
        let bg_tile_map_select;
        let mut pixel_x;
        let mut pixel_y;
        if self.lcdc.window_display_enable && x >= self.window_x - 7 && y >= self.window_y {
            // Window tile over background
            bg_tile_x = (x - (self.window_x - 7)) / 8;
            bg_tile_y = (y - self.window_y) / 8;
            bg_tile_map_select = self.lcdc.window_tile_map_select;
            pixel_x = (x - (self.window_x - 7)) % 8;
            pixel_y = (y - self.window_y) % 8;
        } else {
            // Background tile
            bg_tile_x = x.wrapping_add(self.scroll_x) / 8;
            bg_tile_y = y.wrapping_add(self.scroll_y) / 8;
            bg_tile_map_select = self.lcdc.bg_tile_map_select;
            pixel_x = x.wrapping_add(self.scroll_x) % 8;
            pixel_y = y.wrapping_add(self.scroll_y) % 8;
        }

        let bg_tile_idx = bg_xy_idx(bg_tile_x, bg_tile_y, bg_tile_map_select);
        let bg_tile_number = self.background_tile_map_bank0[bg_tile_idx];
        let bg_attributes = self.background_tile_map_bank1[bg_tile_idx];
        if bg_attributes.horizontal_flip {
            pixel_x = 7 - pixel_x;
        }
        if bg_attributes.vertical_flip {
            pixel_y = 7 - pixel_y;
        }
        let bg_window_shade =
            self.get_bg_tile_shade(bg_tile_number, pixel_x, pixel_y, bg_attributes.vram_bank);

        let (sprite_shade, sprite) = self.sprites_to_draw[x as usize];
        let display_background = !self.dmg_compatibility || self.lcdc.bg_window_display_priority;

        let mut draw_sprite = false;
        let mut draw_bg = false;
        // BG / Window / Sprite priority
        if bg_attributes.bg_oam_priority {
            draw_bg = true;
        } else if !self.dmg_compatibility && !self.lcdc.bg_window_display_priority {
            draw_sprite = true;
        } else if sprite_shade == 0 && display_background {
            draw_bg = true;
        } else if !sprite.bg_priority || bg_window_shade == 0 {
            // Sprite in front of BG, or behind BG shade 0
            draw_sprite = true;
        } else if display_background {
            // Sprite behind BG, draw BG
            draw_bg = true;
        }

        let pixel_color = if draw_sprite {
            if self.dmg_compatibility {
                self.get_sprite_draw_color(sprite_shade, &sprite)
            } else {
                self.cgb_sprite_palette
                    .get_color(sprite.cgb_palette, sprite_shade)
            }
        } else if draw_bg {
            if self.dmg_compatibility {
                self.get_bg_draw_color(bg_window_shade)
            } else {
                self.cgb_background_palette
                    .get_color(bg_attributes.palette, bg_window_shade)
            }
        } else {
            Color::WHITE
        };

        let idx = (y as usize * SCREEN_WIDTH + x as usize) * 3;
        self.frame_buffer[idx] = pixel_color.r;
        self.frame_buffer[idx + 1] = pixel_color.g;
        self.frame_buffer[idx + 2] = pixel_color.b;
    }

    fn get_sprite_tile_shade(&self, tile_number: u8, x: u8, y: u8, vram_bank: bool) -> u8 {
        debug_assert!(x < 8 && y < 16, format!("x: {}, y: {}", x, y));

        let tile_start_address = tile_number as usize * 16;
        let tile_address = tile_start_address + (y as usize * 2);

        let tile_bank = if vram_bank {
            &self.tile_data_bank1
        } else {
            &self.tile_data_bank0
        };

        let byte1 = tile_bank[tile_address];
        let byte2 = tile_bank[tile_address + 1];
        let mask = 0x80 >> x;
        let mut shade = 0;
        if byte2 & mask != 0 {
            shade |= 0x2
        };
        if byte1 & mask != 0 {
            shade |= 0x1
        };
        shade
    }

    fn get_bg_tile_shade(&self, tile_number: u8, x: u8, y: u8, vram_bank: bool) -> u8 {
        debug_assert!(x < 8 && y < 8, format!("x: {}, y: {}", x, y));

        #[allow(clippy::collapsible_if)]
        let tile_start_address = if self.lcdc.tile_address_mode {
            tile_number as usize * 16
        } else {
            if tile_number >= 128 {
                (tile_number as usize - 128) * 16 + 0x800
            } else {
                tile_number as usize * 16 + 0x1000
            }
        };
        let tile_address = tile_start_address + (y as usize * 2);

        let tile_bank = if vram_bank {
            &self.tile_data_bank1
        } else {
            &self.tile_data_bank0
        };

        let byte1 = tile_bank[tile_address];
        let byte2 = tile_bank[tile_address + 1];
        let mask = 0x80 >> x;
        let mut shade = 0;
        if byte2 & mask != 0 {
            shade |= 0x2
        };
        if byte1 & mask != 0 {
            shade |= 0x1
        };
        shade
    }

    pub fn write_vram(&mut self, address: usize, value: u8) {
        if self.lcd_status.mode == 3 && self.lcdc.display_enable {
            if DEBUG {
                println!("PPU: Write ignored");
            }
            return; // VRAM inacessible in mode 3
        }

        match address {
            0x8000..=0x97ff if self.vram_bank_sel == 0 => {
                self.tile_data_bank0[address - 0x8000] = value
            }
            0x8000..=0x97ff if self.vram_bank_sel == 1 => {
                self.tile_data_bank1[address - 0x8000] = value
            }
            0x9800..=0x9fff if self.vram_bank_sel == 0 => {
                self.background_tile_map_bank0[address - 0x9800] = value
            }
            0x9800..=0x9fff if self.vram_bank_sel == 1 => {
                self.background_tile_map_bank1[address - 0x9800].set_from_u8(value)
            }
            _ => panic!("Invalid VRAM write to address {:02x}", address),
        }
    }

    pub fn read_vram(&self, address: usize) -> u8 {
        if self.lcd_status.mode == 3 && self.lcdc.display_enable {
            return 0xff; // VRAM inacessible in mode 3
        }

        match address {
            0x8000..=0x97ff if self.vram_bank_sel == 0 => self.tile_data_bank0[address - 0x8000],
            0x8000..=0x97ff if self.vram_bank_sel == 1 => self.tile_data_bank1[address - 0x8000],
            0x9800..=0x9fff if self.vram_bank_sel == 0 => {
                self.background_tile_map_bank0[address - 0x9800]
            }
            0x9800..=0x9fff if self.vram_bank_sel == 1 => {
                self.background_tile_map_bank1[address - 0x9800].as_u8()
            }
            _ => panic!("Invalid VRAM read from address {:02x}", address),
        }
    }

    pub fn write_oam(&mut self, address: usize, value: u8) {
        if !self.lcdc.display_enable
            || !(self.lcd_status.mode == 2 || self.lcd_status.mode == 3) && address < 0xfea0
        {
            self.sprite_attribute_table[address - 0xfe00] = value;
        } else if DEBUG {
            println!("PPU: OAM write ignored");
        }
    }

    pub fn read_oam(&self, address: usize) -> u8 {
        if self.lcdc.display_enable
            && (self.lcd_status.mode == 2 || self.lcd_status.mode == 3 || address >= 0xfea0)
        {
            0xff
        } else {
            self.sprite_attribute_table[address - 0xfe00]
        }
    }

    pub fn write_register(&mut self, address: usize, value: u8) {
        match address {
            0xff40 => self.lcdc.set_from_u8(value),
            0xff41 => self.lcd_status.set_from_u8(value),
            0xff42 => self.scroll_y = value,
            0xff43 => self.scroll_x = value,
            0xff44 => {} // LCDC y-coordinate, read-only
            0xff45 => self.ly_compare = value,
            0xff47 => self.bg_palette = value,
            0xff48 => self.obj_palette_0 = value,
            0xff49 => self.obj_palette_1 = value,
            0xff4a => self.window_y = value,
            0xff4b => self.window_x = value,
            0xff4f => self.vram_bank_sel = value & 0x01,
            0xff68 => self.cgb_background_palette.write_index(value),
            0xff69 => self
                .cgb_background_palette
                .write_data(value, self.lcd_status.mode),
            0xff6a => self.cgb_sprite_palette.write_index(value),
            0xff6b => self
                .cgb_sprite_palette
                .write_data(value, self.lcd_status.mode),
            0xff6c => self.object_priority_mode = value & 0x01 == 0x01,
            _ => println!(
                "PPU register write to address {:04x} not implemented, data: {:02x}",
                address, value
            ),
        }
    }

    pub fn read_register(&self, address: usize) -> u8 {
        match address {
            0xff40 => self.lcdc.as_u8(),
            0xff41 => self.lcd_status.as_u8(),
            0xff42 => self.scroll_y,
            0xff43 => self.scroll_x,
            0xff44 => self.scanline,
            0xff45 => self.ly_compare,
            0xff47 => self.bg_palette,
            0xff48 => self.obj_palette_0,
            0xff49 => self.obj_palette_1,
            0xff4a => self.window_y,
            0xff4b => self.window_x,
            0xff4f => self.vram_bank_sel | 0xfe,
            0xff68 => self.cgb_background_palette.read_index(),
            0xff69 => self.cgb_background_palette.read_data(),
            0xff6a => self.cgb_sprite_palette.read_index(),
            0xff6b => self.cgb_sprite_palette.read_data(),
            0xff6c => if self.object_priority_mode { 0x01  } else { 0x00 },
            _ => {
                println!(
                    "PPU register read from address {:04x} not implemented",
                    address
                );
                0xff
            }
        }
    }

    fn get_sprite_draw_color(&self, palette_index: u8, sprite: &Sprite) -> Color {
        if sprite.dmg_palette {
            let palette = self.obj_palette_1;
            let color_number = (palette >> (palette_index * 2)) & 0x03;
            self.cgb_sprite_palette.get_color(1, color_number)
        } else {
            let palette = self.obj_palette_0;
            let color_number = (palette >> (palette_index * 2)) & 0x03;
            self.cgb_sprite_palette.get_color(0, color_number)
        }
    }

    fn get_bg_draw_color(&self, palette_index: u8) -> Color {
        let color_number = (self.bg_palette >> (palette_index * 2)) & 0x03;
        self.cgb_background_palette.get_color(0, color_number)
    }

    pub fn interrupt_state(&self) -> bool {
        (self.lcd_status.coincidence_interrupt_enable && self.ly_compare == self.scanline)
            || (self.lcd_status.oam_interrupt_enable && self.lcd_status.mode == 2)
            || (self.lcd_status.vblank_interrupt_enable && self.lcd_status.mode == 1)
            || (self.lcd_status.hblank_interrupt_enable && self.lcd_status.mode == 0)
    }

    fn render_frame(&mut self) {
        self.texture
            .update(None, &self.frame_buffer, SCREEN_WIDTH * 3)
            .expect("Failed to update texture");
        self.display.render(&self.texture);
    }
}

fn bg_xy_idx(x: u8, y: u8, select_high: bool) -> usize {
    let idx = x as usize + (32 * y as usize);
    if select_high {
        idx + 0x400
    } else {
        idx
    }
}
