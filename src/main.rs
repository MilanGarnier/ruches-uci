#![feature(random)]
#![feature(ptr_as_ref_unchecked)]
#![feature(test)]
#![feature(async_closure)]

#[macro_use]
extern crate derive_error;
#[macro_use]
extern crate log;

pub mod prelude;
use prelude::*;

use std::sync::LazyLock;

use uci::UciShell;

#[macro_use]
pub mod output_mgr;
pub mod eval;
pub mod localvec;
pub mod position;
pub mod search;
pub mod tt; // transposition tables
mod uci;

static INTERFACE: LazyLock<UciShell> = LazyLock::new(|| uci::UciShell::new());

#[tokio::main/*(flavor = "current_thread")*/]

async fn main() {
    output_mgr::init();
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
            .runcommand(uci::parse(command).unwrap())
            .await
            .unwrap();
    } else {
        INTERFACE.run().await;
    }
}
