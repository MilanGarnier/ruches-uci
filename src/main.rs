#![feature(random)]

pub mod eval;
pub mod position;
mod uci;

#[tokio::main]
async fn main() {
    position::movegen::static_attacks::init_attack_tables().await;
    let mut args: Vec<_> = std::env::args().collect();

    let mut interface = uci::UciInterface::create();

    // either single command or multiple command
    if args.len() > 2 {
        args.remove(0); // ignore path
        let mut command = String::new();
        for x in args {
            command += &x;
            command += " ";
        }
        println!("Running command {:?}", command);
        interface.runcommand(&command);
    } else {
        interface.run();
    }

    //println!("Running unit tests");
    //crate::test::run();
}
