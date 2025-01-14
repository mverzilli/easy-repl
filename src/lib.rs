// #![deny(missing_docs)]

pub mod command;
mod completion;
pub mod repl;

pub use anyhow;

pub use command::{CommandStatus, Critical, CriticalError};
pub use repl::Repl;
