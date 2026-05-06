use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;
use std::process::Child;
use std::process::ChildStdout;
use std::process::Command;
use std::process::ExitStatus;
use std::process::Stdio;

pub mod claude;
pub mod codex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentCommand {
    program: String,
    args: Vec<String>,
    current_dir: Option<PathBuf>,
}

impl AgentCommand {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            current_dir: None,
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn with_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn with_current_dir(mut self, current_dir: impl Into<PathBuf>) -> Self {
        self.current_dir = Some(current_dir.into());
        self
    }

    pub fn program(&self) -> &str {
        &self.program
    }

    pub fn args(&self) -> &[String] {
        &self.args
    }

    pub fn current_dir(&self) -> Option<&Path> {
        self.current_dir.as_deref()
    }

    pub fn to_std_command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command.args(&self.args);

        if let Some(current_dir) = &self.current_dir {
            command.current_dir(current_dir);
        }

        command
    }

    pub fn spawn_jsonl(&self) -> io::Result<JsonlChild> {
        let mut command = self.to_std_command();
        command.stdin(Stdio::null());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::inherit());

        let mut child = command.spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("agent command stdout was not piped"))?;

        Ok(JsonlChild {
            child,
            lines: JsonlLines::new(BufReader::new(stdout)),
        })
    }
}

#[derive(Debug)]
pub struct JsonlLines<R> {
    reader: R,
    buffer: String,
}

impl<R> JsonlLines<R>
where
    R: BufRead,
{
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            buffer: String::new(),
        }
    }
}

impl<R> Iterator for JsonlLines<R>
where
    R: BufRead,
{
    type Item = io::Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        self.buffer.clear();

        match self.reader.read_line(&mut self.buffer) {
            Ok(0) => None,
            Ok(_) => {
                while self.buffer.ends_with('\n') || self.buffer.ends_with('\r') {
                    self.buffer.pop();
                }

                Some(Ok(self.buffer.clone()))
            }
            Err(error) => Some(Err(error)),
        }
    }
}

#[derive(Debug)]
pub struct JsonlChild {
    child: Child,
    lines: JsonlLines<BufReader<ChildStdout>>,
}

impl JsonlChild {
    pub fn next_line(&mut self) -> io::Result<Option<String>> {
        match self.lines.next() {
            Some(line) => line.map(Some),
            None => Ok(None),
        }
    }

    pub fn wait(mut self) -> io::Result<ExitStatus> {
        self.child.wait()
    }
}

impl Iterator for JsonlChild {
    type Item = io::Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        self.lines.next()
    }
}
