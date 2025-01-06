mod dyn_attacks;

#[cfg(feature = "static_attacks")]
mod static_attacks;

#[cfg(feature = "static_attacks")]
pub use static_attacks::{generate_bishops, generate_queens, generate_rooks};

#[cfg(not(feature = "static_attacks"))]
pub use dyn_attacks::{generate_bishops, generate_queens, generate_rooks};

pub use dyn_attacks::{generate_king, generate_knights, generate_pawns};
