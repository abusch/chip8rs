use std::path::Path;

use anyhow::{bail, Context, Result};
use clap::{App, Arg};
use game_loop::game_loop;
use log::{error, info};
use pixels::{Pixels, SurfaceTexture};
use winit::{
    dpi::LogicalSize,
    event::{Event, VirtualKeyCode},
    event_loop::EventLoop,
    window::WindowBuilder,
};
use winit_input_helper::WinitInputHelper;

mod config;
mod cpu;
mod gfx;
mod interconnect;
mod ram;

use cpu::Cpu;
use gfx::Gfx;
use interconnect::Interconnect;
use ram::Ram;

const WIDTH: usize = 64;
const HEIGHT: usize = 32;

/// This represents the Chip-8 virtual machine. It is composed of a `Cpu` and an `Interconnect`.
pub struct Chip8 {
    cpu: Cpu,
    interconnect: Interconnect,
    ticks: u64,
}

impl Chip8 {
    pub fn new<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let rom = std::fs::read(path)?;
        let mut ram = Ram::default();
        ram.load_at(config::FONT_DATA_ADDR, &config::FONT_DATA[..]);
        ram.load_at(config::PROG_ADDR, &rom);

        Ok(Self {
            cpu: Cpu::new(),
            interconnect: Interconnect {
                ram,
                gfx: Gfx::new(),
                delay_timer: 0,
                sound_timer: 0,
                keys: [false; 16],
            },
            ticks: 0,
        })
    }

    pub fn gfx_buffer(&mut self) -> &[u8] {
        self.interconnect.gfx.get_frame()
    }

    pub fn set_key(&mut self, key: u8, is_down: bool) {
        self.interconnect.keys[key as usize] = is_down;
    }

    pub fn step(&mut self) {
        self.ticks += 1;
        self.cpu.emulate_cycle(&mut self.interconnect);
        if self.ticks == 16 {
            self.interconnect.tick();
            self.ticks = 0;
        }
    }
}

pub struct Game {
    chip8: Chip8,
    pixels: Pixels,
    input: WinitInputHelper,
}

impl Game {
    pub fn new(pixels: Pixels, chip8: Chip8) -> Result<Self> {
        let input = WinitInputHelper::new();
        Ok(Self {
            chip8,
            pixels,
            input,
        })
    }

    pub fn update(&mut self) {
        self.chip8.step();
    }

    pub(crate) fn update_controls(&mut self, event: &Event<()>) {
        self.input.update(event);
        for (i, key) in KEYS.iter().enumerate() {
            self.chip8.set_key(i as u8, self.input.key_held(*key));
        }
    }
}

fn main() -> Result<()> {
    env_logger::init();

    let app = App::new("chip8rs")
        .author("Antoine Busch")
        .version("0.1")
        .arg(Arg::new("ROM").index(1).required(true))
        .arg(
            Arg::new("scale")
                .required(false)
                .default_value("8")
                .possible_values(&["1", "2", "4", "8", "16", "32"])
                .short('s')
                .long("scale"),
        )
        .get_matches();

    let rom = app.value_of("ROM").expect("Missing ROM file");
    let scale = match app.value_of("scale").context("Missing scale")? {
        "1" => 1.0,
        "2" => 2.0,
        "4" => 4.0,
        "8" => 8.0,
        "16" => 16.0,
        "32" => 32.0,
        _ => bail!("Invalid scale factor"),
    };

    info!("loading rom {}", rom);
    let chip8 = Chip8::new(rom)?;

    let event_loop = EventLoop::new();
    let window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        let scaled_size = LogicalSize::new(WIDTH as f64 * scale, HEIGHT as f64 * scale);
        WindowBuilder::new()
            .with_title("Chip8rs -- Chip8 Emulator")
            .with_inner_size(scaled_size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(WIDTH as u32, HEIGHT as u32, surface_texture)?
    };

    let game = Game::new(pixels, chip8)?;

    game_loop(
        event_loop,
        window,
        game,
        1000,
        0.1,
        |g| {
            /* update */
            g.game.update();
        },
        |g| {
            /* render */
            if g.game.chip8.interconnect.gfx.dirty {
                g.game
                    .pixels
                    .get_frame()
                    .chunks_exact_mut(4)
                    .zip(g.game.chip8.gfx_buffer().iter())
                    .for_each(|(b, v)| {
                        if *v == 0 {
                            b.copy_from_slice(&[0, 0, 0, 0])
                        } else {
                            b.copy_from_slice(&[255, 255, 255, 255])
                        }
                    });
                if let Err(e) = g.game.pixels.render() {
                    error!("Render error: {}", e);
                    g.exit();
                }
            }
        },
        |g, event| {
            g.game.update_controls(&event);
            // Close events
            if g.game.input.key_pressed(VirtualKeyCode::Escape) || g.game.input.quit() {
                g.exit();
            }
        },
    );
}

const KEYS: [VirtualKeyCode; 16] = [
    VirtualKeyCode::X,
    VirtualKeyCode::Key1,
    VirtualKeyCode::Key2,
    VirtualKeyCode::Key3,
    VirtualKeyCode::Q,
    VirtualKeyCode::W,
    VirtualKeyCode::E,
    VirtualKeyCode::A,
    VirtualKeyCode::S,
    VirtualKeyCode::D,
    VirtualKeyCode::Z,
    VirtualKeyCode::C,
    VirtualKeyCode::Key4,
    VirtualKeyCode::R,
    VirtualKeyCode::F,
    VirtualKeyCode::V,
];
