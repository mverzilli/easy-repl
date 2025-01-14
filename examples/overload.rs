use anyhow::{self, Context};
use mini_async_repl::{
    command::{
        lift_validation_err, validate, ArgsError, Command, CommandArgInfo, CommandArgType,
        ExecuteCommand,
    },
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
}
impl ExecuteCommand for DescribeCommandHandler {
    fn execute(
        &mut self,
        args: Vec<String>,
        args_info: Vec<CommandArgInfo>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>> {
        let valid = validate(args.clone(), args_info.clone());
        if let Err(e) = valid {
            return Box::pin(lift_validation_err(Err(e)));
        }

        // Note: this example could also be implemented by
        // providing one CommandHandler for each overload.
        // For now I think it's better not to constraint approaches
        // because it's not yet clear to me what the best design is.
        let variant_1 = validate(args.clone(), args_info);
        if let Ok(()) = variant_1 {
            return Box::pin(self.handle_variant_1());
        }

        let variant_2 = validate(
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

        let variant_3 = validate(
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

        Box::pin(lift_validation_err(Err(ArgsError::NoVariantFound)))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[rustfmt::skip]
    let mut repl = Repl::builder()
        .add("describe", Command::new(
            "Variant 1",
            vec![],
            Box::new(DescribeCommandHandler::new()),
        ))
        .add("describe", Command::new(
        	"Variant 2",
        	vec![
        		CommandArgInfo::new_with_name(CommandArgType::I32, "a"),
        		CommandArgInfo::new_with_name(CommandArgType::I32, "b"),
        	],
        	Box::new(DescribeCommandHandler::new()),
        ))           
        .add("describe", Command::new(
            "Variant 3",
            vec![
        		CommandArgInfo::new_with_name(CommandArgType::I32, "a"),
        		CommandArgInfo::new_with_name(CommandArgType::String, "b"),
        	],
        	Box::new(DescribeCommandHandler::new()),
        ))
        .build()
        .context("Failed to create repl")?;

    repl.run().await.context("Critical REPL error")?;

    Ok(())
}
