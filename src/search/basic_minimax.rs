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
                x = async move { let a = eval_minimax::<T>(&mut pos.clone(), depth); tokio::time::sleep(Duration::from_millis(0)).await; a} => {
                    x.unwrap()
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

use crate::{
    eval::{ApproxEval, BasicEvaluation, Eval, EvalState},
    position::{Position, movegen::AugmentedPos},
    uci::{UciOutputStream, UciResponse},
};

use super::Search;

pub fn eval_minimax<T: BasicEvaluation>(pos: &mut Position, depth: usize) -> Result<EvalState, ()> {
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
            let movelist = AugmentedPos::list_issues(pos)?;
            //let is_check = !meta.is_check();

            let mut best_eval = EvalState::new(Eval::m0(turn.other()));
            let mut best_move = None;
            let mut explored = 0;
            for m in movelist.iter() {
                pos.stack(m);
                let eval = eval_minimax::<T>(pos, depth - 1);
                pos.unstack(m);
                match eval {
                    Err(()) => continue,
                    Ok(eval) => {
                        if (EvalState::pick_best_for(turn, &best_eval, &eval)) == 1 {
                            best_eval = eval;
                            best_move = Some(m);
                        }
                        if explored == 0 {
                            best_move = Some(m)
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
                best_eval.nest(*m);
                Ok(best_eval)
            } else {
                Ok(best_eval)
            }
        }
    }
}
