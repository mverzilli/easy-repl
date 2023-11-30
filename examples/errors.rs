use std::time::Instant;

use anyhow::{self, Context};
use easy_repl::{
    command::{
        ExecuteCommand,
        NewCommand,
        CommandArgInfo,
        CommandArgType,
        Validator,
        ArgsError,
        Critical,
    },
    CommandStatus,
    Repl,
};
use std::pin::Pin;
use std::future::Future;

struct OkCommandHandler {}
impl OkCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
    async fn handle_command(&mut self) -> anyhow::Result<CommandStatus> {        
        Ok(CommandStatus::Done)
    }
    async fn resolved(result: Result<(), ArgsError>) -> Result<CommandStatus, anyhow::Error> {
        match result {
            Ok(_) => Ok(CommandStatus::Done),
            Err(e) => Err(e.into()),
        }
    }
}
impl ExecuteCommand for OkCommandHandler {
    fn execute(&mut self, args: Vec<String>) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>> {
        let valid = Validator::validate(args.clone(), vec![
            CommandArgInfo::new_with_name(CommandArgType::String, "name"),
        ]);
        if let Err(e) = valid {
            return Box::pin(OkCommandHandler::resolved(Err(e)));
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
    async fn resolved(result: Result<(), ArgsError>) -> Result<CommandStatus, anyhow::Error> {
        match result {
            Ok(_) => Ok(CommandStatus::Done),
            Err(e) => Err(e.into()),
        }
    }
    // this could be any function returning Result with an error implementing Error
    // here for simplicity we make use of the Other variant of std::io::Error
    fn may_throw(description: String) -> Result<(), std::io::Error> {
        Err(std::io::Error::new(std::io::ErrorKind::Other, description))
    }
}
impl ExecuteCommand for RecoverableErrorHandler {
    fn execute(&mut self, args: Vec<String>) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>> {
        let valid = Validator::validate(args.clone(), vec![CommandArgInfo::new_with_name(CommandArgType::String, "text")]);
        if let Err(e) = valid {
            return Box::pin(RecoverableErrorHandler::resolved(Err(e)));
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
        // More explicitly it could be:
        //   if let Err(err) = may_throw(text) {
        //       Err(easy_repl::CriticalError::Critical(err.into()))?;
        //   }
        // or even:
        //   if let Err(err) = may_throw(text) {
        //       return Err(easy_repl::CriticalError::Critical(err.into())).into();
        //   }
        Ok(CommandStatus::Done)
    }
    async fn resolved(result: Result<(), ArgsError>) -> Result<CommandStatus, anyhow::Error> {
        match result {
            Ok(_) => Ok(CommandStatus::Done),
            Err(e) => Err(e.into()),
        }
    }
    // this could be any function returning Result with an error implementing Error
    // here for simplicity we make use of the Other variant of std::io::Error
    fn may_throw(description: String) -> Result<(), std::io::Error> {
        Err(std::io::Error::new(std::io::ErrorKind::Other, description))
    }
}
impl ExecuteCommand for CriticalErrorHandler {
    fn execute(&mut self, args: Vec<String>) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>> {
        let valid = Validator::validate(args.clone(), vec![CommandArgInfo::new_with_name(CommandArgType::String, "text")]);
        if let Err(e) = valid {
            return Box::pin(CriticalErrorHandler::resolved(Err(e)));
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
    async fn resolved(result: Result<(), ArgsError>) -> Result<CommandStatus, anyhow::Error> {
        match result {
            Ok(_) => Ok(CommandStatus::Done),
            Err(e) => Err(e.into()),
        }
    }
    // this could be any function returning Result with an error implementing Error
    // here for simplicity we make use of the Other variant of std::io::Error
    fn may_throw(description: String) -> Result<(), std::io::Error> {
        Err(std::io::Error::new(std::io::ErrorKind::Other, description))
    }
}
impl ExecuteCommand for RouletteErrorHandler {
    fn execute(&mut self, args: Vec<String>) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>> {
        let valid = Validator::validate(args.clone(), vec![]);
        if let Err(e) = valid {
            return Box::pin(RouletteErrorHandler::resolved(Err(e)));
        }
        Box::pin(self.handle_command())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let start = Instant::now();

    #[rustfmt::skip]
    let mut repl = Repl::builder()
        .add("ok", NewCommand {
            description: "Run a command that just succeeds".into(),
            args_info: vec![],
            handler: Box::new(OkCommandHandler::new()),
        })
        .add("error", NewCommand {
            description: "Command with recoverable error handled by the REPL".into(),
            args_info: vec![CommandArgInfo::new_with_name(CommandArgType::String, "text")],
            handler: Box::new(RecoverableErrorHandler::new()),
        })
        .add("critical", NewCommand {
            description: "Command returns a critical error that must be handled outside of REPL".into(),
            args_info: vec![CommandArgInfo::new_with_name(CommandArgType::String, "text")],
            handler: Box::new(CriticalErrorHandler::new()),
        })
        .add("roulette", NewCommand {
            description: "Feeling lucky?".into(),
            args_info: vec![],
            handler: Box::new(RouletteErrorHandler::new(Instant::now())),
        })
        .build()
        .context("Failed to create repl")?;

    let repl_res = repl.run().await;
    match repl_res {
        Ok(_) => Ok(()),
        Err(e) => {
            println!("Repl halted. Quitting.");
            Ok(())
        }
    }
}