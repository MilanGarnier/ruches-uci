use std::{
    fmt::Display,
    io::{Stdout, Write, stdin, stdout},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
    time::Duration,
};

use futures::channel::oneshot::{Sender, channel};
use tokio::task::JoinHandle;

use crate::{eval::MaterialBalance, position::Position, search::Search};

const BUILD_NAME: &str = env!("CARGO_PKG_NAME");
const BUILD_VERSION: &str = env!("CARGO_PKG_VERSION");
const BUILD_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
// const BUILD_ABOUT: &str = "Simple rust chess engine that will get better";
// if accessible, you know the engine is in one of these states

pub struct UciOut<O: Write> {
    _a: PhantomData<O>,
    debug: bool,
}

pub struct UciShell {
    // state will be locked during critical commands
    runtime: Arc<Mutex<tokio::runtime::Runtime>>,
    worker: Arc<Mutex<Option<(tokio::task::JoinHandle<()>, Sender<()>)>>>,
    debug: bool,
    position: Arc<Mutex<Position>>, // TODO add here internal configuration
}

pub trait UciOutputStream: Send {
    fn send_response<T: Display>(r: T) -> Result<(), std::io::Error>;
    fn send_debug<T: Display>(_r: T) -> Result<(), std::io::Error>;
}

//unsafe impl Sync for UciShell {}

impl UciShell {
    pub fn new() -> Self {
        Self {
            runtime: Arc::new(Mutex::new(tokio::runtime::Runtime::new().unwrap())),
            worker: Arc::new(Mutex::new(None)),
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
            "d" => Ok(ParsedCommand::PrintBoard),

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
                Some("infinite") => GoCommand::Infinite,
                _ => todo!(),
            })),

            "stop" => Ok(ParsedCommand::Stop),
            "quit" => Ok(ParsedCommand::Quit),

            _ => Err(()), // return self.failed_parsing_behavior("unsupported command."),
        },
    }
}

impl UciOutputStream for UciOut<std::io::Stdout> {
    fn send_response<T: Display>(r: T) -> Result<(), std::io::Error> {
        let mut out_mut = stdout();
        /*if let UciResponse::Debug(_) = r {
            if self.debug == false {
                return Ok(());
            }
        }*/
        write!(out_mut, "{r}")
    }

    fn send_debug<T: Display>(r: T) -> Result<(), std::io::Error> {
        if cfg!(debug_assertions) {
            Self::send_response(r)?
        }
        Ok(())
    }
}

#[cfg(test)]
pub struct NullUciStream {}
#[cfg(test)]
impl UciOutputStream for NullUciStream {
    fn send_response<T>(_r: T) -> Result<(), std::io::Error> {
        Ok(())
    }
    fn send_debug<T: Display>(_r: T) -> Result<(), std::io::Error> {
        Ok(())
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
    Infinite,
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

pub enum UciResponse<'a> {
    Info(&'a str),
    Raw(&'a str),
    Debug(&'a str), //TODO : add more and never use println elsewhere
    Id(&'a str, String),
    Ok,
    Ready,
    Option { name: &'a str, o: UciOption },
}

pub enum UciParam {
    Info,
    String(String),
    Depth(usize),
    ScoreCp(i32),
    TimeMs(usize),
    Variant(Vec<String>),
}
impl Display for UciParam {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::String(s) => write!(f, "string {s}"),
            Self::Depth(d) => write!(f, "depth {d}"),
            Self::ScoreCp(s) => write!(f, "score cp {s}"),
            Self::TimeMs(t) => write!(f, "time {t}"),
            Self::Variant(v) => {
                write!(f, "pv")?;
                for m in v.into_iter().rev() {
                    write!(f, " {m}")?
                }
                Ok(())
            }
        }
    }
}

impl<'a> Display for UciResponse<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UciResponse::Debug(x) => {
                writeln!(f, "info string \"{}\"", x)
            }
            UciResponse::Info(x) => writeln!(f, "info {}", x),
            UciResponse::Raw(x) => writeln!(f, "{x}"),
            UciResponse::Id(x, y) => writeln!(f, "id {x} {y}"),
            UciResponse::Ok => writeln!(f, "uciok"),
            UciResponse::Ready => writeln!(f, "uciready"),
            UciResponse::Option { name, o } => writeln!(f, "option name {name} {o}"),
        }
    }
}

pub enum CommandResult {
    Finished(bool),
    Pending(tokio::task::JoinHandle<bool>),
}

impl UciShell {
    fn try_register(&self, j: JoinHandle<()>, sendstop: Sender<()>) -> Result<(), ()> {
        let mut lock = match self.worker.lock() {
            Ok(x) => x,
            Err(_) => todo!("Failed unlocking"),
        };
        let channel = lock.deref_mut();
        match channel {
            Some(_) => todo!("Cannot register"),
            None => (),
        };
        *channel = Some((j, sendstop));
        Ok(())
    }

    // blocking until quit is recieved
    pub async fn run<Out: UciOutputStream + 'static>(&'static self) {
        loop {
            let mut line = String::new();
            stdin().read_line(&mut line).unwrap();
            let command = parse(line).unwrap();
            // .await.expect("Can't read line").unwrap();

            let res = self.runcommand::<Out>(command).await;

            match res.unwrap() {
                CommandResult::Finished(true) => return,
                CommandResult::Finished(false) => (),
                CommandResult::Pending(h) => {}
            }
        }
    }

    // returns response
    pub async fn runcommand<Out: UciOutputStream + 'static>(
        &'static self,
        c: ParsedCommand,
    ) -> Result<CommandResult, Box<dyn std::error::Error>> {
        match c {
            _ => (),
        }
        match c {
            ParsedCommand::Quit => return Ok(CommandResult::Finished(true)),
            ParsedCommand::Stop => {
                let mut lock = match self.worker.lock() {
                    Ok(x) => x,
                    Err(_) => todo!("Failed unlocking"),
                };
                let channel = lock.deref_mut();
                let channel = std::mem::replace(channel, None);
                match channel {
                    Some((x, sendstop)) => {
                        sendstop.send(()).unwrap();
                        tokio::select! {
                            _ = tokio::time::sleep(Duration::from_millis(1000)) => {
                                Out::send_debug("Timeout reached, kill previous command").unwrap();
                            },
                            _ = async { loop { if x.is_finished() {break;} else {tokio::time::sleep(Duration::from_millis(10)).await} } } => {
                                Out::send_debug("Command ended peacefully").unwrap();
                            },
                        }
                        if x.is_finished() == false {
                            x.abort();
                            return Ok(CommandResult::Finished(false));
                        }
                    }
                    None => {
                        Out::send_debug("No command to quit.").unwrap();
                        ()
                    }
                }
            }
            ParsedCommand::Uci => {
                Out::send_response(UciResponse::Id(
                    "name",
                    format!("{} {}", BUILD_NAME, BUILD_VERSION),
                ))?;
                Out::send_response(UciResponse::Id("authors", format!("{}", BUILD_AUTHORS)))?;
                // TODO: self.send_response(UciResponse:: &format!("option name UCI_EngineAbout {}", BUILD_ABOUT));
                Out::send_response(UciResponse::Option {
                    name: "Threads",
                    o: UciOption::Spin {
                        default: 1,
                        min: 1,
                        max: 1024,
                    },
                })?;

                Out::send_response(UciResponse::Ok)?;
            }

            ParsedCommand::IsReady => {
                // wait for running commands
                Out::send_response(UciResponse::Ready)?;
            }

            ParsedCommand::PrintBoard => {
                //self.position.lock().unwrap().pretty_print(&h);
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
                    let c = self.position.lock().unwrap().perft_top::<Out>(i);
                    Out::send_response(UciResponse::Raw(""))?;
                    Out::send_response(UciResponse::Raw(
                        format!("Nodes searched : {}", c).as_str(),
                    ))?;
                    Out::send_response(UciResponse::Raw(""))?;
                }
                GoCommand::Infinite => {
                    let (sendstop, sigstop) = channel();
                    let p = self.position.lock().unwrap().clone();
                    let lock = self.runtime.lock().unwrap();
                    let runtime = lock.deref();
                    let t = runtime.spawn(crate::search::SearchDefault::infinite::<
                        MaterialBalance,
                        Out,
                    >(sigstop, p));
                    self.try_register(t, sendstop).unwrap();
                }
            },
            _ => panic!("Should not be able to be here"),
        };
        return Ok(CommandResult::Finished(false));
    }
}
