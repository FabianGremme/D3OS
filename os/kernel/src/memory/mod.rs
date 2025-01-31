pub mod alloc;
pub mod physical;
pub mod r#virtual;
pub mod nvmem;

pub mod cxl;
mod messages;
mod test_capabilities;
mod arbmux;

#[derive(Clone, Copy)]
pub enum MemorySpace {
    Kernel,
    User
}

pub const PAGE_SIZE: usize = 0x1000;
