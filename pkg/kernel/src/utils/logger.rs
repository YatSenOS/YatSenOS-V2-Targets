use log::{LevelFilter, Metadata, Record};
use owo_colors::OwoColorize;

pub fn init() {
    static LOGGER: Logger = Logger;
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(LevelFilter::Info);

    info!("Current log level: {}", log::max_level());

    info!("Logger Initialized.");
}

struct Logger;

impl log::Log for Logger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        match record.level() {
            log::Level::Error => println_warn!(
                "{} {}@{}:{}",
                "[E]".red().bold(),
                record.file_static().unwrap_or("").bold(),
                record.line().unwrap_or(0),
                record.args()
            ),
            log::Level::Warn => {
                println_warn!("{} {}", "[!]".yellow().bold(), record.args().yellow())
            }
            log::Level::Info => println!("{} {}", "[+]".green().bold(), record.args().green()),
            log::Level::Debug => {
                println_serial!("{} {}", "[D]".blue().bold(), record.args().blue())
            }
            log::Level::Trace => {
                println_serial!("{} {}", "[T]".dimmed().bold(), record.args().dimmed())
            }
        }
    }

    fn flush(&self) {}
}
