use anyhow::{self, Context};
use mini_async_repl::{
    command::{ArgsError, Command, CommandArgInfo, CommandArgType, ExecuteCommand, Validator},
    CommandStatus, Repl,
};
use std::future::Future;
use std::pin::Pin;

struct DescribeCommandHandler {}
impl DescribeCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
    async fn handle_variant_1(&mut self) -> anyhow::Result<CommandStatus> {
        println!("No arguments");
        Ok(CommandStatus::Done)
    }
    async fn handle_variant_2(&mut self, a: i32, b: i32) -> anyhow::Result<CommandStatus> {
        println!("Got two integers: {} {}", a, b);
        Ok(CommandStatus::Done)
    }
    async fn handle_variant_3(&mut self, a: i32, b: String) -> anyhow::Result<CommandStatus> {
        println!("An integer `{}` and a string `{}`", a, b);
        Ok(CommandStatus::Done)
    }
    async fn resolved(result: Result<(), ArgsError>) -> Result<CommandStatus, anyhow::Error> {
        match result {
            Ok(_) => Ok(CommandStatus::Done),
            Err(e) => Err(e.into()),
        }
    }
}
impl ExecuteCommand for DescribeCommandHandler {
    fn execute(
        &mut self,
        args: Vec<String>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>> {
        let variant_1 = Validator::validate(args.clone(), vec![]);
        if let Ok(()) = variant_1 {
            return Box::pin(self.handle_variant_1());
        }

        let variant_2 = Validator::validate(
            args.clone(),
            vec![
                CommandArgInfo::new_with_name(CommandArgType::I32, "a"),
                CommandArgInfo::new_with_name(CommandArgType::I32, "b"),
            ],
        );
        if let Ok(()) = variant_2 {
            let a = args[0].parse::<i32>();
            let b = args[1].parse::<i32>();

            match (a, b) {
                (Ok(a), Ok(b)) => {
                    return Box::pin(self.handle_variant_2(a, b));
                }
                _ => (),
            }
        }

        let variant_3 = Validator::validate(
            args.clone(),
            vec![
                CommandArgInfo::new_with_name(CommandArgType::I32, "a"),
                CommandArgInfo::new_with_name(CommandArgType::String, "b"),
            ],
        );
        if let Ok(()) = variant_3 {
            let a = args[0].parse::<i32>();
            let b = args[1].clone();

            match a {
                Ok(a) => {
                    return Box::pin(self.handle_variant_3(a, b));
                }
                _ => (),
            }
        }

        Box::pin(DescribeCommandHandler::resolved(Err(
            ArgsError::NoVariantFound,
        )))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[rustfmt::skip]
    let mut repl = Repl::builder()
        .add("describe", Command {
            description: "Variant 1".into(),
            args_info: vec![],
            handler: Box::new(DescribeCommandHandler::new()),
        })
        .add("describe", Command {
        	description: "Variant 2".into(),
        	args_info: vec![
        		CommandArgInfo::new_with_name(CommandArgType::I32, "a"),
        		CommandArgInfo::new_with_name(CommandArgType::I32, "b"),
        	],
        	handler: Box::new(DescribeCommandHandler::new()),
        })           
        .add("describe", Command {
            description: "Variant 3".into(),
            args_info: vec![
        		CommandArgInfo::new_with_name(CommandArgType::I32, "a"),
        		CommandArgInfo::new_with_name(CommandArgType::String, "b"),
        	],
        	handler: Box::new(DescribeCommandHandler::new()),
        })
        .build()
        .context("Failed to create repl")?;

    repl.run().await.context("Critical REPL error")?;

    Ok(())
}
