//! Main REPL logic.

use std::{collections::HashMap, io::Write, rc::Rc};

use rustyline::{self, completion::FilenameCompleter, error::ReadlineError};
use shell_words;
use textwrap;
use thiserror;
use trie_rs::{Trie, TrieBuilder};

use crate::command::{ArgsError, CommandStatus, CriticalError, NewCommand};
use crate::completion::{completion_candidates, Completion};

/// Reserved command names. These commands are always added to REPL.
pub const RESERVED: &[(&str, &str)] = &[("help", "Show this help message"), ("quit", "Quit repl")];

/// Read-eval-print loop.
///
/// REPL is ment do be constructed using the builder pattern via [`Repl::builder()`].
/// Commands are added during building and currently cannot be added/removed/modified
/// after [`Repl`] has been built. This is because the names are used to generate Trie
/// with all the names for fast name lookup and completion.
///
/// [`Repl`] can be used in two ways: one can use the [`Repl::run`] method directly to just
/// start the evaluation loop, or [`Repl::next`] can be used to get back control between
/// loop steps.
pub struct Repl {
    description: String,
    prompt: String,
    text_width: usize,
    commands: HashMap<String, Vec<NewCommand>>,
    trie: Rc<Trie<u8>>,
    editor: rustyline::Editor<Completion>,
    out: Box<dyn Write>,
    predict_commands: bool,
}

/// State of the REPL after command execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LoopStatus {
    /// REPL should continue execution.
    Continue,
    /// Should break of evaluation loop (quit command or end of input).
    Break,
}

/// Builder pattern implementation for [`Repl`].
///
/// All setter methods take owned `self` so the calls can be chained, for example:
/// ```rust
/// # use mini_async_repl::Repl;
/// let repl = Repl::builder()
///     .description("My REPL")
///     .prompt("repl> ")
///     .build()
///     .expect("Failed to build REPL");
/// ```
pub struct ReplBuilder {
    commands: Vec<(String, NewCommand)>,
    description: String,
    prompt: String,
    text_width: usize,
    editor_config: rustyline::config::Config,
    out: Box<dyn Write>,
    with_hints: bool,
    with_completion: bool,
    with_filename_completion: bool,
    predict_commands: bool,
}

/// Error when building REPL.
#[derive(Debug, thiserror::Error)]
pub enum BuilderError {
    /// More than one command have the same.
    #[error("more than one command with name '{0}' added")]
    DuplicateCommands(String),
    /// Given command name is not valid.
    #[error("name '{0}' cannot be parsed correctly, thus would be impossible to call")]
    InvalidName(String),
    /// Command name is one of [`RESERVED`] names.
    #[error("'{0}' is a reserved command name")]
    ReservedName(String),
}

pub(crate) fn split_args(line: &str) -> Result<Vec<String>, shell_words::ParseError> {
    shell_words::split(line)
}

impl Default for ReplBuilder {
    fn default() -> Self {
        ReplBuilder {
            prompt: "> ".into(),
            text_width: 80,
            description: Default::default(),
            commands: Default::default(),
            out: Box::new(std::io::stderr()),
            editor_config: rustyline::config::Config::builder()
                .output_stream(rustyline::OutputStreamType::Stderr) // NOTE: cannot specify `out`
                .completion_type(rustyline::CompletionType::List)
                .build(),
            with_hints: true,
            with_completion: true,
            with_filename_completion: false,
            predict_commands: true,
        }
    }
}

macro_rules! setters {
    ($( $(#[$meta:meta])* $name:ident: $type:ty )+) => {
        $(
            $(#[$meta])*
            pub fn $name<T: Into<$type>>(mut self, v: T) -> Self {
                self.$name = v.into();
                self
            }
        )+
    };
}

impl ReplBuilder {
    setters! {
        /// Repl description shown in [`Repl::help`]. Defaults to an empty string.
        description: String
        /// Prompt string, defaults to `"> "`.
        prompt: String
        /// Width of the text used when wrapping the help message. Defaults to 80.
        text_width: usize
        /// Configuration for [`rustyline`]. Some sane defaults are used.
        editor_config: rustyline::config::Config
        /// Where to print REPL output. By default [`std::io::Stderr`] is used.
        ///
        /// Note that [`rustyline`] will always use [`std::io::Stderr`] or [`std::io::Stdout`].
        /// These must be configured in [`ReplBuilder::editor_config`], and currently there seems to be no way
        /// to use other output stream for [`rustyline`] (which probably also makes little sense).
        out: Box<dyn Write>
        /// Print command hints. Defaults to `true`.
        ///
        /// Hints will show the end of a command if there is only one avaliable.
        /// For example, assuming commands `"move"` and `"make"`, in the following position (`|`
        /// indicates the cursor):
        /// ```text
        /// > mo|
        /// ```
        /// a hint will be shown as
        /// ```text
        /// > mo|ve
        /// ```
        /// but when there is only
        /// ```text
        /// > m|
        /// ```
        /// then no hints will be shown.
        with_hints: bool
        /// Use completion. Defaults to `true`.
        with_completion: bool
        /// Add filename completion, besides command completion. Defaults to `false`.
        with_filename_completion: bool
        /// Execute commands when entering incomplete names. Defaults to `true`.
        ///
        /// With this option commands can be executed by entering only part of command name.
        /// If there is only a single command mathing given prefix, then it will be executed.
        /// For example, with commands `"make"` and "`move`", entering just `mo` will resolve
        /// to `move` and the command will be executed, but entering `m` will result in an error.
        predict_commands: bool
    }

    /// Add a command with given `name`. Use along with the [`command!`] macro.
    pub fn add(mut self, name: &str, cmd: NewCommand) -> Self {
        self.commands.push((name.into(), cmd));
        self
    }

    /// Finalize the configuration and return the REPL or error.
    pub fn build(self) -> Result<Repl, BuilderError> {
        let mut commands: HashMap<String, Vec<NewCommand>> = HashMap::new();
        let mut trie = TrieBuilder::new();
        for (name, cmd) in self.commands {
            let cmds = commands.entry(name.clone()).or_default();
            let args = split_args(&name).map_err(|_e| BuilderError::InvalidName(name.clone()))?;
            if args.len() != 1 || name.is_empty() {
                return Err(BuilderError::InvalidName(name));
            } else if RESERVED.iter().any(|(n, _)| *n == name) {
                return Err(BuilderError::ReservedName(name));
            } else if cmds.iter().any(|c| c.arg_types() == cmd.arg_types()) {
                return Err(BuilderError::DuplicateCommands(name));
            }
            cmds.push(cmd);
            trie.push(name);
        }
        for (name, _) in RESERVED.iter() {
            trie.push(name);
        }

        let trie = Rc::new(trie.build());
        let helper = Completion {
            trie: trie.clone(),
            with_hints: self.with_hints,
            with_completion: self.with_completion,
            filename_completer: if self.with_filename_completion {
                Some(FilenameCompleter::new())
            } else {
                None
            },
        };
        let mut editor = rustyline::Editor::with_config(self.editor_config);
        editor.set_helper(Some(helper));

        Ok(Repl {
            description: self.description,
            prompt: self.prompt,
            text_width: self.text_width,
            commands,
            trie,
            editor,
            out: self.out,
            predict_commands: self.predict_commands,
        })
    }
}

impl Repl {
    /// Start [`ReplBuilder`] with default values.
    pub fn builder() -> ReplBuilder {
        ReplBuilder::default()
    }

    fn format_help_entries(&self, entries: &[(String, String)]) -> String {
        if entries.is_empty() {
            return String::new();
        }
        let width = entries
            .iter()
            .map(|(sig, _)| sig)
            .max_by_key(|sig| sig.len())
            .unwrap()
            .len();
        entries
            .iter()
            .map(|(sig, desc)| {
                let indent = " ".repeat(width + 2 + 2);
                let opts = textwrap::Options::new(self.text_width)
                    .initial_indent("")
                    .subsequent_indent(&indent);
                let line = format!("  {sig:width$}  {desc}");
                textwrap::fill(&line, opts)
            })
            .fold(String::new(), |mut out, next| {
                out.push('\n');
                out.push_str(&next);
                out
            })
    }

    /// Returns formatted help message.
    pub fn help(&self) -> String {
        let mut names: Vec<_> = self.commands.keys().collect();
        names.sort();

        let signature =
            |name: &String, args_info: &Vec<String>| format!("{} {}", name, args_info.join(" "));
        let user: Vec<_> = self
            .commands
            .iter()
            .flat_map(|(name, cmds)| {
                cmds.iter()
                    .map(move |cmd| (signature(name, &cmd.arg_types()), cmd.description.clone()))
            })
            .collect();

        let other: Vec<_> = RESERVED
            .iter()
            .map(|(name, desc)| ((*name).to_string(), desc.to_string()))
            .collect();

        let msg = format!(
            r#"
{}

Available commands:
{}

Other commands:
{}
        "#,
            self.description,
            self.format_help_entries(&user),
            self.format_help_entries(&other)
        );
        msg.trim().into()
    }

    async fn handle_line(&mut self, line: &str) -> anyhow::Result<LoopStatus> {
        // if there is any parsing error just continue to next input
        let args = match split_args(line) {
            Err(err) => {
                writeln!(&mut self.out, "Error: {err}")?;
                return Ok(LoopStatus::Continue);
            }
            Ok(args) => args,
        };
        let prefix = &args[0];
        let mut candidates = completion_candidates(&self.trie, prefix);
        let exact = !candidates.is_empty() && &candidates[0] == prefix;
        let can_take_first = !candidates.is_empty() && (exact || self.predict_commands);
        if !can_take_first {
            writeln!(&mut self.out, "Command not found: {prefix}")?;
            if candidates.len() > 1 || (!self.predict_commands && !exact) {
                candidates.sort();
                writeln!(&mut self.out, "Candidates:\n  {}", candidates.join("\n  "))?;
            }
            writeln!(&mut self.out, "Use 'help' to see available commands.")?;
            Ok(LoopStatus::Continue)
        } else {
            let name = &candidates[0];
            let tail: Vec<_> = args[1..].iter().map(String::as_str).collect();
            match self.handle_command(name, &tail).await {
                Ok(CommandStatus::Done) => Ok(LoopStatus::Continue),
                Ok(CommandStatus::Quit) => Ok(LoopStatus::Break),
                Err(err) if err.downcast_ref::<CriticalError>().is_some() => Err(err),
                Err(err) => {
                    // other errors are handled here
                    writeln!(&mut self.out, "Error: {err}")?;
                    if err.is::<ArgsError>() {
                        // in case of ArgsError we know it could not have been a reserved command
                        let cmds = self.commands.get_mut(name).unwrap();
                        writeln!(&mut self.out, "Usage:")?;
                        for cmd in cmds.iter() {
                            writeln!(
                                &mut self.out,
                                "  {} {}",
                                name,
                                cmd.args_info
                                    .clone()
                                    .into_iter()
                                    .map(|info| info.to_string())
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            )?;
                        }
                    }
                    Ok(LoopStatus::Continue)
                }
            }
        }
    }

    /// Run a single REPL iteration and return whether this is the last one or not.
    pub async fn next(&mut self) -> anyhow::Result<LoopStatus> {
        match self.editor.readline(&self.prompt) {
            Ok(line) => {
                if !line.trim().is_empty() {
                    self.editor.add_history_entry(line.trim());
                    self.handle_line(&line).await
                } else {
                    Ok(LoopStatus::Continue)
                }
            }
            Err(ReadlineError::Interrupted) => {
                writeln!(&mut self.out, "CTRL-C")?;
                Ok(LoopStatus::Break)
            }
            Err(ReadlineError::Eof) => Ok(LoopStatus::Break),
            // TODO: not sure if these should be propagated or handler here
            Err(err) => {
                writeln!(&mut self.out, "Error: {err:?}")?;
                Ok(LoopStatus::Continue)
            }
        }
    }

    async fn handle_command(&mut self, name: &str, args: &[&str]) -> anyhow::Result<CommandStatus> {
        match name {
            "help" => {
                let help = self.help();
                writeln!(&mut self.out, "{help}")?;
                Ok(CommandStatus::Done)
            }
            "quit" => Ok(CommandStatus::Quit),
            _ => {
                // find_command must have returned correct name

                // if all commands are not possible to call because of argument error
                // return the last argument one as our result
                let mut last_arg_err = None;
                let cmds = self.commands.get_mut(name).unwrap();
                for cmd in cmds.iter_mut() {
                    match cmd.execute(args).await {
                        Err(e) => {
                            if !e.is::<ArgsError>() {
                                return Err(e);
                            } else {
                                last_arg_err = Some(Err(e));
                            }
                        }
                        other => return other,
                    }
                }
                // last_arg_err should always have at least a value here
                last_arg_err.unwrap()
            }
        }
    }

    /// Run the evaluation loop until [`LoopStatus::Break`] is received.
    pub async fn run(&mut self) -> anyhow::Result<()> {
        while self.next().await? == LoopStatus::Continue {}
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::{CommandArgInfo, CommandArgType, ExecuteCommand, TrivialCommandHandler};
    use std::future::Future;
    use std::pin::Pin;

    #[test]
    fn builder_duplicate() {
        let command_x_1 = NewCommand {
            description: "Command X".into(),
            args_info: vec![],
            handler: Box::new(TrivialCommandHandler::new()),
        };

        let command_x_2 = NewCommand {
            description: "Command X 2".into(),
            args_info: vec![],
            handler: Box::new(TrivialCommandHandler::new()),
        };

        let result = Repl::builder()
            .add("name_x", command_x_1)
            .add("name_x", command_x_2)
            .build();

        assert!(matches!(result, Err(BuilderError::DuplicateCommands(_))));
    }

    #[test]
    fn builder_overload() {
        let command_x_1 = NewCommand {
            description: "Command X".into(),
            args_info: vec![],
            handler: Box::new(TrivialCommandHandler::new()),
        };

        let command_x_2 = NewCommand {
            description: "Command X 2".into(),
            args_info: vec![CommandArgInfo::new(CommandArgType::I32)],
            handler: Box::new(TrivialCommandHandler::new()),
        };

        #[rustfmt::skip]
        let result = Repl::builder()
            .add("name_x", command_x_1)
            .add("name_x", command_x_2)
            .build();
        assert!(matches!(result, Ok(_)));
    }

    #[test]
    fn builder_empty() {
        let command_empty = NewCommand {
            description: "".into(),
            args_info: vec![],
            handler: Box::new(TrivialCommandHandler::new()),
        };

        let result = Repl::builder().add("", command_empty).build();
        assert!(matches!(result, Err(BuilderError::InvalidName(_))));
    }

    #[test]
    fn builder_spaces() {
        let command_empty = NewCommand {
            description: "".into(),
            args_info: vec![],
            handler: Box::new(TrivialCommandHandler::new()),
        };

        let result = Repl::builder()
            .add("name-with spaces", command_empty)
            .build();
        assert!(matches!(result, Err(BuilderError::InvalidName(_))));
    }

    #[test]
    fn builder_reserved() {
        let command_help = NewCommand {
            description: "".into(),
            args_info: vec![],
            handler: Box::new(TrivialCommandHandler::new()),
        };

        let result = Repl::builder().add("help", command_help).build();
        assert!(matches!(result, Err(BuilderError::ReservedName(_))));

        let command_quit = NewCommand {
            description: "".into(),
            args_info: vec![],
            handler: Box::new(TrivialCommandHandler::new()),
        };

        let result = Repl::builder().add("quit", command_quit).build();
        assert!(matches!(result, Err(BuilderError::ReservedName(_))));
    }

    #[tokio::test]
    async fn repl_quits() {
        let command_foo = NewCommand {
            description: "description".into(),
            args_info: vec![],
            handler: Box::new(TrivialCommandHandler::new()),
        };

        let mut repl = Repl::builder().add("foo", command_foo).build().unwrap();
        assert_eq!(
            repl.handle_line("quit".into()).await.unwrap(),
            LoopStatus::Break
        );

        struct QuittingCommandHandler {}
        impl QuittingCommandHandler {
            pub fn new() -> Self {
                Self {}
            }
            async fn handle_command(
                &mut self,
                _args: Vec<String>,
            ) -> anyhow::Result<CommandStatus> {
                Ok(CommandStatus::Quit)
            }
        }
        impl ExecuteCommand for QuittingCommandHandler {
            fn execute(
                &mut self,
                args: Vec<String>,
            ) -> Pin<Box<dyn Future<Output = anyhow::Result<CommandStatus>> + '_>> {
                Box::pin(self.handle_command(args))
            }
        }
        let command_quit = NewCommand {
            description: "description".into(),
            args_info: vec![],
            handler: Box::new(QuittingCommandHandler::new()),
        };

        let mut repl = Repl::builder().add("foo", command_quit).build().unwrap();
        assert_eq!(
            repl.handle_line("foo".into()).await.unwrap(),
            LoopStatus::Break
        );
    }
}
