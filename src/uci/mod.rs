use std::{
    io::{Stdin, Stdout, Write, stdin, stdout},
    sync::Mutex,
};

use crate::position::Position;

const BUILD_NAME: &str = env!("CARGO_PKG_NAME");
const BUILD_VERSION: &str = env!("CARGO_PKG_VERSION");
const BUILD_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
// const BUILD_ABOUT: &str = "Simple rust chess engine that will get better";
// if accessible, you know the engine is in one of these states

pub struct UciShell {
    s_in: Stdin,
    s_out: Stdout,

    // state will be locked during critical commands
    state: Mutex<State>,
    debug: bool,
    position: Position, // TODO add here internal configuration
}

pub trait UciParser {
    fn parse(line: String) -> Result<ParsedCommand, ()>;
    fn send_response(&mut self, r: UciResponse) -> Result<(), std::io::Error>;
}

impl UciShell {
    pub fn new() -> Self {
        Self {
            s_in: stdin(),
            s_out: stdout(),

            state: Mutex::new(State::Free),
            debug: true,
            position: Position::startingpos(),
        }
    }
}
impl UciParser for UciShell {
    fn parse(line: String) -> Result<ParsedCommand, ()> {
        let mut parsed = line.split_whitespace();
        match parsed.nth(0) {
            None => Err(()), // no command passed TODO!
            Some(x) => match x {
                "uci" => Ok(ParsedCommand::Uci),
                "isready" => Ok(ParsedCommand::IsReady),
                "printboard" => Ok(ParsedCommand::PrintBoard),

                "position" => Ok(ParsedCommand::Position(
                    // parse fen | starting pos
                    match parsed.nth(0) {
                        None => {
                            return Err(());
                        }
                        Some(pos) => match pos {
                            "startpos" => Position::startingpos(),
                            "fen" => match Position::extract_fen(&mut parsed) {
                                None => return Err(()), //("error while parsing FEN data");
                                Some(p) => p,
                            },
                            _ => return Err(()), // ("unknown position type"),
                        },
                    },
                    // parse moves
                    match parsed.nth(0) {
                        Some("moves") | Some("move") => {
                            let mut v: Vec<String> = Vec::new();
                            for move_notation in parsed {
                                v.push(move_notation.to_string());
                            }
                            Some(v)
                        }
                        None => None,
                        _ => {
                            return Err(()); // no valid argument
                        }
                    },
                )),

                "go" => Ok(ParsedCommand::Go(match parsed.nth(0) {
                    Some("perft") => GoCommand::Perft(match parsed.nth(0) {
                        Some(i) => {
                            let i = i.parse::<usize>().expect("Should have been int");
                            i
                        }
                        _ => return Err(()), // self.debug_msg("Missing depth");
                    }),
                    _ => todo!(),
                })),

                "quit" => Ok(ParsedCommand::Quit),

                _ => Err(()), // return self.failed_parsing_behavior("unsupported command."),
            },
        }
    }

    fn send_response(&mut self, r: UciResponse) -> Result<(), std::io::Error> {
        match r {
            UciResponse::Debug(x) => {
                if self.debug {
                    writeln!(self.s_out, "info string \"{}\"", x)
                } else {
                    Ok(())
                }
            }
            UciResponse::String(x) => writeln!(self.s_out, "{}", x),
            UciResponse::Id(x, y) => writeln!(self.s_out, "id {x} {y}"),
            UciResponse::Ok => writeln!(self.s_out, "uciok"),
            UciResponse::Ready => writeln!(self.s_out, "uciok"),
        }
    }
}

pub enum ParsedCommand {
    Uci,
    IsReady,
    Position(Position, Option<Vec<String>>),
    Go(GoCommand),
    Quit,
    // non standard ones :
    PrintBoard,
}

pub enum GoCommand {
    Perft(usize),
}

pub enum UciResponse {
    String(String),
    Debug(String), //TODO : add more and never use println elsewhere
    Id(&'static str, String),
    Ok,
    Ready,
}

enum State {
    Free,
    Computing,
    Quit,
}

impl UciShell {
    // blocking until quit is recieved
    pub fn run(&mut self) {
        // TODO: might thread this
        loop {
            let mut buffer = String::new();
            self.s_in.read_line(&mut buffer).expect("Unknown error.");
            let x = Self::parse(buffer);

            let sigquit = match x {
                Ok(x) => self.runcommand(x).unwrap(),
                Err(_) => false, // { std::io::Write::write_fmt(&mut self.s_out.lock(), Arguments:: "Error: {:?}", x); false }
            };
            if sigquit {
                return;
            }
        }
    }

    fn wait_ready(&mut self) {
        let _unused = self.state.lock().unwrap();
    }

    // returns response
    pub fn runcommand(&mut self, c: ParsedCommand) -> Result<bool, Box<dyn std::error::Error>> {
        match c {
            ParsedCommand::Uci => {
                self.send_response(UciResponse::Id(
                    "name",
                    format!("{} {}", BUILD_NAME, BUILD_VERSION),
                ))?;
                self.send_response(UciResponse::Id("authors", format!("{}", BUILD_AUTHORS)))?;
                // TODO: self.send_response(UciResponse:: &format!("option name UCI_EngineAbout {}", BUILD_ABOUT));
                self.send_response(UciResponse::Ok)?;
            }

            ParsedCommand::IsReady => {
                // wait for running commands
                self.wait_ready();
                self.send_response(UciResponse::Ready)?;
            }

            ParsedCommand::PrintBoard => {
                println!("Current board state (for debug purposes only!)");
                self.position.pretty_print();
            }

            ParsedCommand::Position(p, m) => {
                // parse fen | starting pos
                self.position = p;
                match m {
                    Some(mv) => {
                        for m in mv {
                            match self.position.getmove(&m) {
                                Err(()) => {
                                    panic!("position was illegal to begin with");
                                }
                                Ok(None) => {
                                    panic!("did not manage to play move");
                                }
                                Ok(Some(m)) => self.position.stack(&m),
                            }
                        }
                    }
                    None => (),
                }
            }

            ParsedCommand::Go(x) => match x {
                GoCommand::Perft(i) => {
                    let c = self.position.perft_top(i);
                    self.send_response(UciResponse::String("".to_string()))?;
                    self.send_response(UciResponse::String(format!("Nodes searched : {}", c)))?;
                    self.send_response(UciResponse::String("".to_string()))?;
                }
            },

            ParsedCommand::Quit => {
                return Ok(true);
            }
        };
        return Ok(false);
    }
}
