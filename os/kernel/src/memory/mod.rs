pub mod vmm;
pub mod pages;
pub mod frames;

pub mod nvmem;

pub mod kheap;
pub mod kstack;
pub mod acpi_handler;

pub mod cxl;
mod messages;
mod test_capabilities;
mod arbmux;
pub(crate) mod srat;

#[derive(Clone, Copy)]
pub enum MemorySpace {
    Kernel,
    User
}

pub const PAGE_SIZE: usize = 0x1000;
