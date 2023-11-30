use anyhow::{self, Context};
use mini_async_repl::{
    command::{
        resolved_command, validate, Command, CommandArgInfo, CommandArgType, ExecuteCommand,
    },
    CommandStatus, Repl,
};
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

struct CountCommandHandler {}
impl CountCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
    async fn handle_command(&mut self, x: i32, y: i32) -> anyhow::Result<CommandStatus> {
        for i in x..=y {
            print!(" {}", i);
        }
        println!();
        Ok(CommandStatus::Done)
    }
}
impl ExecuteCommand for CountCommandHandler {
    fn execute(
        &mut self,
        args: Vec<String>,
        args_info: Vec<CommandArgInfo>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>> {
        let valid = validate(args.clone(), args_info.clone());
        if let Err(e) = valid {
            return Box::pin(resolved_command(Err(e)));
        }

        let x = args[0].parse::<i32>();
        let y = args[1].parse::<i32>();

        match (x, y) {
            (Ok(x), Ok(y)) => Box::pin(self.handle_command(x, y)),
            _ => panic!("Unreachable, validator should have covered this"),
        }
    }
}

struct SayCommandHandler {}
impl SayCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
    async fn handle_command(&mut self, x: f32) -> anyhow::Result<CommandStatus> {
        println!("x is equal to {}", x);
        Ok(CommandStatus::Done)
    }
}
impl ExecuteCommand for SayCommandHandler {
    fn execute(
        &mut self,
        args: Vec<String>,
        args_info: Vec<CommandArgInfo>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>> {
        let valid = validate(args.clone(), args_info.clone());
        if let Err(e) = valid {
            return Box::pin(resolved_command(Err(e)));
        }

        let x = args[0].parse::<f32>();
        match x {
            Ok(x) => Box::pin(self.handle_command(x)),
            _ => panic!("Unreachable, validator should have covered this"),
        }
    }
}

struct OutXCommandHandler {
    outside_x: Rc<RefCell<String>>,
}
impl OutXCommandHandler {
    pub fn new(outside_x: Rc<RefCell<String>>) -> Self {
        Self { outside_x }
    }
    async fn handle_command(&mut self) -> anyhow::Result<CommandStatus> {
        let mut x = self.outside_x.borrow_mut();
        *x += "x";
        println!("{}", x);
        Ok(CommandStatus::Done)
    }
}
impl ExecuteCommand for OutXCommandHandler {
    fn execute(
        &mut self,
        args: Vec<String>,
        args_info: Vec<CommandArgInfo>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>> {
        let valid = validate(args.clone(), args_info.clone());
        if let Err(e) = valid {
            return Box::pin(resolved_command(Err(e)));
        }
        Box::pin(self.handle_command())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let outside_x = Rc::new(RefCell::new(String::from("Out x")));

    #[rustfmt::skip]
    let mut repl = Repl::builder()
        .description("Example REPL")
        .prompt("=> ")
        .text_width(60 as usize)
        .add("count", Command::new(
        	"Count from X to Y",
        	vec![
        		CommandArgInfo::new_with_name(CommandArgType::I32, "X"),
        		CommandArgInfo::new_with_name(CommandArgType::I32, "Y"),
        	],
        	Box::new(CountCommandHandler::new()),
        ))
        .add("say", Command::new(
        	"Say X",
        	vec![CommandArgInfo::new_with_name(CommandArgType::F32, "X")],
        	Box::new(SayCommandHandler::new()),
        ))
        .add("outx", Command::new(
        	"Use mutably outside var x. This command has a really long description so we need to wrap it somehow, it is interesting how actually the wrapping will be performed.",
        	vec![],
        	Box::new(OutXCommandHandler::new(outside_x.clone())),
        ))
        .build().context("Failed to create repl")?;

    repl.run().await.context("Critical REPL error")?;

    Ok(())
}
