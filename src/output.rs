use crate::error::SlackCliError;
use colored::Colorize;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Human,
    Json,
}

pub struct Output {
    format: OutputFormat,
    quiet: bool,
    pretty: bool,
}

impl Output {
    pub fn new(json: bool, quiet: bool, pretty: bool) -> Self {
        Self {
            format: if json {
                OutputFormat::Json
            } else {
                OutputFormat::Human
            },
            quiet,
            pretty,
        }
    }

    fn json_string<T: Serialize + ?Sized>(&self, data: &T) -> String {
        if self.pretty {
            serde_json::to_string_pretty(data).unwrap()
        } else {
            serde_json::to_string(data).unwrap()
        }
    }

    pub fn print<T: Serialize + HumanReadable>(&self, data: &T) {
        match self.format {
            OutputFormat::Json => {
                println!("{}", self.json_string(data));
            }
            OutputFormat::Human => {
                if !self.quiet {
                    data.print_human();
                }
            }
        }
    }

    pub fn print_list<T: Serialize + HumanReadable>(&self, items: &[T], title: &str) {
        match self.format {
            OutputFormat::Json => {
                println!("{}", self.json_string(items));
            }
            OutputFormat::Human => {
                if !self.quiet {
                    println!("{}", title.bold());
                    println!("{}", "─".repeat(40));
                    for item in items {
                        item.print_human();
                    }
                    println!("\n{} items", items.len());
                }
            }
        }
    }

    /// Print a JSON wrapper with items and extra top-level fields (e.g. total count).
    pub fn print_list_wrapped<T: Serialize + HumanReadable>(
        &self,
        items: &[T],
        title: &str,
        wrapper: &serde_json::Value,
    ) {
        match self.format {
            OutputFormat::Json => {
                println!("{}", self.json_string(wrapper));
            }
            OutputFormat::Human => {
                if !self.quiet {
                    println!("{}", title.bold());
                    println!("{}", "─".repeat(40));
                    for item in items {
                        item.print_human();
                    }
                    println!("\n{} items", items.len());
                }
            }
        }
    }

    pub fn success(&self, msg: &str) {
        if !self.quiet && self.format == OutputFormat::Human {
            println!("{} {}", "✓".green(), msg);
        }
    }

    pub fn status(&self, msg: &str) {
        if !self.quiet && self.format == OutputFormat::Human {
            eprintln!("{}", msg.dimmed());
        }
    }

    pub fn error(&self, msg: &str) {
        eprintln!("{} {}", "✗".red(), msg);
    }

    /// Print a structured error. When format is JSON, outputs {"error": "...", "code": "..."}
    /// to stderr. Otherwise falls back to the red X human-readable output.
    pub fn error_structured(&self, err: &SlackCliError) {
        match self.format {
            OutputFormat::Json => {
                let obj = serde_json::json!({
                    "error": err.to_string(),
                    "code": err.code(),
                });
                eprintln!("{}", self.json_string(&obj));
            }
            OutputFormat::Human => {
                self.error(&err.to_string());
            }
        }
    }

    pub fn is_json(&self) -> bool {
        self.format == OutputFormat::Json
    }
}

pub trait HumanReadable {
    fn print_human(&self);
}
