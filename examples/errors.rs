use std::time::Instant;

use anyhow::{self, Context};
use mini_async_repl::{
    command::{
        lift_validation_err, validate, Command, CommandArgInfo, CommandArgType, Critical,
        ExecuteCommand,
    },
    CommandStatus, Repl,
};
use std::future::Future;
use std::pin::Pin;

struct OkCommandHandler {}
impl OkCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
    async fn handle_command(&mut self) -> anyhow::Result<CommandStatus> {
        Ok(CommandStatus::Done)
    }
}
impl ExecuteCommand for OkCommandHandler {
    fn execute(
        &mut self,
        args: Vec<String>,
        args_info: Vec<CommandArgInfo>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>> {
        let valid = validate(args.clone(), args_info.clone());
        if let Err(e) = valid {
            return Box::pin(lift_validation_err(Err(e)));
        }
        Box::pin(self.handle_command())
    }
}

struct RecoverableErrorHandler {}
impl RecoverableErrorHandler {
    pub fn new() -> Self {
        Self {}
    }
    async fn handle_command(&mut self, text: String) -> anyhow::Result<CommandStatus> {
        Self::may_throw(text)?;
        Ok(CommandStatus::Done)
    }
    // this could be any function returning Result with an error implementing Error
    // here for simplicity we make use of the Other variant of std::io::Error
    fn may_throw(description: String) -> Result<(), std::io::Error> {
        Err(std::io::Error::new(std::io::ErrorKind::Other, description))
    }
}
impl ExecuteCommand for RecoverableErrorHandler {
    fn execute(
        &mut self,
        args: Vec<String>,
        args_info: Vec<CommandArgInfo>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>> {
        let valid = validate(args.clone(), args_info.clone());
        if let Err(e) = valid {
            return Box::pin(lift_validation_err(Err(e)));
        }
        Box::pin(self.handle_command(args[0].clone()))
    }
}

struct CriticalErrorHandler {}
impl CriticalErrorHandler {
    pub fn new() -> Self {
        Self {}
    }
    async fn handle_command(&mut self, text: String) -> anyhow::Result<CommandStatus> {
        // Short notation using the Critical trait
        Self::may_throw(text).into_critical()?;
        Ok(CommandStatus::Done)
    }
    // this could be any function returning Result with an error implementing Error
    // here for simplicity we make use of the Other variant of std::io::Error
    fn may_throw(description: String) -> Result<(), std::io::Error> {
        Err(std::io::Error::new(std::io::ErrorKind::Other, description))
    }
}
impl ExecuteCommand for CriticalErrorHandler {
    fn execute(
        &mut self,
        args: Vec<String>,
        args_info: Vec<CommandArgInfo>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>> {
        let valid = validate(args.clone(), args_info.clone());
        if let Err(e) = valid {
            return Box::pin(lift_validation_err(Err(e)));
        }
        Box::pin(self.handle_command(args[0].clone()))
    }
}

struct RouletteErrorHandler {
    start: Instant,
}
impl RouletteErrorHandler {
    pub fn new(start: Instant) -> Self {
        Self { start }
    }
    async fn handle_command(&mut self) -> anyhow::Result<CommandStatus> {
        let ns = Instant::now().duration_since(self.start).as_nanos();
        let cylinder = ns % 6;
        match cylinder {
            0 => Self::may_throw("Bang!".into()).into_critical()?,
            1..=2 => Self::may_throw("Blank cartridge?".into())?,
            _ => (),
        }
        Ok(CommandStatus::Done)
    }
    // this could be any function returning Result with an error implementing Error
    // here for simplicity we make use of the Other variant of std::io::Error
    fn may_throw(description: String) -> Result<(), std::io::Error> {
        Err(std::io::Error::new(std::io::ErrorKind::Other, description))
    }
}
impl ExecuteCommand for RouletteErrorHandler {
    fn execute(
        &mut self,
        args: Vec<String>,
        args_info: Vec<CommandArgInfo>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>> {
        let valid = validate(args.clone(), args_info.clone());
        if let Err(e) = valid {
            return Box::pin(lift_validation_err(Err(e)));
        }
        Box::pin(self.handle_command())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[rustfmt::skip]
    let mut repl = Repl::builder()
        .add("ok", Command::new(
            "Run a command that just succeeds",
            vec![],
            Box::new(OkCommandHandler::new()),
        ))
        .add("error", Command::new(
            "Command with recoverable error handled by the REPL",
            vec![CommandArgInfo::new_with_name(CommandArgType::String, "text")],
            Box::new(RecoverableErrorHandler::new()),
        ))
        .add("critical", Command::new(
            "Command returns a critical error that must be handled outside of REPL",
            vec![CommandArgInfo::new_with_name(CommandArgType::String, "text")],
            Box::new(CriticalErrorHandler::new()),
        ))
        .add("roulette", Command::new(
            "Feeling lucky?",
            vec![],
            Box::new(RouletteErrorHandler::new(Instant::now())),
        ))
        .build()
        .context("Failed to create repl")?;

    let repl_res = repl.run().await;
    match repl_res {
        Ok(_) => Ok(()),
        Err(_) => {
            println!("Repl halted. Quitting.");
            Ok(())
        }
    }
}
