pub struct MiniMaxMVP {}
impl Search for MiniMaxMVP {
    async fn infinite<T: BasicEvaluation, Out: UciOutputStream>(
        mut sigstop: futures::channel::oneshot::Receiver<()>,
        pos: Position,
    ) {
        let mut depth: usize = 1;
        let mut e = EvalState::new(Eval::Approx(ApproxEval::EQUAL));
        loop {
            e = tokio::select! {
                _ = &mut sigstop => {
                    break;
                }
                x = async move { let a = eval_minimax::<T>(&mut pos.clone(), depth); tokio::time::sleep(Duration::from_millis(0)).await; a} => { x

                }
            };
            Out::send_response(UciResponse::Info(format!("{e}").as_str())).unwrap();
            depth += 1;
        }
        Out::send_debug(crate::uci::UciResponse::Debug("Received stop signal")).unwrap();
        Out::send_response(crate::uci::UciResponse::Info(format!("{e}").as_str())).unwrap();
    }
}

use std::time::Duration;

use log::warn;

use crate::{
    AugmentedPos, PositionSpec,
    eval::{ApproxEval, BasicEvaluation, Eval, EvalState},
    movegen::SimplifiedMove,
    position::Position,
    uci::{UciOutputStream, UciResponse},
};

use super::Search;

pub fn eval_minimax<T: BasicEvaluation>(pos: &Position, depth: usize) -> EvalState {
    //#[cfg(debug_assertions)]
    //pos.assert_squares_occupied_only_once();
    match depth {
        // TODO: add quiescent search for depth 1
        0 => EvalState::new(T::eval(pos)),
        _ => {
            let turn = pos.turn();

            let e = AugmentedPos::map_issues(
                pos,
                |p, _x| {
                    let mut a = eval_minimax::<T>(p, depth - 1);
                    a.nest(*_x);
                    a
                },
                |e0, e1| EvalState::pick_best_for(pos.turn(), e0, e1),
            );

            let e = match e {
                Some(x) => x,
                None => {
                    warn!("This could also be a draw, TODO");
                    EvalState::new(Eval::m0(turn.other()))
                }
            };
            e
        }
    }
}
