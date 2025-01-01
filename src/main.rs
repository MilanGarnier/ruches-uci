#![feature(random)]
#![feature(ptr_as_ref_unchecked)]
#![feature(test)]
#![feature(async_closure)]

use std::io::{stdin, stdout};

pub mod eval;
pub mod position;
pub mod tt; // transposition tables
mod uci;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    //println!("Zobrist key {:?}", position::zobrist::random_zobrist_seed());

    let mut args: Vec<_> = std::env::args().collect();

    let mut interface = uci::UciShell::new(stdin(), stdout());

    // either single command or multiple command
    if args.len() > 2 {
        args.remove(0); // ignore path
        let mut command = String::new();
        for x in args {
            command += &x;
            command += " ";
        }
        println!("Running command {:?}", command);
        interface.runcommand(uci::parse(command).unwrap()).unwrap();
    } else {
        interface.run().await;
    }

    //println!("Running unit tests");
    //crate::test::run();
}
