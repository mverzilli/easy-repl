//! Implementation of [`Command`]s with utilities that help to crate them.

use anyhow;
use thiserror;

use std::pin::Pin;
use std::future::Future;
use std::fmt::Display;
use std::fmt::Formatter;

pub trait ExecuteCommand {
    fn execute(&mut self, args: Vec<String>) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>>;
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
    fn execute(&mut self, args: Vec<String>) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>> {
        Box::pin(self.handle_command(args))
    }
}

#[derive(Clone)]
pub struct CommandArgInfo {
    pub arg_type: CommandArgType,
    pub name: Option<String>
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


pub struct NewCommand {
    /// Command desctiption that will be displayed in the help message
    pub description: String,
    /// Names and types of arguments to the command
    pub args_info: Vec<CommandArgInfo>,
    /// Command handler which should validate arguments and perform command logic
    pub handler: Box<dyn ExecuteCommand>,
}

impl NewCommand {
    pub fn execute(&mut self, args: &[&str]) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> +'_>> {
        self.handler.execute(args.iter().map(|s| s.to_string()).collect())
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

pub struct Validator {}
impl Validator {
    pub fn validate(args: Vec<String>, arg_infos: Vec<CommandArgInfo>) -> std::result::Result<(), ArgsError> {        
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
                            error: err.to_string()
                        });
                    }
                },
                CommandArgType::F32 => {
                  if let Err(err) = &arg_value.parse::<f32>() {
                        return Err(ArgsError::WrongArgumentValue {
                            argument: arg_value.to_string(),
                            error: err.to_string()
                        });
                    }  
                }
                CommandArgType::String => (),
                CommandArgType::Custom => ()
            }
        }

        Ok(())
    }
}


// #[macro_export]
// macro_rules! validator {
//     ($($type:ty),*) => {
//         |args: &[&str]| -> std::result::Result<(), $crate::command::ArgsError> {
//             // check the number of arguments
//             let n_args: usize = <[()]>::len(&[ $( $crate::validator!(@replace $type ()) ),* ]);
//             if args.len() != n_args {
//                 return Err($crate::command::ArgsError::WrongNumberOfArguments {
//                     got: args.len(),
//                     expected: n_args,
//             });
//             }
//             #[allow(unused_variables, unused_mut)]
//             let mut i = 0;
//             #[allow(unused_assignments)]
//             {
//                 $(
//                     if let Err(err) = args[i].parse::<$type>() {
//                         return Err($crate::command::ArgsError::WrongArgumentValue {
//                             argument: args[i].into(),
//                             error: err.into()
//                     });
//                     }
//                     i += 1;
//                 )*
//             }

//             Ok(())
//         }
//     };
//     // Helper that allows to replace one expression with another (possibly "noop" one)
//     (@replace $_old:tt $new:expr) => { $new };
// }


/// Command handler.
///
/// It should return the status in case of correct execution. In case of
/// errors, all the errors will be handled by the REPL, except for
/// [`CriticalError`], which will be passed up from the REPL.
///
/// The handler should validate command arguments and can return [`ArgsError`]
/// to indicate that arguments were wrong.
pub type Handler<'a> = dyn 'a + FnMut(&[&str]) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + 'a>>;

/// Single command that can be called in the REPL.
///
/// Though it is possible to construct it by manually, it is not advised.
/// One should rather use the provided [`command!`] macro which will generate
/// appropriate arguments validation and `args_info` based on passed specification.
pub struct Command<'a> {
    /// Command desctiption that will be displayed in the help message
    pub description: String,
    /// Names and types of arguments to the command
    pub args_info: Vec<String>,
    /// Command handler which should validate arguments and perform command logic
    pub handler: Box<Handler<'a>>,

}

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
/// # use easy_repl::{CriticalError, Critical};
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
    WrongArgumentValue {
        argument: String,
        error: String,
    },
    #[error("no command variant found for provided args")]
    NoVariantFound,
}

impl<'a> Command<'a> {
    /// Validate the arguments and invoke the handler if arguments are correct.
    pub async fn run(&mut self, args: &[&str]) -> anyhow::Result<CommandStatus> {
        (self.handler)(args).await
    }

    /// Returns the string description of the argument types
    pub fn arg_types(&self) -> Vec<&str> {
        self.args_info
            .iter()
            .map(|info| info.split(':').collect::<Vec<_>>()[1])
            .collect()
    }
}

impl<'a> std::fmt::Debug for Command<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Command")
            .field("description", &self.description)
            .finish()
    }
}

// #[macro_export]
// macro_rules! validator {
//     ($($type:ty),*) => {
//         |args: &[&str]| -> std::result::Result<(), $crate::command::ArgsError> {
//             // check the number of arguments
//             let n_args: usize = <[()]>::len(&[ $( $crate::validator!(@replace $type ()) ),* ]);
//             if args.len() != n_args {
//                 return Err($crate::command::ArgsError::WrongNumberOfArguments {
//                     got: args.len(),
//                     expected: n_args,
//             });
//             }
//             #[allow(unused_variables, unused_mut)]
//             let mut i = 0;
//             #[allow(unused_assignments)]
//             {
//                 $(
//                     if let Err(err) = args[i].parse::<$type>() {
//                         return Err($crate::command::ArgsError::WrongArgumentValue {
//                             argument: args[i].into(),
//                             error: err.into()
//                     });
//                     }
//                     i += 1;
//                 )*
//             }

//             Ok(())
//         }
//     };
//     // Helper that allows to replace one expression with another (possibly "noop" one)
//     (@replace $_old:tt $new:expr) => { $new };
// }

#[macro_export]
macro_rules! command {
    ($description:expr, ( $($( $name:ident )? : $type:ty),* ) => $handler:expr $(,)?) => {
        $crate::command::Command {
            description: $description.into(),
            args_info: vec![ $(
                concat!($(stringify!($name), )? ":", stringify!($type)).into()
            ),* ], // TODO
            handler: command!(@handler $($type)*, $handler),
        }
    };
    (@handler $($type:ty)*, $handler:expr) => {
        Box::new( move |#[allow(unused_variables)] args| -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>> {
            let args = args.clone();
            Box::pin(async move {
                let validator = $crate::validator!($($type),*);
                validator(args)?;
                #[allow(unused_mut)]
                let mut handler = $handler;
                command!(@handler_call handler; args; $($type;)*)
            })            
        })
    };

    // transform element of $args into parsed function argument by calling .parse::<$type>().unwrap()
    // on each, this starts a recursive muncher that constructs following argument getters args[i]
    (@handler_call $handler:ident; $args:ident; $($types:ty;)*) => {
        command!(@handler_call $handler, $args, 0; $($types;)* =>)
    };
    // $num is used to index $args; pop $type from beginning of list, add new parsed at the endo of $parsed
    (@handler_call $handler:ident, $args:ident, $num:expr; $type:ty; $($types:ty;)* => $($parsed:expr;)*) => {
        command!(@handler_call $handler, $args, $num + 1;
            $($types;)* =>
            $($parsed;)* $args[$num].parse::<$type>().unwrap();
        )
    };
    // finally when there are no more types emit code that calls the handler with all arguments parsed
    (@handler_call $handler:ident, $args:ident, $num:expr; => $($parsed:expr;)*) => {
        $handler( $($parsed),* )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validator_no_args() {
        let arg_types = vec![];
        assert!(Validator::validate(vec![], arg_types.clone()).is_ok());
        assert!(Validator::validate(vec!["hello".into()], arg_types.clone()).is_err())
    }

    #[test]
    fn validator_one_arg() {
        let arg_types = vec![CommandArgInfo::new(CommandArgType::I32)];
        assert!(Validator::validate(vec![], arg_types.clone()).is_err());
        assert!(Validator::validate(vec!["hello".into()], arg_types.clone()).is_err());
        assert!(Validator::validate(vec!["13".into()], arg_types.clone()).is_ok())
    }

    #[test]
    fn validator_multiple_args() {
        let arg_types = vec![CommandArgInfo::new(CommandArgType::I32), CommandArgInfo::new(CommandArgType::F32), CommandArgInfo::new(CommandArgType::String)];

        assert!(Validator::validate(vec![], arg_types.clone()).is_err());
        assert!(Validator::validate(vec!["1".into(), "2.1".into(), "hello".into()], arg_types.clone()).is_ok());
        assert!(Validator::validate(vec!["1.2".into(), "2.1".into(), "hello".into()], arg_types.clone()).is_err());
        assert!(Validator::validate(vec!["1".into(), "a".into(), "hello".into()], arg_types.clone()).is_err());
        assert!(Validator::validate(vec!["1".into(), "2.1".into(), "hello".into(), "world".into()], arg_types.clone()).is_err());
    }

    #[tokio::test]
    async fn manual_command() {
        let mut cmd = NewCommand {
            description: "Test command".into(),
            args_info: vec![CommandArgInfo::new(CommandArgType::String)],
            handler: Box::new(TrivialCommandHandler::new())
        };
        let result = cmd.execute(&["hello"]).await;

        match result {
            Ok(CommandStatus::Done) => {},
            _ => panic!("Wrong variant")
        }
    }

    #[tokio::test]
    async fn command_with_args() {
        let mut cmd = NewCommand {
            description: "Example cmd".into(),
            args_info: vec![CommandArgInfo::new(CommandArgType::I32), CommandArgInfo::new(CommandArgType::F32)],
            handler: Box::new(TrivialCommandHandler::new())
        };
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

            async fn handle_command(&mut self, _args: Vec<String>) -> anyhow::Result<CommandStatus> {
                let err = std::io::Error::new(std::io::ErrorKind::InvalidData, "example error");
                Err(CriticalError::Critical(err.into()).into())
            }
        }

        impl ExecuteCommand for WithCriticalCommandHandler {
            fn execute(&mut self, args: Vec<String>) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>> {
                Box::pin(self.handle_command(args))
            }
        }

        let mut cmd = NewCommand {
            description: "Example cmd".into(),
            args_info: vec![CommandArgInfo::new(CommandArgType::I32), CommandArgInfo::new(CommandArgType::F32)],
            handler: Box::new(WithCriticalCommandHandler::new())
        };
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
