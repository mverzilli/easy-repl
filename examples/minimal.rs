use easy_repl::{Repl, CommandStatus, command};
use anyhow::{self, Context};

fn main() -> anyhow::Result<()> {
    let mut repl = Repl::builder()
        .add("add", command! {
            "Count from X to Y";
            X:i32, Y:i32 => |x, y| {
                println!("{} + {} = {}", x, y, x + y);
                Ok(CommandStatus::Done)
            }
        })
        .add("mul", command! {
            "Count from X to Y";
            X:i32, Y:i32 => |x, y| {
                println!("{} * {} = {}", x, y, x * y);
                Ok(CommandStatus::Done)
            }
        })
        .build().context("Failed to create repl")?;

    repl.run().context("Critical REPL error")?;

    Ok(())
}

