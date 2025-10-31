pub mod cpu;
pub mod opcodes;
pub mod bus;
pub mod cartridge;
pub mod trace;
pub mod ppu;
pub mod render;
pub mod joypad;
pub mod apu;
pub mod audio;

use cpu::CPU;
use bus::Bus;
use cartridge::Rom;
use render::frame::Frame;
use ppu::NesPPU;
use audio::AudioOutput;

use minifb::{Key, Window, WindowOptions};

use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate bitflags;

fn main() {
    // Init minifb window
    let mut window = Window::new(
        "NES Emulator - minifb",
        Frame::WIDTH,
        Frame::HEIGHT,
        WindowOptions {
            resize: false,
            scale: minifb::Scale::X2,
            ..WindowOptions::default()
        },
    )
    .unwrap_or_else(|e| {
        panic!("Failed to create window: {}", e);
    });

    // Limit to max ~60 fps update rate
    window.set_target_fps(60);

    // Initialize audio output
    let audio_output = Rc::new(RefCell::new(
        AudioOutput::new().unwrap_or_else(|e| {
            eprintln!("Warning: Failed to initialize audio: {}", e);
            eprintln!("Continuing without audio...");
            panic!("Audio initialization failed");
        })
    ));

    // Load ROM
    let bytes: Vec<u8> = std::fs::read("roms/mario.nes").unwrap();
    // let bytes: Vec<u8> = std::fs::read("roms/hello.nes").unwrap();
    let rom = Rom::new(&bytes).unwrap();
    let mut frame = Frame::new();

    let mut key_map = HashMap::new();
    key_map.insert(Key::Down, joypad::JoypadButton::DOWN);
    key_map.insert(Key::Up, joypad::JoypadButton::UP);
    key_map.insert(Key::Right, joypad::JoypadButton::RIGHT);
    key_map.insert(Key::Left, joypad::JoypadButton::LEFT);
    key_map.insert(Key::Backspace, joypad::JoypadButton::SELECT);
    key_map.insert(Key::Enter, joypad::JoypadButton::START);
    key_map.insert(Key::Z, joypad::JoypadButton::BUTTON_A);
    key_map.insert(Key::X, joypad::JoypadButton::BUTTON_B);

    // run the game cycle
    let bus = Bus::new(rom, move |ppu: &NesPPU, joypad: &mut joypad::Joypad| {
        render::render(ppu, &mut frame);
        
        // Update window with frame buffer
        window.update_with_buffer(&frame.data, Frame::WIDTH, Frame::HEIGHT)
            .unwrap_or_else(|e| {
                eprintln!("Failed to update window: {}", e);
            });

        // Check if window is still open
        if !window.is_open() || window.is_key_down(Key::Escape) {
            std::process::exit(0);
        }

        // Handle key presses
        // Reset all button states first
        for button in [
            joypad::JoypadButton::DOWN,
            joypad::JoypadButton::UP,
            joypad::JoypadButton::RIGHT,
            joypad::JoypadButton::LEFT,
            joypad::JoypadButton::SELECT,
            joypad::JoypadButton::START,
            joypad::JoypadButton::BUTTON_A,
            joypad::JoypadButton::BUTTON_B,
        ] {
            joypad.set_button_pressed_status(button, false);
        }

        // Set pressed buttons
        let keys = window.get_keys();
        for key in keys {
            if let Some(button) = key_map.get(&key) {
                joypad.set_button_pressed_status(*button, true);
            }
        }
    });

    let mut cpu = CPU::new(bus);

    cpu.reset();
    
    // Run with audio
    cpu.run_with_callback(move |cpu| {
        // Extract audio samples from APU and queue them
        let apu = cpu.bus.apu_mut();
        let samples_available = apu.samples_available();
        
        if samples_available > 0 {
            let mut audio = audio_output.borrow_mut();
            for _ in 0..samples_available {
                let sample = apu.get_sample();
                audio.queue_sample(sample);
            }
        }
    });
}
