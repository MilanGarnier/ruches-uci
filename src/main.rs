#![feature(random)]
#![feature(ptr_as_ref_unchecked)]
#![feature(test)]
#![feature(async_closure)]

use std::{
    io::Stdout,
    sync::LazyLock,
};

use uci::{UciOut, UciShell};

pub mod eval;
pub mod localvec;
pub mod position;
pub mod search;
pub mod tt; // transposition tables
mod uci;

static INTERFACE: LazyLock<UciShell> = LazyLock::new(|| uci::UciShell::new());

#[tokio::main/*(flavor = "current_thread")*/]

async fn main() {
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
