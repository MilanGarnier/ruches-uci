#![feature(random)]
#![feature(ptr_as_ref_unchecked)]
#![feature(test)]
#![feature(async_closure)]
//#![feature(impl_trait_in_assoc_type)]
#![feature(associated_type_defaults)]

use log::LevelFilter;
use std::io::Write;
use std::{io::Stdout, sync::LazyLock};

use uci::{UciOut, UciShell};

pub mod prelude;
use prelude::*;
pub mod bitboard;
pub mod eval;
pub mod localvec;
pub mod piece;
pub mod player;
pub mod position;
pub mod search;
pub mod tt; // transposition tables
pub mod uci;

static INTERFACE: LazyLock<UciShell> = LazyLock::new(|| uci::UciShell::new());

extern crate enum_iterator;

#[tokio::main/*(flavor = "current_thread")*/]
async fn main() {
    env_logger::Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{}:{} {} [{}] - {}",
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                record.level(),
                record.args()
            )
        })
        .filter(Some("logger_example"), LevelFilter::Debug)
        .init();

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
