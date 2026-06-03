//! Database seeds.
//!
//! - [`demo`] — **optional** filler data (sample org/admin/teams, players, a
//!   question bank, a finished match, notifications) for local/demo use. Gated
//!   behind the `DEMO_SEEDS` flag.

mod demo;

pub use demo::{run_demo_seeds, DemoSeedConfig};
