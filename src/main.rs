#![feature(random)]
#![feature(once_cell_get_mut)]
#![feature(ptr_as_ref_unchecked)]
#![feature(test)]

use uci::{UciParser, UciShell};
pub mod eval;
pub mod position;
pub mod tt; // transposition tables
mod uci;

fn main() {
    //println!("Zobrist key {:?}", position::zobrist::random_zobrist_seed());

    let mut args: Vec<_> = std::env::args().collect();

    let mut interface = uci::UciShell::new();

    // either single command or multiple command
    if args.len() > 2 {
        args.remove(0); // ignore path
        let mut command = String::new();
        for x in args {
            command += &x;
            command += " ";
        }
        println!("Running command {:?}", command);
        interface.runcommand(UciShell::parse(command).unwrap());
    } else {
        interface.run();
    }

    //println!("Running unit tests");
    //crate::test::run();
}
