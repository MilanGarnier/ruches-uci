
pub mod eval;
pub mod position;
pub mod test;

mod uci;




fn main() {
    let mut interface = uci::UciInterface::create();
    interface.run();

    //println!("Running unit tests");
    //crate::test::run();
}
