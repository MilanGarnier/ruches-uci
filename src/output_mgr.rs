use crate::prelude::*;

pub use log::{Level, LevelFilter, Record};

use std::ops::DerefMut;
use std::{fmt::Display, sync::RwLock};

use std::io::{Write, stderr, stdout};

pub static OUT: OutputManager = OutputManager::default();
pub fn init() -> Result<(), log::SetLoggerError> {
    log::set_logger(&OUT).map(|()| log::set_max_level(LevelFilter::Info))
}

#[cfg(feature = "error_backtrace")]
const LOG_LEVEL: Level = Level::Trace;

// change this to trace when inspecting bugs
#[cfg(not(feature = "error_backtrace"))]
#[cfg(debug_assertions)]
const LOG_LEVEL: Level = Level::Debug;

#[cfg(not(debug_assertions))]
const LOG_LEVEL: Level = Level::Info;

pub struct OutputManager {
    runtime_debug: RwLock<bool>,
    #[cfg(feature = "error_backtrace")]
    backtrace: RwLock<bool>,
}

impl log::Log for OutputManager {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        let mut a = metadata.level() <= LOG_LEVEL; // let messages higher than info pass
        a |= (self.runtime_debug.read().unwrap().clone() && metadata.level() == Level::Debug); // allow for runtime debug even if disabled at build

        #[cfg(feature = "error_backtrace")]
        {
            a |= *self.backtrace.read().unwrap();
        }
        a
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            match record.metadata().level() {
                Level::Trace => {
                    println!(
                        "[trace - {}:{}:1:{}] {}",
                        record.file().unwrap(),
                        record.line().unwrap(),
                        record.module_path().unwrap(),
                        record.args()
                    )
                }
                Level::Debug => {
                    #[cfg(debug_assertions)]
                    println!(" info string [debug] {}", record.args());
                    #[cfg(not(debug_assertions))]
                    println!("info string {}", record.args());
                }
                Level::Error => println!(
                    "info string [error - {}:{}:1] {}",
                    record.file().unwrap(),
                    record.line().unwrap(),
                    record.args()
                ),
                Level::Warn => println!(
                    "info string [warn  - {}:{}:1] {}",
                    record.file().unwrap(),
                    record.line().unwrap(),
                    record.args()
                ),
                _ => println!("{}", record.args()),
            }
        }
        #[cfg(feature = "error_backtrace")]
        {
            if (record.level() == Level::Error) {
                let a = self.backtrace.write();
                *a.expect("poisoned").deref_mut() = true;
            }
        }
    }

    fn flush(&self) {
        stdout().flush().unwrap();
    }
}

impl OutputManager {
    const fn default() -> Self {
        Self {
            runtime_debug: RwLock::new(cfg!(debug_assertions)),
            #[cfg(feature = "error_backtrace")]
            backtrace: RwLock::new(false),
        }
    }
    #[deprecated]
    pub fn send_response<T: Display>(&self, r: T) -> Result<(), std::io::Error> {
        write!(SelectStream::<OutputResponse>::get(self), "{r}")
    }
    #[deprecated]
    pub fn send_debug<T: Display>(&self, r: T) -> Result<(), std::io::Error> {
        write!(SelectStream::<OutputDebug>::get(self), "{r}")
    }
}

pub fn out<L: OutputLevel>(_l: L) -> impl Write
where
    OutputManager: SelectStream<L>,
{
    SelectStream::<L>::get(&OUT)
}

pub trait OutputLevel {}

#[derive(Default)]
pub struct OutputResponse {}

#[derive(Default)]
pub struct OutputDebug {}

#[derive(Default)]
pub struct OutputDev {}

#[derive(Default)]
pub struct OutputMetrics {}

#[derive(Default)]
pub struct OutputErr {}
impl OutputLevel for OutputResponse {}
impl OutputLevel for OutputDebug {}
impl OutputLevel for OutputErr {}
impl OutputLevel for OutputDev {}
impl OutputLevel for OutputMetrics {}

// Default
pub trait SelectStream<Lvl: OutputLevel>: Send {
    fn get(&self) -> impl std::io::Write {
        std::io::sink()
    } //
}

impl SelectStream<OutputResponse> for OutputManager {
    fn get(&self) -> impl std::io::Write {
        stdout()
    }
}

impl SelectStream<OutputDebug> for OutputManager {
    fn get(&self) -> impl std::io::Write {
        let d = self.runtime_debug.read().unwrap().clone();
        DebugStream::from((d, stdout()))
    }
}

impl SelectStream<OutputErr> for OutputManager {
    fn get(&self) -> impl Write {
        stderr()
    }
}

impl SelectStream<OutputMetrics> for OutputManager {
    fn get(&self) -> impl Write {
        todo!("Metric stream not implemented");
        #[allow(unreachable_code)]
        std::io::sink()
    }
}

impl SelectStream<OutputDev> for OutputManager {
    fn get(&self) -> impl Write {
        #[cfg(debug_assertions)]
        {
            let a = self.runtime_debug.read().unwrap().clone();
            DebugStream::from((a, stderr()))
        }
        #[cfg(not(debug_assertions))]
        {
            std::io::sink()
        }
    }
}

fn check_if_output<T: OutputLevel>() {}

struct DebugStream<O: Write> {
    o: RwLock<Option<O>>,
}
impl<O: Write> From<(bool, O)> for DebugStream<O> {
    fn from((debug, out): (bool, O)) -> Self {
        match debug {
            true => Self {
                o: RwLock::new(Some(out)),
            },
            false => Self {
                o: RwLock::new(None),
            },
        }
    }
}

impl<O: Write> Write for DebugStream<O> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match &mut self.o.write().unwrap().as_mut() {
            Some(x) => {
                write!(x, "info String \"")?;
                let a = x.write(buf)?;
                writeln!(x, "\"")?;
                Ok(a)
            }
            None => Ok(0),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match &mut self.o.write().unwrap().as_mut() {
            Some(x) => x.flush(),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
pub struct NullUciStream {}
#[cfg(test)]
impl<O: OutputLevel> SelectStream<O> for NullUciStream {}
