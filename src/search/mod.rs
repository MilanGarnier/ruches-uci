use futures::channel;

use crate::{eval::BasicEvaluation, position::Position, uci::UciOutputStream};

mod basic_minimax;

pub trait Search {
    fn infinite<T: BasicEvaluation, Out : UciOutputStream>(
        sigstop: channel::oneshot::Receiver<()>,
        pos: Position,
    ) -> impl std::future::Future<Output = ()> + Send;
    // TODO: add other
}

pub type SearchDefault = basic_minimax::MiniMaxMVP;
