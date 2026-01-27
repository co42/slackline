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
}

impl Output {
    pub fn new(json: bool, quiet: bool) -> Self {
        Self {
            format: if json {
                OutputFormat::Json
            } else {
                OutputFormat::Human
            },
            quiet,
        }
    }

    pub fn print<T: Serialize + HumanReadable>(&self, data: &T) {
        match self.format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(data).unwrap());
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
                println!("{}", serde_json::to_string_pretty(items).unwrap());
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

    pub fn error(&self, msg: &str) {
        eprintln!("{} {}", "✗".red(), msg);
    }

    pub fn is_json(&self) -> bool {
        self.format == OutputFormat::Json
    }
}

pub trait HumanReadable {
    fn print_human(&self);
}
