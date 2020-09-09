#![allow(clippy::verbose_bit_mask)]
extern crate sdl2;

use gumdrop::Options;
use sdl2::render::TextureCreator;
use sdl2::video::WindowContext;
use std::thread;
use std::time::{Duration, Instant};

mod cpu;
use cpu::CPU;
mod mmu;
use mmu::{DmaType, MMU};
mod ppu;
use ppu::PPU;
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
const SCALE_FACTOR: u32 = 4;

struct GameBoy<'a> {
    cpu: CPU<'a>,
    cycle_count: u16,
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
            cpu: CPU::new(rom_path, sdl, display, texture_creator, dmg_mode),
            cycle_count: 0,
        }
    }

    pub fn execute_cycle(&mut self) {
        self.cpu.mmu.ppu.execute_cycle(&mut self.cpu.mmu.interrupt_controller);
        if self.cycle_count == 0 {
            self.cpu.mmu.cartridge.update_rtc(15_625);
            let quit = self.cpu.mmu.joypad.poll_inputs();
            if quit {
                self.cpu.mmu.cartridge.save();
                std::process::exit(0);
            }
        }

        self.cpu.mmu.timer.execute_cycle(&mut self.cpu.mmu.interrupt_controller, &mut self.cpu.mmu.sound_controller);
        let double_speed = self.cpu.mmu.double_speed;
        if (self.cycle_count % 4 == 0) || (double_speed && (self.cycle_count % 2 == 0)) {
            self.cpu.execute_cycle();
        }

        let dma_cycles = match self.cpu.mmu.dma_config.dma_type {
            DmaType::Oam if double_speed => 2,
            DmaType::Oam if !double_speed => 4,
            _ => 2,
        };
        if self.cycle_count % dma_cycles == 0 {
            self.cpu.mmu.execute_cycle();
        }

        self.cpu.mmu.sound_controller.execute_cycle();

        self.cycle_count = self.cycle_count.wrapping_add(1);
    }

    pub fn run(&mut self) {
        let mut benchmark_time = Instant::now();
        let mut count = 0;

        loop {
            if LIMIT_SPEED && self.cpu.mmu.sound_controller.buffer_full() {
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
