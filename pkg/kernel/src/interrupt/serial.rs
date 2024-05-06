use super::consts;
use crate::drivers::input::push_key;
use crate::drivers::serial::get_serial_for_sure;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

pub unsafe fn reg_idt(idt: &mut InterruptDescriptorTable) {
    idt[consts::Interrupts::IrqBase as u8 + consts::Irq::Serial0 as u8]
        .set_handler_fn(interrupt_handler);
}

pub fn init() {
    super::enable_irq(consts::Irq::Serial0 as u8, 0);
    debug!("Serial0(COM1) IRQ enabled.");
}

/// Receive character from uart 16550
/// Should be called on every interrupt
pub fn receive() {
    let data = get_serial_for_sure().receive();

    if let Some(data) = data {
        push_key(data);
    }
}

pub extern "x86-interrupt" fn interrupt_handler(_st: InterruptStackFrame) {
    super::ack(super::consts::Irq::Serial0 as u8);
    receive();
}
