mod uefi;

#[macro_use]
mod macros;
#[macro_use]
mod regs;

pub mod clock;
pub mod func;
pub mod logger;
pub mod resource;

pub use macros::*;
pub use regs::*;
pub use resource::Resource;
use x86_64::instructions::interrupts;

pub const fn get_ascii_header() -> &'static str {
    concat!(
        r"
__  __      __  _____            ____  _____
\ \/ /___ _/ /_/ ___/___  ____  / __ \/ ___/
 \  / __ `/ __/\__ \/ _ \/ __ \/ / / /\__ \
 / / /_/ / /_ ___/ /  __/ / / / /_/ /___/ /
/_/\__,_/\__//____/\___/_/ /_/\____//____/

                                       v",
        env!("CARGO_PKG_VERSION")
    )
}

pub const fn get_header() -> &'static str {
    concat!(">>> YatSenOS v", env!("CARGO_PKG_VERSION"))
}

pub fn halt() {
    let disabled = !interrupts::are_enabled();
    interrupts::enable_and_hlt();
    if disabled {
        interrupts::disable();
    }
}
