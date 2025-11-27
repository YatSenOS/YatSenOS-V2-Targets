mod apic;
mod clock;
mod consts;
mod exception;
mod serial;
mod syscall;

use apic::*;
pub use syscall::SyscallArgs;
use x86_64::structures::idt::InterruptDescriptorTable;

use crate::memory::physical_to_virtual;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        unsafe {
            exception::reg_idt(&mut idt);
            serial::reg_idt(&mut idt);
            clock::reg_idt(&mut idt);
            syscall::reg_idt(&mut idt);
        }
        idt
    };
}

/// init interrupts system
pub fn init() {
    IDT.load();
    debug!("XApic support = {}.", apic::XApic::support());
    let mut lapic = unsafe { XApic::new(physical_to_virtual(LAPIC_ADDR)) };
    lapic.cpu_init();
    serial::init();

    info!("Interrupts Initialized.");
}

#[inline(always)]
pub fn enable_irq(irq: u8, cpuid: u8) {
    let mut ioapic = unsafe { IoApic::new(physical_to_virtual(IOAPIC_ADDR)) };
    ioapic.enable(irq, cpuid);
}

#[inline(always)]
pub fn ack(_irq: u8) {
    let mut lapic = unsafe { XApic::new(physical_to_virtual(LAPIC_ADDR)) };
    lapic.eoi();
}
