//! JSON report contract: one object per command on stdout.
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Error,
    Warning,
}

#[derive(Serialize, Clone, Debug)]
pub struct Finding {
    pub file: Option<String>,
    pub line: Option<u32>,
    pub col: Option<u32>,
    pub level: Level,
    pub rule: String,
    pub msg: String,
}

impl Finding {
    fn new(level: Level, rule: &str, msg: &str) -> Self {
        Finding { file: None, line: None, col: None, level, rule: rule.into(), msg: msg.into() }
    }
    pub fn error(rule: &str, msg: &str) -> Self {
        Self::new(Level::Error, rule, msg)
    }
    pub fn warning(rule: &str, msg: &str) -> Self {
        Self::new(Level::Warning, rule, msg)
    }
    pub fn at(mut self, file: &str, line: u32, col: u32) -> Self {
        self.file = Some(file.into());
        self.line = Some(line);
        self.col = Some(col);
        self
    }
}

#[derive(Serialize, Debug)]
pub struct Report {
    pub ok: bool,
    pub command: String,
    pub data: Value,
    pub errors: Vec<Finding>,
}

impl Report {
    /// `ok` is derived: false only when an error-level finding exists.
    pub fn new(command: &str, data: Value, errors: Vec<Finding>) -> Self {
        let ok = !errors.iter().any(|f| f.level == Level::Error);
        Report { ok, command: command.into(), data, errors }
    }

    pub fn exit_code(&self) -> i32 {
        if self.ok {
            0
        } else {
            1
        }
    }

    /// Print (stdout JSON, or text) and return the exit code (0/1).
    pub fn emit(&self, json: bool) -> i32 {
        if json {
            println!("{}", serde_json::to_string(self).expect("report serializes"));
        } else {
            println!("{}: {}", self.command, if self.ok { "ok" } else { "failed" });
            for f in &self.errors {
                let lvl = if f.level == Level::Error { "error" } else { "warning" };
                let loc = match (&f.file, f.line, f.col) {
                    (Some(p), Some(l), Some(c)) => format!("{p}:{l}:{c}: "),
                    (Some(p), _, _) => format!("{p}: "),
                    _ => String::new(),
                };
                println!("{loc}{lvl}[{}]: {}", f.rule, f.msg);
            }
        }
        self.exit_code()
    }
}

/// Environment/usage error (exit 2). Prints JSON to stdout when `json`, else text to stderr.
pub fn env_error_in(json: bool, command: &str, msg: &str) -> i32 {
    if json {
        let r = json!({"ok": false, "command": command, "data": {}, "errors": [Finding::error("env", msg)]});
        println!("{r}");
    } else {
        eprintln!("{command}: {msg}");
    }
    2
}

pub fn env_error(command: &str, msg: &str) -> i32 {
    env_error_in(false, command, msg)
}
