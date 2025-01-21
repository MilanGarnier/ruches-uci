#![feature(random)]
#![feature(ptr_as_ref_unchecked)]
#![feature(test)]
#![feature(async_closure)]
#![feature(impl_trait_in_assoc_type)]
#![feature(associated_type_defaults)]

#[cfg(debug_assertions)]
use std::io::Write;
use std::time::SystemTime;
use std::{io::Stdout, sync::LazyLock};

use uci::{UciOut, UciShell};
pub mod algorithms;
pub mod prelude;
use prelude::*;
pub mod bitboard;
pub mod eval;
#[deprecated]
pub mod localvec;
pub mod piece;
pub mod player;
pub mod position;
pub mod search;
pub mod tt; // transposition tables
pub mod uci;

static INTERFACE: LazyLock<UciShell> = LazyLock::new(|| uci::UciShell::new());

extern crate enum_iterator;

static START_TIME: LazyLock<SystemTime> = LazyLock::new(|| SystemTime::now());

#[cfg(debug_assertions)]
fn loglevel() -> log::LevelFilter {
    let x = std::env::var("LOG");
    match x {
        Ok(level) => match level.as_str() {
            "TRACE" => {
                println!("TRACE");
                log::LevelFilter::Trace
            }
            "DEBUG" => {
                println!("DEBUG");
                log::LevelFilter::Debug
            }
            "INFO" => {
                println!("INFO");
                log::LevelFilter::Info
            }
            "WARN" => {
                println!("WARN");
                log::LevelFilter::Warn
            }
            "ERROR" => {
                println!("ERROR");
                log::LevelFilter::Error
            }
            _ => {
                println!("Info is the default log level.");
                log::LevelFilter::Info
            }
        },
        Err(_) => log::LevelFilter::Error,
    }
}

#[cfg(debug_assertions)]
fn setup_logger() {
    env_logger::Builder::new()
        .filter_level(loglevel())
        .format(|buf, record| {
            writeln!(
                buf,
                "{}:{} {} \t [{}] - {}",
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                std::time::SystemTime::now()
                    .duration_since(START_TIME.clone())
                    .unwrap()
                    .as_secs(),
                record.level(),
                record.args()
            )
        })
        .init();
}

#[cfg(not(debug_assertions))]
fn setup_logger() {
    ();
}

#[tokio::main/*(flavor = "current_thread")*/]
async fn main() {
    let _ = START_TIME.clone();
    setup_logger();
    log::debug!("Debug logging enabled"); // Test
    let mut args: Vec<_> = std::env::args().collect();

    // either single command or multiple command
    if args.len() > 2 {
        args.remove(0); // ignore path
        let mut command = String::new();
        for x in args {
            command += &x;
            command += " ";
        }
        //println!("Running command {:?}", command);
        INTERFACE
            .runcommand::<UciOut<Stdout>>(uci::parse(command).unwrap())
            .await
            .unwrap();
    } else {
        INTERFACE.run::<UciOut<Stdout>>().await;
    }
}
