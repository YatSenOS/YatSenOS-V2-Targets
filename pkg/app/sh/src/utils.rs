use lib::*;
use owo_colors::{AnsiColors, OwoColorize};
use string::String;

const COLORS: [AnsiColors; 6] = [
    AnsiColors::Red,
    AnsiColors::Yellow,
    AnsiColors::Green,
    AnsiColors::Blue,
    AnsiColors::Cyan,
    AnsiColors::Magenta,
];

fn rainbow_text(text: &str) -> String {
    let mut result = String::with_capacity(text.len() * 3);
    let mut color_idx = 0;
    for c in text.chars() {
        result.push_str(&format!("{}", c.color(COLORS[color_idx])));
        color_idx = (color_idx + 1) % COLORS.len();
    }
    result
}

pub fn show_welcome_text() {
    const WELCOME_TEXT: &str = "Welcome to YatSenOS shell";
    println!(
        "{:>16} {} {:<}",
        "<<<".bold(),
        rainbow_text(WELCOME_TEXT).bold(),
        ">>>".bold()
    );
    println!("{: >60}", "type `help` for help".bold());
}

const VERSION_STR: &str = concat!("YatSenOS shell v", env!("CARGO_PKG_VERSION"));

struct Action(&'static str, Option<&'static str>, &'static str);

const ACTIONS_MAP: [Action; 9] = [
    Action("help", None, "show this help"),
    Action("ps", None, "show process list"),
    Action("ls", None, "list directory"),
    Action("cd", Some("<path>"), "change directory"),
    Action("cat", Some("<file>"), "show file content"),
    Action("exec", Some("<file>"), "execute file"),
    Action("nohup", Some("<file>"), "execute file in background"),
    Action("kill", Some("<pid>"), "kill process"),
    Action("clear", None, "clear screen"),
];

const SHORTCUTS: [Action; 2] = [
    Action("Ctrl + D", None, "exit shell"),
    Action("Ctrl + C", None, "cancel current command"),
];

fn format_cmds(cate: &str, actions: &[Action]) -> String {
    let mut result = String::new();
    result.push_str(&format!("{}\n", cate.bright_green().bold()));
    for action in actions {
        let action_str = match action.1 {
            Some(arg) => format!("{} {}", action.0.cyan().bold(), arg.bright_cyan()),
            None => format!("{}", action.0.cyan().bold()),
        };

        let real_len = action.0.len() + action.1.map_or(0, |arg| arg.len() + 1);
        let blank_left = 16 - real_len;

        result.push_str(&format!(
            "{:>width$} | {}\n",
            action_str,
            action.2.dimmed(),
            width = action_str.len() + blank_left,
        ));
    }
    result
}

pub fn show_help_text() {
    println!("\n{} by {}\n", VERSION_STR.bold(), "GZTime".bold());
    println!("{}", format_cmds("Usage", &ACTIONS_MAP));
    println!("{}", format_cmds("Shortcuts", &SHORTCUTS));
}
