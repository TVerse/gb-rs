#![allow(unused_variables)]
#![allow(unused_imports)]
#![feature(slice_flatten)]

use std::error::Error;
use std::fs;
use std::fs::File;
use std::path::Path;
use std::sync::atomic::AtomicUsize;

use clap::{ArgEnum, Parser};
use glium::{glutin, Surface};
use lib_gb_rs::{
    parse_into_cartridge, ArithmeticOperation, Buffer, Color, CommonRegister, ExecutionEvent,
    GameBoy, Immediate8, Instruction, Register16, Register8, RotationShiftOperation,
};
use simplelog::*;

use crate::glutin::event::{DeviceEvent, Event};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, arg_enum, default_value_t = LogLevel::Info)]
    file_log_level: LogLevel,

    #[clap(short, long, arg_enum, default_value_t = LogLevel::Info)]
    console_log_level: LogLevel,

    #[clap(default_value_t = String::from("vendored_test_roms/blargg/instr_timing/instr_timing.gb"))]
    path: String,
}

#[derive(ArgEnum, Copy, Clone, Debug)]
enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    fn as_level_filter(&self) -> LevelFilter {
        match self {
            LogLevel::Trace => LevelFilter::Trace,
            LogLevel::Debug => LevelFilter::Debug,
            LogLevel::Info => LevelFilter::Info,
            LogLevel::Warn => LevelFilter::Warn,
            LogLevel::Error => LevelFilter::Error,
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = Args::parse();

    CombinedLogger::init(vec![
        TermLogger::new(
            args.console_log_level.as_level_filter(),
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            args.file_log_level.as_level_filter(),
            Config::default(),
            File::create("../../gb_rs.log").unwrap(),
        ),
    ])
    .unwrap();

    let path = args.path;

    let event_loop = glutin::event_loop::EventLoop::new();
    let wb = glutin::window::WindowBuilder::new();
    let cb = glutin::ContextBuilder::new().with_vsync(false);
    let display = glium::Display::new(wb, cb, &event_loop)?;

    event_loop.run(move |event, _target, control_flow| {
        let next_frame_time = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 1.0, 1.0);
        target.finish().unwrap();

        *control_flow = glutin::event_loop::ControlFlow::WaitUntil(next_frame_time);
        log::warn!("Waiting for next frame");

        match event {
            Event::WindowEvent {
                event: glutin::event::WindowEvent::CloseRequested,
                ..
            } => {
                log::warn!("Closing window");
                *control_flow = glutin::event_loop::ControlFlow::Exit;
            }
            Event::DeviceEvent {
                event: DeviceEvent::Key(ki),
                ..
            } => {
                log::info!("Got key: {:?}, {:?}", ki.virtual_keycode, ki.state);
            }
            _ => (),
        }
    });
}
