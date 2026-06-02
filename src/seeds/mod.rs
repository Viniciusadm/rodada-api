//! Database seeds, split into two categories:
//!
//! - [`catalog`] — **required** global data (games + game modes). Idempotent and
//!   safe to run on every startup, including production. The product cannot
//!   create matches without it.
//! - [`demo`] — **optional** filler data (sample org/admin/teams, players, a
//!   question bank, a finished match, notifications) for local/demo use. Gated
//!   behind the `DEMO_SEEDS` flag.
//!
//! The catalog must be seeded before the demo data, since the demo match
//! references catalog game/mode IDs.

mod catalog;
mod demo;

pub use catalog::run_catalog_seeds;
pub use demo::{run_demo_seeds, DemoSeedConfig};
