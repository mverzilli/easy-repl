//! Implementation of [`Command`]s with utilities that help to crate them.

use anyhow;
use thiserror;

use std::fmt::Display;
use std::fmt::Formatter;
use std::future::Future;
use std::pin::Pin;

pub trait ExecuteCommand {
    fn execute(
        &mut self,
        args: Vec<String>,
        args_info: Vec<CommandArgInfo>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>>;
}

pub async fn resolved_command(
    result: Result<(), ArgsError>,
) -> Result<CommandStatus, anyhow::Error> {
    match result {
        Ok(_) => Ok(CommandStatus::Done),
        Err(e) => Err(e.into()),
    }
}

pub struct TrivialCommandHandler {}
impl TrivialCommandHandler {
    pub fn new() -> Self {
        Self {}
    }

    async fn handle_command(&mut self, _args: Vec<String>) -> anyhow::Result<CommandStatus> {
        Ok(CommandStatus::Done)
    }
}

impl ExecuteCommand for TrivialCommandHandler {
    fn execute(
        &mut self,
        args: Vec<String>,
        _args_info: Vec<CommandArgInfo>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>> {
        Box::pin(self.handle_command(args))
    }
}

#[derive(Clone)]
pub struct CommandArgInfo {
    pub arg_type: CommandArgType,
    pub name: Option<String>,
}
impl CommandArgInfo {
    pub fn new(arg_type: CommandArgType) -> Self {
        CommandArgInfo {
            arg_type,
            name: None,
        }
    }

    pub fn new_with_name(arg_type: CommandArgType, name: &str) -> Self {
        CommandArgInfo {
            arg_type,
            name: Some(name.into()),
        }
    }

    pub fn to_string(self) -> String {
        format!("{}:{}", self.name.unwrap_or("".to_string()), self.arg_type)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandArgType {
    I32,
    F32,
    String,
    Custom,
}

impl Display for CommandArgType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandArgType::I32 => write!(f, "i32"),
            CommandArgType::F32 => write!(f, "f32"),
            CommandArgType::String => write!(f, "String"),
            CommandArgType::Custom => write!(f, "Custom"),
        }
    }
}

pub struct Command {
    /// Command desctiption that will be displayed in the help message
    pub(crate) description: String,
    /// Names and types of arguments to the command
    pub(crate) args_info: Vec<CommandArgInfo>,
    /// Command handler which should validate arguments and perform command logic
    pub(crate) handler: Box<dyn ExecuteCommand>,
}

impl Command {
    pub fn new(
        desc: &str,
        args_info: Vec<CommandArgInfo>,
        handler: Box<dyn ExecuteCommand>,
    ) -> Self {
        Self {
            description: desc.into(),
            args_info,
            handler,
        }
    }

    pub fn execute(
        &mut self,
        args: &[&str],
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>> {
        self.handler.execute(
            args.iter().map(|s| s.to_string()).collect(),
            self.args_info.clone(),
        )
    }

    /// Returns the string description of the argument types
    pub fn arg_types(&self) -> Vec<String> {
        self.args_info
            .iter()
            .map(|info| {
                let info_string = info.clone().to_string();
                let parts = info_string.split(':').collect::<Vec<_>>();
                parts[1].to_string()
            })
            .collect()
    }
}

pub fn validate(
    args: Vec<String>,
    arg_infos: Vec<CommandArgInfo>,
) -> std::result::Result<(), ArgsError> {
    if args.len() != arg_infos.len() {
        return Err(ArgsError::WrongNumberOfArguments {
            got: args.len(),
            expected: arg_infos.len(),
        });
    }

    for (i, arg_value) in args.iter().enumerate() {
        let arg_info = arg_infos[i].clone();
        let arg_type: CommandArgType = arg_info.arg_type;
        match arg_type {
            CommandArgType::I32 => {
                if let Err(err) = &arg_value.parse::<i32>() {
                    return Err(ArgsError::WrongArgumentValue {
                        argument: arg_value.to_string(),
                        error: err.to_string(),
                    });
                }
            }
            CommandArgType::F32 => {
                if let Err(err) = &arg_value.parse::<f32>() {
                    return Err(ArgsError::WrongArgumentValue {
                        argument: arg_value.to_string(),
                        error: err.to_string(),
                    });
                }
            }
            CommandArgType::String => (),
            CommandArgType::Custom => (),
        }
    }

    Ok(())
}

/// Command handler.
///
/// It should return the status in case of correct execution. In case of
/// errors, all the errors will be handled by the REPL, except for
/// [`CriticalError`], which will be passed up from the REPL.
///
/// The handler should validate command arguments and can return [`ArgsError`]
/// to indicate that arguments were wrong.
pub type Handler<'a> =
    dyn 'a + FnMut(&[&str]) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + 'a>>;

/// Return status of a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandStatus {
    /// Indicates that REPL should continue execution
    Done,
    /// Indicates that REPL should quit
    Quit,
}

/// Special error wrapper used to indicate that a critical error occured.
///
/// [`Handler`] can return [`CriticalError`] to indicate that this error
/// should not be handled by the REPL (which just prints error message
/// and continues for all other errors).
///
/// This is most conveniently used via the [`Critical`] extension trait.
#[derive(Debug, thiserror::Error)]
pub enum CriticalError {
    /// The contained error is critical and should be returned back from REPL.
    #[error(transparent)]
    Critical(#[from] anyhow::Error),
}

/// Extension trait to easily wrap errors in [`CriticalError`].
///
/// This is implemented for [`std::result::Result`] so can be used to coveniently
/// wrap errors that implement [`std::error::Error`] to indicate that they are
/// critical and should be returned by the REPL, for example:
/// ```rust
/// # use mini_async_repl::{CriticalError, Critical};
/// let result: Result<(), std::fmt::Error> = Err(std::fmt::Error);
/// let critical = result.into_critical();
/// assert!(matches!(critical, Err(CriticalError::Critical(_))));
/// ```
///
/// See `examples/errors.rs` for a concrete usage example.
pub trait Critical<T, E> {
    /// Wrap the contained [`Err`] in [`CriticalError`] or leave [`Ok`] untouched
    fn into_critical(self) -> Result<T, CriticalError>;
}

impl<T, E> Critical<T, E> for Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn into_critical(self) -> Result<T, CriticalError> {
        self.map_err(|e| CriticalError::Critical(e.into()))
    }
}

/// Wrong command arguments.
#[allow(missing_docs)]
#[derive(Debug, thiserror::Error)]
pub enum ArgsError {
    #[error("wrong number of arguments: got {got}, expected {expected}")]
    WrongNumberOfArguments { got: usize, expected: usize },
    #[error("failed to parse argument value '{argument}': {error}")]
    WrongArgumentValue { argument: String, error: String },
    #[error("no command variant found for provided args")]
    NoVariantFound,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validator_no_args() {
        let arg_types = vec![];
        assert!(validate(vec![], arg_types.clone()).is_ok());
        assert!(validate(vec!["hello".into()], arg_types.clone()).is_err())
    }

    #[test]
    fn validator_one_arg() {
        let arg_types = vec![CommandArgInfo::new(CommandArgType::I32)];
        assert!(validate(vec![], arg_types.clone()).is_err());
        assert!(validate(vec!["hello".into()], arg_types.clone()).is_err());
        assert!(validate(vec!["13".into()], arg_types.clone()).is_ok())
    }

    #[test]
    fn validator_multiple_args() {
        let arg_types = vec![
            CommandArgInfo::new(CommandArgType::I32),
            CommandArgInfo::new(CommandArgType::F32),
            CommandArgInfo::new(CommandArgType::String),
        ];

        assert!(validate(vec![], arg_types.clone()).is_err());
        assert!(validate(
            vec!["1".into(), "2.1".into(), "hello".into()],
            arg_types.clone()
        )
        .is_ok());
        assert!(validate(
            vec!["1.2".into(), "2.1".into(), "hello".into()],
            arg_types.clone()
        )
        .is_err());
        assert!(validate(
            vec!["1".into(), "a".into(), "hello".into()],
            arg_types.clone()
        )
        .is_err());
        assert!(validate(
            vec!["1".into(), "2.1".into(), "hello".into(), "world".into()],
            arg_types.clone()
        )
        .is_err());
    }

    #[tokio::test]
    async fn manual_command() {
        let mut cmd = Command::new(
            "Test command",
            vec![CommandArgInfo::new(CommandArgType::String)],
            Box::new(TrivialCommandHandler::new()),
        );
        let result = cmd.execute(&["hello"]).await;

        match result {
            Ok(CommandStatus::Done) => {}
            _ => panic!("Wrong variant"),
        }
    }

    #[tokio::test]
    async fn command_with_args() {
        let mut cmd = Command::new(
            "Example cmd",
            vec![
                CommandArgInfo::new(CommandArgType::I32),
                CommandArgInfo::new(CommandArgType::F32),
            ],
            Box::new(TrivialCommandHandler::new()),
        );
        let result = cmd.execute(&["13", "1.1"]).await;

        match result {
            Ok(CommandStatus::Done) => {}
            Ok(v) => panic!("Wrong variant: {:?}", v),
            Err(e) => panic!("Error: {:?}", e),
        };
    }

    #[tokio::test]
    async fn command_with_critical() {
        struct WithCriticalCommandHandler {}
        impl WithCriticalCommandHandler {
            fn new() -> Self {
                WithCriticalCommandHandler {}
            }

            async fn handle_command(
                &mut self,
                _args: Vec<String>,
            ) -> anyhow::Result<CommandStatus> {
                let err = std::io::Error::new(std::io::ErrorKind::InvalidData, "example error");
                Err(CriticalError::Critical(err.into()).into())
            }
        }

        impl ExecuteCommand for WithCriticalCommandHandler {
            fn execute(
                &mut self,
                args: Vec<String>,
                _args_info: Vec<CommandArgInfo>,
            ) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>> {
                Box::pin(self.handle_command(args))
            }
        }

        let mut cmd = Command::new(
            "Example cmd",
            vec![
                CommandArgInfo::new(CommandArgType::I32),
                CommandArgInfo::new(CommandArgType::F32),
            ],
            Box::new(WithCriticalCommandHandler::new()),
        );
        let result = cmd.execute(&["13", "1.1"]).await;

        match result {
            Ok(v) => panic!("Wrong variant: {:?}", v),
            Err(e) => {
                if e.downcast_ref::<CriticalError>().is_none() {
                    panic!("Wrong error: {:?}", e)
                }
            }
        };
    }
}
