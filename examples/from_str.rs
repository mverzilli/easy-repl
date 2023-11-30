use std::net::IpAddr;
use std::path::PathBuf;

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
    async fn resolved(result: Result<(), ArgsError>) -> Result<CommandStatus, anyhow::Error> {
        match result {
            Ok(_) => Ok(CommandStatus::Done),
            Err(e) => Err(e.into()),
        }
    }
}
impl ExecuteCommand for LsCommandHandler {
    fn execute(&mut self, args: Vec<String>) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>> {
        let valid = Validator::validate(args.clone(), vec![
            CommandArgInfo::new_with_name(CommandArgType::Custom, "dir"),
        ]);
        if let Err(e) = valid {
            return Box::pin(LsCommandHandler::resolved(Err(e)));
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
    async fn resolved(result: Result<(), ArgsError>) -> Result<CommandStatus, anyhow::Error> {
        match result {
            Ok(_) => Ok(CommandStatus::Done),
            Err(e) => Err(e.into()),
        }
    }
}
impl ExecuteCommand for IpAddrCommandHandler {
    fn execute(&mut self, args: Vec<String>) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>> {
        let valid = Validator::validate(args.clone(), vec![
            CommandArgInfo::new_with_name(CommandArgType::Custom, "ip"),
        ]);
        if let Err(e) = valid {
            return Box::pin(IpAddrCommandHandler::resolved(Err(e)));
        }

        let ip = args[0].parse();

        match ip {
        	Ok(ip) => Box::pin(self.handle_command(ip)),
        	Err(e) => Box::pin(IpAddrCommandHandler::resolved(Err(ArgsError::WrongArgumentValue {
        	        		argument: args[0].clone(),
        	        		error: e.to_string(),
        	    		})))
        }
    }
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[rustfmt::skip]
    let mut repl = Repl::builder()
        .add("ls", NewCommand {
            description: "List files in a directory".into(),
            args_info: vec![CommandArgInfo::new_with_name(CommandArgType::Custom, "dir")],
            handler: Box::new(LsCommandHandler::new()),
        })
        .add("ipaddr", NewCommand {
            description: "Just parse and print the given IP address".into(),
            args_info: vec![CommandArgInfo::new_with_name(CommandArgType::Custom, "ip")],
            handler: Box::new(IpAddrCommandHandler::new()),
        })
        .build()
        .context("Failed to create repl")?;

    repl.run().await.context("Critical REPL error")
}
