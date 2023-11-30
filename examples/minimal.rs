use anyhow::{self, Context};
use easy_repl::{
    command::{
        ExecuteCommand,
        NewCommand,
        CommandArgInfo,
        CommandArgType,
        Validator,
        ArgsError,
    },
    CommandStatus,
    Repl,
};
use std::pin::Pin;
use std::future::Future;

struct SayHelloCommandHandler {}
impl SayHelloCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
    async fn handle_command(&mut self, name: String) -> anyhow::Result<CommandStatus> {        
        println!("Hello {}!", name);
        Ok(CommandStatus::Done)
    }
    async fn resolved(result: Result<(), ArgsError>) -> Result<CommandStatus, anyhow::Error> {
        match result {
            Ok(_) => Ok(CommandStatus::Done),
            Err(e) => Err(e.into()),
        }
    }
}
impl ExecuteCommand for SayHelloCommandHandler {
    fn execute(&mut self, args: Vec<String>) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>> {
        let valid = Validator::validate(args.clone(), vec![
            CommandArgInfo::new_with_name(CommandArgType::String, "name"),
        ]);
        if let Err(e) = valid {
            return Box::pin(AddCommandHandler::resolved(Err(e)));
        }
        Box::pin(self.handle_command(args[0].clone()))
    }
}

struct AddCommandHandler {}
impl AddCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
    async fn handle_command(&mut self, x: i32, y:i32) -> anyhow::Result<CommandStatus> {        
        println!("{} + {} = {}", x, y, x + y);
        Ok(CommandStatus::Done) 
    }
    async fn resolved(result: Result<(), ArgsError>) -> Result<CommandStatus, anyhow::Error> {
        match result {
            Ok(_) => Ok(CommandStatus::Done),
            Err(e) => Err(e.into()),
        }
    }
}
impl ExecuteCommand for AddCommandHandler {
    fn execute(&mut self, args: Vec<String>) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>> {
        // TODO: validator
        let valid = Validator::validate(args.clone(), vec![
            CommandArgInfo::new_with_name(CommandArgType::I32, "X"),
            CommandArgInfo::new_with_name(CommandArgType::I32, "Y"),
        ]);
        if let Err(e) = valid {
            return Box::pin(AddCommandHandler::resolved(Err(e)));
        }

        let x = args[0].parse::<i32>();
        let y = args[1].parse::<i32>();

        match (x, y) {
            (Ok(x), Ok(y)) => Box::pin(self.handle_command(x, y)),
            _ => panic!("Unreachable, validator should have covered this")
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let hello_cmd = NewCommand {
        description: "Say hello".into(),
        args_info: vec![CommandArgInfo::new_with_name(CommandArgType::String, "name")],
        handler: Box::new(SayHelloCommandHandler::new()),
    };

    let add_cmd = NewCommand {
        description: "Add X to Y".into(),
        args_info: vec![
            CommandArgInfo::new_with_name(CommandArgType::I32, "X"),
            CommandArgInfo::new_with_name(CommandArgType::I32, "Y"),
        ],
        handler: Box::new(AddCommandHandler::new()),
    };

    #[rustfmt::skip]
    let mut repl = Repl::builder()
        .add("hello", hello_cmd)
        .add("add",  add_cmd)
        .build()
        .context("Failed to create repl")?;

    repl.run().await.context("Critical REPL error")?;

    Ok(())
}
