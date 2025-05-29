use alloc::sync::Arc;
use core::any::Any;
use log::info;
use pci_types::{BaseClass, EndpointHeader, SubClass};
use spin::RwLock;
use x86_64::structures::paging::{Page, PageTableFlags};
use x86_64::structures::paging::page::PageRange;
use x86_64::VirtAddr;
use crate::device::ide::IdeDrive;
use crate::{pci_bus, process_manager};
use crate::memory::{MemorySpace, PAGE_SIZE};
use crate::memory::vmm::VmaType;
use crate::storage::add_block_device;

const MASS_STORAGE_DEVICE: BaseClass = 0x01;
const SATA_CONTROLLER: SubClass = 0x06;


struct AhciController{
    hba_regs: HBARegister,
}
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct HBARegister{
     hostCapabilities: u32,
     globalHostControl: u32,
     interruptStatus: u32,
     portsImplemented: u32,
     version: u32,
     commandCompletionCoalescingControl: u32,
     commandCompletionCoalescingPorts: u32,
     enclosureManagementLocation: u32,
     enclosureManagementControlu: u32,
     extendedHostCapabilities: u32,
     biosHandoffControl: u32,
     reserved: [u8;116],
     vendorSpecific: [u8;96],
}

pub fn init(){
    info!("searching the bus for mass storage devices that use sata");
    let mut found_devices = pci_bus().search_by_class(MASS_STORAGE_DEVICE as BaseClass, SATA_CONTROLLER as SubClass);
    info!("habe die folgenden Geräte gefunden {:?}", found_devices.len());
    let mut device = found_devices.pop().unwrap();
    unsafe {
        let ahciController = Arc::new(AhciController::new(device));
        info!("der ahci controller hat die folgenden Felder: {:?}", ahciController.hba_regs);
    }


    /*for device in found_devices {
        let device_id = device.read().header().id(&pci_bus().config_space());
        info!("Found IDE controller [{}:{}]", device_id.0, device_id.1);

        let ide_controller = Arc::new(crate::device::ide::IdeController::new(device));
        crate::device::ide::IdeController::plugin(Arc::clone(&ide_controller));

        let found_drives = ide_controller.init_drives();
        for drive in found_drives.iter() {
            let block_device = Arc::new(IdeDrive::new(Arc::clone(&ide_controller), *drive));
            add_block_device("ata", block_device);
        }
    }*/
    //die GHCR sind in Section 3 der Spezifikation zu finden. ich weiß noch nicht, wie man bis dahin kommt
}

impl AhciController {
    unsafe fn new(device: &RwLock<EndpointHeader>) -> Self {
        let device_header = device.read();

        // bei base address register (bar5) stehen die wichtigen Daten für die pci capabilities, register, etc.
        let bar5 = device_header.bar(5,&pci_bus().config_space());
        // bei bar4 findet sich ein io port
        let bar4 = device_header.bar(4,&pci_bus().config_space());
        info!("bar with slot one has the following info: {:?}", bar5);
        let bar_io = bar4.unwrap().unwrap_io();
        let bar_mem = bar5.unwrap().unwrap_mem();
        info!("bar io is {:?} and bar mem is {:?}", bar_io, bar_mem);

        //map the memory where the control registers are located
        let address = bar_mem.0 as u64;
        let length = bar_mem.1 as u64;
        info!(
                "(Address: [0x{:x}], Length: [{} B])",
                address,
                length
            );

        // Map non-volatile memory range to kernel address space
        let start_page = Page::from_start_address(VirtAddr::new(address)).unwrap();
        process_manager()
            .read()
            .kernel_process()
            .expect("Failed to get kernel process")
            .virtual_address_space
            .map(
                PageRange {
                    start: start_page,
                    end: start_page + (length / PAGE_SIZE as u64),
                },
                MemorySpace::Kernel,
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                VmaType::DeviceMemory,
                "ahci",
            );



        let ahci_base_addr = bar_mem.0 as *mut u8;;
        let cap = ahci_base_addr as *mut u32;
        let ghc = ahci_base_addr.offset(4 as isize) as *mut u32;
        let is = ahci_base_addr.offset(8 as isize) as *mut u32;
        let pi = ahci_base_addr.offset(12 as isize) as *mut u32;
        let vs = ahci_base_addr.offset(16 as isize) as *mut u32;

        let cccc = ahci_base_addr.offset(20 as isize) as *mut u32;
        let cccp = ahci_base_addr.offset(24 as isize) as *mut u32;
        let eml = ahci_base_addr.offset(28 as isize) as *mut u32;
        let emc = ahci_base_addr.offset(32 as isize) as *mut u32;
        let ehc = ahci_base_addr.offset(36 as isize) as *mut u32;
        let bhc = ahci_base_addr.offset(40 as isize) as *mut u32;

        // hier komme ich auf komische Ergebnisse
        //let capabilities = cap.read();
        info!("cap = {:?}, ghc = {:?}, is = {:?}, pi = {:?}, vs = {:?}", cap.read(), ghc.read(), is.read(), pi.read(), vs.read());

        Self{ hba_regs: HBARegister {
            hostCapabilities: cap.read(),
            globalHostControl: ghc.read(),
            interruptStatus: is.read(),
            portsImplemented: pi.read(),
            version: vs.read(),
            commandCompletionCoalescingControl: cccc.read(),
            commandCompletionCoalescingPorts: cccp.read(),
            enclosureManagementLocation: eml.read(),
            enclosureManagementControlu: emc.read(),
            extendedHostCapabilities: ehc.read(),
            biosHandoffControl: bhc.read(),
            reserved: [0;116],
            vendorSpecific: [0;96],
        } }
    }
}





