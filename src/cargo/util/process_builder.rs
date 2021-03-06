use std::fmt;
use std::fmt::{Show,Formatter};
use std::os;
use std::path::Path;
use std::io::process::{Command,ProcessOutput,InheritFd};
use util::{CargoResult,io_error,process_error};
use std::collections::HashMap;

#[deriving(Clone,PartialEq)]
pub struct ProcessBuilder {
    program: String,
    args: Vec<String>,
    path: Vec<String>,
    env: HashMap<String, String>,
    cwd: Path
}

impl Show for ProcessBuilder {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        try!(write!(f, "`{}", self.program));

        if self.args.len() > 0 {
            try!(write!(f, " {}", self.args.connect(" ")));
        }

        write!(f, "`")
    }
}

// TODO: Upstream a Windows/Posix branch to Rust proper
static PATH_SEP : &'static str = ":";

impl ProcessBuilder {
    pub fn args<T: Show>(mut self, arguments: &[T]) -> ProcessBuilder {
        self.args = arguments.iter().map(|a| a.to_str()).collect();
        self
    }

    pub fn get_args<'a>(&'a self) -> &'a [String] {
        self.args.as_slice()
    }

    pub fn extra_path(mut self, path: Path) -> ProcessBuilder {
        // For now, just convert to a string, but we should do something better
        self.path.push(path.display().to_str());
        self
    }

    pub fn cwd(mut self, path: Path) -> ProcessBuilder {
        self.cwd = path;
        self
    }

    pub fn env(mut self, key: &str, val: Option<&str>) -> ProcessBuilder {
        match val {
            Some(v) => {
                self.env.insert(key.to_str(), v.to_str());
            },
            None => {
                self.env.remove(&key.to_str());
            }
        }

        self
    }

    // TODO: should InheritFd be hardcoded?
    pub fn exec(&self) -> CargoResult<()> {
        let mut command = self.build_command();
        command
            .env(self.build_env().as_slice())
            .stdout(InheritFd(1))
            .stderr(InheritFd(2));

        let exit = try!(command.status().map_err(io_error));

        if exit.success() {
            Ok(())
        } else {
            let msg = format!("Could not execute process `{}`", self.debug_string());
            Err(process_error(msg, exit, None))
        }
    }

    pub fn exec_with_output(&self) -> CargoResult<ProcessOutput> {
        let mut command = self.build_command();
        command.env(self.build_env().as_slice());

        let output = try!(command.output().map_err(io_error));

        if output.status.success() {
            Ok(output)
        } else {
            let msg = format!("Could not execute process `{}`", self.debug_string());
            Err(process_error(msg, output.status.clone(), Some(output)))
        }
    }

    fn build_command(&self) -> Command {
        let mut command = Command::new(self.program.as_slice());
        command.args(self.args.as_slice()).cwd(&self.cwd);
        command
    }

    fn debug_string(&self) -> String {
        format!("{} {}", self.program, self.args.connect(" "))
    }

    fn build_env(&self) -> Vec<(String, String)> {
        let mut ret = Vec::new();

        for (key, val) in self.env.iter() {
            // Skip path
            if key.as_slice() != "PATH" {
                ret.push((key.clone(), val.clone()));
            }
        }

        match self.build_path() {
            Some(path) => ret.push(("PATH".to_str(), path)),
            _ => ()
        }

        ret.as_slice().to_owned()
    }

    fn build_path(&self) -> Option<String> {
        let path = self.path.connect(PATH_SEP);

        match self.env.find_equiv(&("PATH")) {
            Some(existing) => {
                if self.path.is_empty() {
                    Some(existing.clone())
                } else {
                    Some(format!("{}{}{}", existing, PATH_SEP, path))
                }
            },
            None => {
                if self.path.is_empty() {
                    None
                } else {
                    Some(path)
                }
            }
        }
    }
}

pub fn process(cmd: &str) -> ProcessBuilder {
    ProcessBuilder {
        program: cmd.to_str(),
        args: vec!(),
        path: vec!(),
        cwd: os::getcwd(),
        env: system_env()
    }
}

fn system_env() -> HashMap<String, String> {
    let mut ret = HashMap::new();

    for &(ref key, ref val) in os::env().iter() {
        ret.insert(key.to_str(), val.to_str());
    }

    ret
}
