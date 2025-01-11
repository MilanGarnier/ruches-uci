pub struct MiniMaxMVP {}
impl Search for MiniMaxMVP {
    async fn infinite<T: BasicEvaluation>(
        mut sigstop: futures::channel::oneshot::Receiver<()>,
        pos: Position,
    ) -> Result<(), UciError> {
        let mut depth: usize = 1;
        let mut e = EvalState::new(Eval::Approx(ApproxEval::EQUAL));
        loop {
            e = tokio::select! {
                _ = &mut sigstop => {
                    break;
                }
                x = async move { let a = eval_minimax::<T>(&pos, depth); tokio::time::sleep(Duration::from_millis(0)).await; a} => {
                    x.unwrap()
                }
            };
            log!(
                log::Level::Info,
                "{}",
                UciResponse::Info(format!("{e}").as_str())
            );
            depth += 1;
        }
        log!(log::Level::Debug, "Received sigstop");
        log!(log::Level::Trace, "Current best eval {e}");

        Ok(())
    }
}

use std::time::Duration;

use crate::uci::{UciError, UciResponse};
use crate::{
    eval::{ApproxEval, BasicEvaluation, Eval, EvalState},
    position::{Position, movegen::AugmentedPos},
};

use super::Search;

pub fn eval_minimax<'a, T: BasicEvaluation>(
    pos: &'a Position,
    depth: usize,
) -> Result<EvalState, ()> {
    //#[cfg(debug_assertions)]
    //pos.assert_squares_occupied_only_once();
    match depth {
        // TODO: add quiescent search for depth 1
        0 => {
            let a = AugmentedPos::list_issues(&pos);
            match a {
                Err(()) => Err(()),
                Ok(_) => Ok(EvalState::new(T::eval(pos))),
            }
        }
        _ => {
            let turn = pos.turn();
            //let is_check = !meta.is_check();

            let mut best_eval = EvalState::new(Eval::m0(turn.other()));
            let mut best_move = None;
            let mut explored = 0;

            let evals = pos
                .map_outcomes_slow_move(|p, m| (eval_minimax::<T>(p, depth - 1), format!("{m}")))
                .unwrap();

            for eval in evals {
                match eval {
                    (Err(()), _) => continue,
                    (Ok(eval), m) => {
                        if explored == 0 || (EvalState::pick_best_for(turn, &best_eval, &eval)) == 1
                        {
                            best_eval = eval;
                            best_move = Some(m);
                        }
                        explored += 1;
                    }
                }
            }
            // if there were no legal moves and no check, set to draw instead of M0
            if explored == 0
            /*&& is_check*/
            {
                best_eval = EvalState::new(Eval::draw());
                Ok(best_eval)
            } else if explored != 0 {
                let m = best_move.unwrap();
                best_eval.nest(m.as_str());
                Ok(best_eval)
            } else {
                Ok(best_eval)
            }
        }
    }
}
