use anyhow::{self, Context};
use mini_async_repl::{
    command::{
        lift_validation_err, validate, Command, CommandArgInfo, CommandArgType, ExecuteCommand,
    },
    CommandStatus, Repl,
};
use std::future::Future;
use std::pin::Pin;

struct SayHelloCommandHandler {}
impl SayHelloCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
    async fn handle_command(&mut self, name: String) -> anyhow::Result<CommandStatus> {
        println!("Hello {}!", name);
        Ok(CommandStatus::Done)
    }
}
impl ExecuteCommand for SayHelloCommandHandler {
    fn execute(
        &mut self,
        args: Vec<String>,
        args_info: Vec<CommandArgInfo>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>> {
        let valid = validate(args.clone(), args_info.clone());
        if valid.is_err() {
            return Box::pin(lift_validation_err(valid));
        }
        Box::pin(self.handle_command(args[0].clone()))
    }
}

struct AddCommandHandler {}
impl AddCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
    async fn handle_command(&mut self, x: i32, y: i32) -> anyhow::Result<CommandStatus> {
        println!("{} + {} = {}", x, y, x + y);
        Ok(CommandStatus::Done)
    }
}
impl ExecuteCommand for AddCommandHandler {
    fn execute(
        &mut self,
        args: Vec<String>,
        args_info: Vec<CommandArgInfo>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>> {
        let valid = validate(args.clone(), args_info.clone());
        if valid.is_err() {
            return Box::pin(lift_validation_err(valid));
        }

        let x = args[0].parse::<i32>();
        let y = args[1].parse::<i32>();

        match (x, y) {
            (Ok(x), Ok(y)) => Box::pin(self.handle_command(x, y)),
            _ => panic!("Unreachable, validator should have covered this"),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let hello_cmd = Command::new(
        "Say hello",
        vec![CommandArgInfo::new_with_name(
            CommandArgType::String,
            "name",
        )],
        Box::new(SayHelloCommandHandler::new()),
    );

    let add_cmd = Command::new(
        "Add X to Y",
        vec![
            CommandArgInfo::new_with_name(CommandArgType::I32, "X"),
            CommandArgInfo::new_with_name(CommandArgType::I32, "Y"),
        ],
        Box::new(AddCommandHandler::new()),
    );

    #[rustfmt::skip]
    let mut repl = Repl::builder()
        .add("hello", hello_cmd)
        .add("add", add_cmd)
        .build()
        .context("Failed to create repl")?;

    repl.run().await.context("Critical REPL error")?;

    Ok(())
}
