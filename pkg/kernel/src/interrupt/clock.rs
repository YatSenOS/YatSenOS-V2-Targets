use super::consts;
use crate::{memory::gdt, utils::Registers};
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

pub unsafe fn reg_idt(idt: &mut InterruptDescriptorTable) {
    idt[consts::Interrupts::IrqBase as usize + consts::Irq::Timer as usize]
        .set_handler_fn(clock_handler)
        .set_stack_index(gdt::CONTEXT_SWITCH_IST_INDEX);
}

pub extern "C" fn clock(mut regs: Registers, mut sf: InterruptStackFrame) {
    super::ack(consts::Interrupts::IrqBase as u8);
    crate::process::switch(&mut regs, &mut sf);
}

as_handler!(clock);
