//! Database seeds.
//!
//! - [`demo`] — **optional** filler data (sample org/admin/teams, players, a
//!   question bank, a finished match, notifications) for local/demo use. Gated
//!   behind the `DEMO_SEEDS` flag.
//!
//! The game catalog is no longer seeded: it lives in code
//! (`domain::games::catalog`). The demo match references catalog game/mode IDs
//! directly by their stable UUIDs.

mod demo;

pub use demo::{run_demo_seeds, DemoSeedConfig};
