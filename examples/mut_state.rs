use anyhow::{self, Context};
use easy_repl::{
    command::{ArgsError, CommandArgInfo, CommandArgType, ExecuteCommand, NewCommand, Validator},
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
    async fn resolved(result: Result<(), ArgsError>) -> Result<CommandStatus, anyhow::Error> {
        match result {
            Ok(_) => Ok(CommandStatus::Done),
            Err(e) => Err(e.into()),
        }
    }
}
impl ExecuteCommand for CountCommandHandler {
    fn execute(
        &mut self,
        args: Vec<String>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>> {
        // TODO: validator
        let valid = Validator::validate(
            args.clone(),
            vec![
                CommandArgInfo::new_with_name(CommandArgType::I32, "X"),
                CommandArgInfo::new_with_name(CommandArgType::I32, "Y"),
            ],
        );
        if let Err(e) = valid {
            return Box::pin(CountCommandHandler::resolved(Err(e)));
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
    async fn resolved(result: Result<(), ArgsError>) -> Result<CommandStatus, anyhow::Error> {
        match result {
            Ok(_) => Ok(CommandStatus::Done),
            Err(e) => Err(e.into()),
        }
    }
}
impl ExecuteCommand for SayCommandHandler {
    fn execute(
        &mut self,
        args: Vec<String>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>> {
        let valid = Validator::validate(
            args.clone(),
            vec![CommandArgInfo::new_with_name(CommandArgType::F32, "X")],
        );
        if let Err(e) = valid {
            return Box::pin(SayCommandHandler::resolved(Err(e)));
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
    async fn resolved(result: Result<(), ArgsError>) -> Result<CommandStatus, anyhow::Error> {
        match result {
            Ok(_) => Ok(CommandStatus::Done),
            Err(e) => Err(e.into()),
        }
    }
}
impl ExecuteCommand for OutXCommandHandler {
    fn execute(
        &mut self,
        args: Vec<String>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>> {
        let valid = Validator::validate(args.clone(), vec![]);
        if let Err(e) = valid {
            return Box::pin(OutXCommandHandler::resolved(Err(e)));
        }
        Box::pin(self.handle_command())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut outside_x = Rc::new(RefCell::new(String::from("Out x")));
    let mut outside_y = String::from("Out y");

    #[rustfmt::skip]
    let mut repl = Repl::builder()
        .description("Example REPL")
        .prompt("=> ")
        .text_width(60 as usize)
        .add("count", NewCommand {
        	description: "Count from X to Y".into(),
        	args_info: vec![
        		CommandArgInfo::new_with_name(CommandArgType::I32, "X"),
        		CommandArgInfo::new_with_name(CommandArgType::I32, "Y"),
        	],
        	handler: Box::new(CountCommandHandler::new()),
        })
        .add("say", NewCommand {
        	description: "Say X".into(),
        	args_info: vec![CommandArgInfo::new_with_name(CommandArgType::F32, "X")],
        	handler: Box::new(SayCommandHandler::new()),
        })
        .add("outx", NewCommand {
        	description: "Use mutably outside var x. This command has a really long description so we need to wrap it somehow, it is interesting how actually the wrapping will be performed.".into(),
        	args_info: vec![],
        	handler: Box::new(OutXCommandHandler::new(outside_x.clone())),
        })
        // TODO: not a very relevant example now that we're not using macros
        // // this shows how to create Command manually with the help of the validator! macro
        // // one could also implement arguments validation manually
        // .add("outy", easy_repl::Command {
        //     description: "Use mutably outside var y".into(),
        //     args_info: vec!["appended".into()],
        //     handler: Box::new(|args| {
        //         let validator = validator!(i32);
        //         validator(args)?;
        //         outside_y += args[0];
        //         println!("{}", outside_y);
        //         Ok(CommandStatus::Done)
        //     }),
        // })
        .build().context("Failed to create repl")?;

    repl.run().await.context("Critical REPL error")?;

    Ok(())
}
