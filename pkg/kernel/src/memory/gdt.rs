use lazy_static::lazy_static;
use x86_64::registers::segmentation::Segment;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};

lazy_static! {
    static ref GDT: (GlobalDescriptorTable, KernelSelectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let data_selector = gdt.add_entry(Descriptor::kernel_data_segment());
        (
            gdt,
            KernelSelectors {
                code_selector,
                data_selector,
            },
        )
    };
}

#[derive(Debug)]
pub struct KernelSelectors {
    pub code_selector: SegmentSelector,
    pub data_selector: SegmentSelector,
}

pub fn init() {
    use x86_64::instructions::segmentation::{CS, DS, ES, FS, GS, SS};
    use x86_64::PrivilegeLevel;

    GDT.0.load();
    unsafe {
        CS::set_reg(GDT.1.code_selector);
        DS::set_reg(GDT.1.data_selector);
        SS::set_reg(SegmentSelector::new(0, PrivilegeLevel::Ring0));
        ES::set_reg(SegmentSelector::new(0, PrivilegeLevel::Ring0));
        FS::set_reg(SegmentSelector::new(0, PrivilegeLevel::Ring0));
        GS::set_reg(SegmentSelector::new(0, PrivilegeLevel::Ring0));
    }

    info!("GDT Initialized.");
}

pub fn get_selector() -> &'static KernelSelectors {
    &GDT.1
}
