use crate::position::{movegen::static_attacks::Lookup, Position};
use std::{ io::stdin, sync::Mutex};

const BUILD_NAME: &str = env!("CARGO_PKG_NAME");
const BUILD_VERSION: &str = env!("CARGO_PKG_VERSION");
const BUILD_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const BUILD_ABOUT: &str = "Simple rust chess engine that will get better";
// if accessible, you know the engine is in one of these states
enum State {
    Free,
    Computing,
    Quit,
}

pub struct UciInterface<'a> {
    // state will be locked during critical commands
    state: Mutex<State>,
    debug: bool,
    position: Position, // TODO add here internal configuration
    runtime : &'a Lookup,
}

impl<'a> UciInterface<'a> {
    pub fn create(runtime : &'a Lookup ) -> Self {
        Self {
            state: Mutex::new(State::Free),
            debug: true,
            position: Position::startingpos(),
            runtime : runtime
        }
    }
    // blocking until quit is recieved
    pub fn run(&mut self) {
        // TODO: might thread this
        loop {
            let mut buffer = String::new();
            stdin().read_line(&mut buffer).expect("Unknown error.");
            let sigquit = self.runcommand(&buffer.trim_end().to_string());
            if sigquit {
                return;
            }
        }
    }

    fn debug_msg(&self, s: &str) {
        if self.debug {
            println!("info string {}", s)
        }
    }
    fn wait_ready(&mut self) {
        let _unused = self.state.lock().unwrap();
    }
    fn failed_parsing_behavior(&self, reason: &str) -> bool {
        self.debug_msg(&format!("Error while parsing : {}", reason));
        false
    }
    fn respond(&self, s: &String) {
        println!("{}", s)
    }
    fn respond_str(&self, s: &str) {
        println!("{}", s)
    }

    // returns response
    pub fn runcommand(&mut self, s: &String) -> bool {
        let mut parsed = s.split_whitespace();
        match parsed.nth(0) {
            None => return self.failed_parsing_behavior("no command provided"),
            Some(x) => match x {
                "uci" => {
                    self.respond(&format!("id name {} {}", BUILD_NAME, BUILD_VERSION));
                    self.respond(&format!("id authors {}", BUILD_AUTHORS));
                    self.respond(&format!("option name UCI_EngineAbout {}", BUILD_ABOUT));
                    self.respond_str("uciok")
                }

                "isready" => {
                    // wait for running commands
                    self.wait_ready();
                    self.respond_str("readyok")
                }

                "printboard" => {
                    println!("Current board state (for debug purposes only!)");
                    self.position.pretty_print();
                }

                "position" => {
                    // parse fen | starting pos
                    match parsed.nth(0) {
                        None => {
                            return self.failed_parsing_behavior("no position data");
                        }
                        Some(pos) => match pos {
                            "startpos" => {
                                self.position = Position::startingpos();
                            }
                            "fen" => match Position::extract_fen(&mut parsed) {
                                None => {
                                    return self
                                        .failed_parsing_behavior("error while parsing FEN data");
                                }
                                Some(p) => {
                                    self.position = p;
                                }
                            },
                            _ => return self.failed_parsing_behavior("unknown position type"),
                        },
                    }
                    // parse moves
                    match parsed.nth(0) {
                        Some("moves") | Some("move") => {
                            for move_notation in parsed {
                                match self.position.getmove(move_notation, &self.runtime) {
                                    Err(())=> {
                                        return self.failed_parsing_behavior(
                                            "position was illegal to begin with",
                                        );
                                    }
                                    Ok(None) => {
                                        return self.failed_parsing_behavior(
                                            "did not manage to play move",
                                        );
                                    }
                                    Ok(Some(m)) => self.position.stack(&m),
                                }
                            }
                        }
                        _ => {
                            return false;
                        }
                    }
                }

                "go" => match parsed.nth(0) {
                    Some("perft") => match parsed.nth(0) {
                        Some(i) => {
                            let i = i.parse::<usize>().expect("Should have been int");
                            let c = self.position.perft_top(i, &self.runtime);
                            self.respond(&format!(""));
                            self.respond(&format!("Nodes searched : {}", c));
                            self.respond(&format!(""));
                        }
                        _ => {
                            self.debug_msg("Missing depth");
                        }
                    },
                    _ => todo!(),
                },

                "quit" => {
                    return true;
                }

                _ => return self.failed_parsing_behavior("unsupported command."),
            },
        };
        false
    }
}
