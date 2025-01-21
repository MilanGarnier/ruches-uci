use super::position::AugmentedPos;
use super::position::Position;
use super::prelude::*;

impl Position {
    #[cfg(feature = "perft")]
    pub fn perft_top<O: UciOutputStream>(&mut self, depth: usize) -> usize {
        use crate::uci::UciResponse;

        match depth {
            0 => 1,
            _ => {
                let sum = AugmentedPos::map_issues(
                    self,
                    |pos, mbv| {
                        let partial_sum = Self::perft_rec(pos, depth - 1, 0);
                        O::send_response(UciResponse::Raw(
                            format!("{mbv}: {}", partial_sum).as_str(),
                        ))
                        .unwrap();
                        partial_sum
                    },
                    |a, b| a + b,
                );

                match sum {
                    Some(x) => x,
                    None => 0,
                }
            }
        }
    }

    fn perft_rec(&self, depth: usize, depth_in: usize) -> usize {
        match depth {
            0 => 1,
            1 => {
                let a = AugmentedPos::map_issues(self, |_, _| 1 as usize, |a, b| a + b);
                match a {
                    Some(x) => x,
                    None => 0,
                }
            }
            _ => {
                let sum = AugmentedPos::map_issues(
                    self,
                    |pos, _| Self::perft_rec(pos, depth - 1, depth_in + 1),
                    |a, b| a + b,
                );

                match sum {
                    Some(x) => x,
                    None => 0,
                }
            }
        }
    }
}
