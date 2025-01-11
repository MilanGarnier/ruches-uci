
use futures::channel;

use crate::{eval::BasicEvaluation, position::Position, uci::UciError};

mod basic_minimax;

pub trait Search {
    fn infinite<T: BasicEvaluation>(
        sigstop: channel::oneshot::Receiver<()>,
        pos: Position,
    ) -> impl std::future::Future<Output = Result<(), UciError>> + Send;
    // TODO: add other
}

pub type SearchDefault = basic_minimax::MiniMaxMVP;
