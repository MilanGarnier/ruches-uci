use std::{
    fmt::Display,
    io::{Read, Write, stdin},
    sync::{Arc, Mutex},
};

use crate::position::Position;

const BUILD_NAME: &str = env!("CARGO_PKG_NAME");
const BUILD_VERSION: &str = env!("CARGO_PKG_VERSION");
const BUILD_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
// const BUILD_ABOUT: &str = "Simple rust chess engine that will get better";
// if accessible, you know the engine is in one of these states

pub struct UciShell<I, O>
where
    I: Read,
    O: Write,
{
    s_in: I,
    s_out: Arc<Mutex<O>>,

    // state will be locked during critical commands
    worker: Option<tokio::task::JoinHandle<bool>>,
    debug: bool,
    position: Arc<Mutex<Position>>, // TODO add here internal configuration
}

pub trait UciParser {
    fn send_response(&self, r: UciResponse) -> Result<(), std::io::Error>;
}

impl<I: Read, O: Write> UciShell<I, O> {
    pub fn new(i: I, o: O) -> Self {
        Self {
            s_in: i,
            s_out: Arc::new(Mutex::new(o)),

            worker: None,
            debug: true,
            position: Arc::new(Mutex::new(Position::startingpos())),
        }
    }
}

pub fn parse(line: String) -> Result<ParsedCommand, ()> {
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
                #[cfg(feature = "perft")]
                Some("perft") => GoCommand::Perft(match parsed.nth(0) {
                    Some(i) => {
                        let i = i.parse::<usize>().expect("Should have been int");
                        i
                    }
                    _ => return Err(()), // self.debug_msg("Missing depth");
                }),
                _ => todo!(),
            })),

            "stop" => Ok(ParsedCommand::Stop),
            "quit" => Ok(ParsedCommand::Quit),

            _ => Err(()), // return self.failed_parsing_behavior("unsupported command."),
        },
    }
}

impl<I: Read, O: Write> UciParser for UciShell<I, O> {
    fn send_response(&self, r: UciResponse) -> Result<(), std::io::Error> {
        let mut out_mut = self.s_out.lock().unwrap();
        if let UciResponse::Debug(_) = r {
            if self.debug == false {
                return Ok(());
            }
        }
        writeln!(out_mut, "{r}")
    }
}

pub enum ParsedCommand {
    Uci,
    IsReady,
    Position(Position, Option<Vec<String>>),
    Go(GoCommand),
    Quit,
    Stop,
    // non standard ones :
    PrintBoard,
}

pub enum GoCommand {
    #[cfg(feature = "perft")]
    Perft(usize),
}
enum UciOption {
    String {
        default: String,
    },
    Spin {
        default: usize,
        min: usize,
        max: usize,
    },
    Check {
        default: bool,
    },
}
impl Display for UciOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UciOption::String { default } => write!(f, "type string default {default}"),
            UciOption::Spin { default, min, max } => {
                write!(f, "type spin default {default} min {min} max {max}")
            }
            UciOption::Check { default } => write!(f, "type check default {default}"),
        }
    }
}

pub enum UciResponse {
    String(String),
    Debug(String), //TODO : add more and never use println elsewhere
    Id(&'static str, String),
    Ok,
    Ready,
    Option { name: &'static str, o: UciOption },
}
impl Display for UciResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UciResponse::Debug(x) => {
                write!(f, "info string \"{}\"", x)
            }
            UciResponse::String(x) => write!(f, "{}", x),
            UciResponse::Id(x, y) => write!(f, "id {x} {y}"),
            UciResponse::Ok => write!(f, "uciok"),
            UciResponse::Ready => write!(f, "uciready"),
            UciResponse::Option { name, o } => write!(f, "option name {name} {o}"),
        }
    }
}

pub enum CommandResult {
    Finished(bool),
    Pending(tokio::task::JoinHandle<bool>),
}

impl<I: Read, O: Write> UciShell<I, O> {
    // blocking until quit is recieved
    pub async fn run(&mut self) {
        loop {
            let mut line = String::new();
            stdin().read_line(&mut line).unwrap();
            let command = parse(line).unwrap();
            // .await.expect("Can't read line").unwrap();

            let busy = match &self.worker {
                Some(x) => {
                    if x.is_finished() {
                        self.worker = None;
                        false
                    } else {
                        true
                    }
                }
                None => false,
            };

            let res = self.runcommand(command);

            match res.unwrap() {
                CommandResult::Finished(true) => return,
                CommandResult::Finished(false) => (),
                CommandResult::Pending(h) => {
                    if busy {
                        panic!("Cannot launch new search, busy")
                    } else {
                        self.worker = Some(h)
                    } // TO remove
                }
            }
        }
    }

    // returns response
    pub fn runcommand(
        &self,
        c: ParsedCommand,
    ) -> Result<CommandResult, Box<dyn std::error::Error>> {
        match c {
            ParsedCommand::Quit => return Ok(CommandResult::Finished(true)),
            ParsedCommand::Stop => {
                match &self.worker {
                    Some(x) => {
                        if x.is_finished() == false {
                            x.abort();
                            return Ok(CommandResult::Finished(true));
                        }
                    }
                    None => (),
                }
                {}
            }
            _ => (),
        }
        match c {
            ParsedCommand::Uci => {
                self.send_response(UciResponse::Id(
                    "name",
                    format!("{} {}", BUILD_NAME, BUILD_VERSION),
                ))?;
                self.send_response(UciResponse::Id("authors", format!("{}", BUILD_AUTHORS)))?;
                // TODO: self.send_response(UciResponse:: &format!("option name UCI_EngineAbout {}", BUILD_ABOUT));
                self.send_response(UciResponse::Option {
                    name: "Threads",
                    o: UciOption::Spin {
                        default: 1,
                        min: 1,
                        max: 1024,
                    },
                })?;

                self.send_response(UciResponse::Ok)?;
            }

            ParsedCommand::IsReady => {
                // wait for running commands
                self.send_response(UciResponse::Ready)?;
            }

            ParsedCommand::PrintBoard => {
                println!("Current board state (for debug purposes only!)");
                self.position.lock().unwrap().pretty_print();
            }

            ParsedCommand::Position(p, m) => {
                // parse fen | starting pos
                let mut lock = self.position.lock();
                let pos = lock.as_mut().unwrap();
                pos.clone_from(&p);
                match m {
                    Some(mv) => {
                        for m in mv {
                            match pos.getmove(&m) {
                                Err(()) => {
                                    panic!("position was illegal to begin with");
                                }
                                Ok(None) => {
                                    panic!("did not manage to play move");
                                }
                                Ok(Some(m)) => pos.stack(&m),
                            }
                        }
                    }
                    None => (),
                }
            }

            ParsedCommand::Go(x) => match x {
                #[cfg(feature = "perft")]
                GoCommand::Perft(i) => {
                    let c = self.position.lock().unwrap().perft_top(i);
                    self.send_response(UciResponse::String("".to_string()))?;
                    self.send_response(UciResponse::String(format!("Nodes searched : {}", c)))?;
                    self.send_response(UciResponse::String("".to_string()))?;
                }
            },
            _ => panic!("Should not be able to be here"),
        };
        return Ok(CommandResult::Finished(false));
    }
}
