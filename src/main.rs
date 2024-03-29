#![allow(clippy::verbose_bit_mask)]

use gumdrop::Options;
use sdl2::render::TextureCreator;
use sdl2::video::WindowContext;
use std::thread;
use std::time::{Duration, Instant};

mod cpu;
use cpu::Cpu;
mod mmu;
use mmu::{DmaType, Mmu};
mod ppu;
use ppu::Ppu;
mod sound;
use sound::SoundController;
mod interrupts;
use interrupts::*;
mod timer;
use timer::Timer;
mod joypad;
use joypad::Joypad;
mod serial;
use serial::SerialLink;
mod display;
use display::Display;
mod input;
use input::Input;
mod cartridge;
mod instructions;
mod mbc;
mod registers;
use cartridge::Cartridge;

pub const DEBUG: bool = false;
const BENCHMARK: bool = false;
const BENCHMARK_COUNT: u32 = 10_000_000;
const LIMIT_SPEED: bool = !BENCHMARK && !DEBUG;

const SCREEN_WIDTH: usize = 160;
const SCREEN_HEIGHT: usize = 144;
const SCALE_FACTOR: u32 = 5;

struct GameBoy<'a> {
    cpu: Cpu<'a>,
    cycle_count: u32,
    input: Input,
}

#[derive(Options)]
struct MyOptions {
    #[options(required, help = "path to ROM")]
    rom: String,

    #[options(help = "print help message")]
    help: bool,

    #[options(help = "run in non-color gameboy mode")]
    dmg_mode: bool,
}

impl<'a> GameBoy<'a> {
    pub fn new(
        sdl: &sdl2::Sdl,
        display: Display,
        texture_creator: &'a TextureCreator<WindowContext>,
        rom_path: &str,
        dmg_mode: bool,
    ) -> Self {
        GameBoy {
            cpu: Cpu::new(rom_path, sdl, display, texture_creator, dmg_mode),
            cycle_count: 0,
            input: Input::new(sdl.event_pump().unwrap()),
        }
    }

    fn poll_inputs(&mut self) {
        self.input.poll_inputs(&mut self.cpu.mmu.joypad);
        self.cpu.mmu.sound_controller.set_run_2x(self.input.run_2x);
        self.cpu.mmu.sound_controller.set_mute(self.input.mute);
        if self.input.quit {
            self.cpu.mmu.cartridge.save();
            std::process::exit(0);
        }
    }

    pub fn execute_cycle(&mut self) {

        if self.cycle_count % 4096 == 0 {
            self.poll_inputs();
        }

        let double_speed = self.cpu.mmu.double_speed;

        let mut update_rtc_cycle = if double_speed { 131072 } else { 65536 };
        if self.input.run_2x {
            update_rtc_cycle *= 2;
        }

        if self.cycle_count % update_rtc_cycle == 0 {
            self.cpu.mmu.cartridge.update_rtc(15_625);
        }

        // PPU runs at 4MHz always
        if !double_speed || self.cycle_count % 2 == 0 {
            self.cpu.mmu.ppu.execute_cycle(&mut self.cpu.mmu.interrupt_controller);
        }

        // Timer runs at 4MHz or 8MHz (every cycle)
        self.cpu.mmu.timer.execute_cycle(&mut self.cpu.mmu.interrupt_controller, &mut self.cpu.mmu.sound_controller, double_speed);

        // Cpu runs at 1MHz or 2MHz (4 cycles)
        if self.cycle_count % 4 == 0 {
            self.cpu.execute_cycle();
        }

        // OAM transfer runs at 1MHz or 2MHz (4 cycles)
        // Other DMAs always run at 2MHz
        let dma_cycles = match self.cpu.mmu.dma_config.dma_type {
            DmaType::Oam => 4,
            _ if !double_speed => 2,
            _                  => 4,
        };
        if self.cycle_count % dma_cycles == 0 {
            self.cpu.mmu.execute_cycle();
        }

        // APU always runs at 4MHz
        if !double_speed || self.cycle_count % 2 == 0 {
            self.cpu.mmu.sound_controller.execute_cycle();
        }

        self.cycle_count = self.cycle_count.wrapping_add(1);
    }

    pub fn run(&mut self) {
        let mut benchmark_time = Instant::now();
        let mut count = 0;

        loop {
            if self.input.pause {
                self.poll_inputs();
                thread::sleep(Duration::from_millis(10));
            } else if LIMIT_SPEED && self.cpu.mmu.sound_controller.buffer_full() {
                self.cpu.mmu.sound_controller.queue_audio();
                thread::sleep(Duration::from_millis(1));
            } else {
                self.execute_cycle();
                count += 1;
                if count == BENCHMARK_COUNT {
                    let now = Instant::now();
                    if BENCHMARK {
                        println!(
                            "Benchmark time: {}ms",
                            now.duration_since(benchmark_time).as_millis()
                        );
                    }
                    benchmark_time = now;
                    count = 0;
                }
            }
        }
    }
}

fn main() {
    let opts = MyOptions::parse_args_default_or_exit();
    let sdl = sdl2::init().unwrap();
    let display = Display::new(&sdl);
    let texture_creator = display.canvas.texture_creator();
    let mut game_boy = GameBoy::new(&sdl, display, &texture_creator, &opts.rom, opts.dmg_mode);
    game_boy.run();
}
