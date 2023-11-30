use std::net::IpAddr;
use std::path::PathBuf;

use anyhow::{self, Context};
use mini_async_repl::{
    command::{
        resolved_command, ArgsError, Command, CommandArgInfo, CommandArgType, ExecuteCommand,
        Validator,
    },
    CommandStatus, Repl,
};
use std::future::Future;
use std::pin::Pin;

struct LsCommandHandler {}
impl LsCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
    async fn handle_command(&mut self, dir: PathBuf) -> anyhow::Result<CommandStatus> {
        for entry in dir.read_dir()? {
            println!("{}", entry?.path().to_string_lossy());
        }
        Ok(CommandStatus::Done)
    }
}
impl ExecuteCommand for LsCommandHandler {
    fn execute(
        &mut self,
        args: Vec<String>,
        args_info: Vec<CommandArgInfo>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>> {
        let valid = Validator::validate(args.clone(), args_info.clone());
        if let Err(e) = valid {
            return Box::pin(resolved_command(Err(e)));
        }

        let dir_buf: PathBuf = args[0].clone().into();
        Box::pin(self.handle_command(dir_buf))
    }
}

struct IpAddrCommandHandler {}
impl IpAddrCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
    async fn handle_command(&mut self, ip: IpAddr) -> anyhow::Result<CommandStatus> {
        println!("{}", ip);
        Ok(CommandStatus::Done)
    }
}
impl ExecuteCommand for IpAddrCommandHandler {
    fn execute(
        &mut self,
        args: Vec<String>,
        args_info: Vec<CommandArgInfo>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>> {
        let valid = Validator::validate(args.clone(), args_info.clone());
        if let Err(e) = valid {
            return Box::pin(resolved_command(Err(e)));
        }

        let ip = args[0].parse();

        match ip {
            Ok(ip) => Box::pin(self.handle_command(ip)),
            Err(e) => Box::pin(resolved_command(Err(ArgsError::WrongArgumentValue {
                argument: args[0].clone(),
                error: e.to_string(),
            }))),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[rustfmt::skip]
    let mut repl = Repl::builder()
        .add("ls", Command::new(
            "List files in a directory",
            vec![CommandArgInfo::new_with_name(CommandArgType::Custom, "dir")],
            Box::new(LsCommandHandler::new()),
        ))
        .add("ipaddr", Command::new(
            "Just parse and print the given IP address".into(),
            vec![CommandArgInfo::new_with_name(CommandArgType::Custom, "ip")],
            Box::new(IpAddrCommandHandler::new()),
        ))
        .build()
        .context("Failed to create repl")?;

    repl.run().await.context("Critical REPL error")
}
