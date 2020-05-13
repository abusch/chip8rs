use std::path::Path;

use clap::{App, Arg};
use log::info;
use minifb::{Key, Scale, ScaleMode, Window, WindowOptions};

mod config;
mod cpu;
mod gfx;
mod interconnect;
mod ram;

use cpu::Cpu;
use gfx::Gfx;
use interconnect::Interconnect;
use ram::Ram;

/// This represents the Chip-8 virtual machine. It is composed of a `Cpu` and an `Interconnect`.
struct Chip8 {
    cpu: Cpu,
    interconnect: Interconnect,
}

impl Chip8 {
    pub fn new<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let rom = std::fs::read(path)?;
        let mut ram = Ram::default();
        ram.load_at(config::FONT_DATA_ADDR, &FONT_DATA[..]);
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
        })
    }

    pub fn gfx_buffer(&self) -> &[u8] {
        self.interconnect.gfx.get_frame()
    }

    pub fn set_key(&mut self, key: u8, is_down: bool) {
        self.interconnect.keys[key as usize] = is_down;
    }

    pub fn step(&mut self) {
        self.cpu.emulate_cycle(&mut self.interconnect);
    }
}

fn main() {
    const WIDTH: usize = 64;
    const HEIGHT: usize = 32;

    env_logger::try_init().unwrap();

    let app = App::new("chip8rs")
        .author("Antoine Busch")
        .version("0.1")
        .arg(Arg::with_name("ROM").index(1).required(true))
        .arg(
            Arg::with_name("scale")
                .required(false)
                .default_value("4")
                .possible_values(&["1", "2", "4", "8", "16", "32"])
                .short("s")
                .long("scale"),
        )
        .get_matches();

    let rom = app.value_of("ROM").expect("Missing ROM file");
    let scale = match app.value_of("scale").unwrap() {
        "1" => Scale::X1,
        "2" => Scale::X2,
        "4" => Scale::X4,
        "8" => Scale::X8,
        "16" => Scale::X16,
        "32" => Scale::X32,
        _ => panic!("Invalid scale factor"),
    };

    info!("loading rom {}", rom);
    let mut chip8 = Chip8::new(rom).unwrap();

    let mut buffer = vec![0u32; WIDTH * HEIGHT];
    let mut window = Window::new(
        "Test - ESC to exit",
        WIDTH,
        HEIGHT,
        WindowOptions {
            borderless: false,
            title: true,
            resize: false,
            scale,
            scale_mode: ScaleMode::Stretch,
            topmost: false,
        },
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    // Limit to max ~60 fps update rate
    // window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    while window.is_open() && !window.is_key_down(Key::Escape) {
        for (i, key) in KEYS.iter().enumerate() {
            chip8.set_key(i as u8, window.is_key_down(*key));
        }
        chip8.step();
        buffer
            .iter_mut()
            .zip(chip8.gfx_buffer().iter())
            .for_each(|(b, v)| *b = if *v == 0 { 0u32 } else { 0xFFFFFFFF });

        // We unwrap here as we want this code to exit if it fails. Real applications may want to handle this in a different way
        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }
}

const FONT_DATA: [u8; 5 * 16] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, 0x20, 0x60, 0x20, 0x20, 0x70, 0xF0, 0x10, 0xF0, 0x80, 0xF0, 0xF0,
    0x10, 0xF0, 0x10, 0xF0, 0x90, 0x90, 0xF0, 0x10, 0x10, 0xF0, 0x80, 0xF0, 0x10, 0xF0, 0xF0, 0x80,
    0xF0, 0x90, 0xF0, 0xF0, 0x10, 0x20, 0x40, 0x40, 0xF0, 0x90, 0xF0, 0x90, 0xF0, 0xF0, 0x90, 0xF0,
    0x10, 0xF0, 0xF0, 0x90, 0xF0, 0x90, 0x90, 0xE0, 0x90, 0xE0, 0x90, 0xE0, 0xF0, 0x80, 0x80, 0x80,
    0xF0, 0xE0, 0x90, 0x90, 0x90, 0xE0, 0xF0, 0x80, 0xF0, 0x80, 0xF0, 0xF0, 0x80, 0xF0, 0x80, 0x80,
];

const KEYS: [Key; 16] = [
    Key::Key1,
    Key::Key2,
    Key::Key3,
    Key::Q,
    Key::W,
    Key::E,
    Key::A,
    Key::S,
    Key::D,
    Key::X,
    Key::Z,
    Key::C,
    Key::Key4,
    Key::R,
    Key::F,
    Key::V,
];
