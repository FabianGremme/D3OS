use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
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
use crate::memory::nvmem::NfitStructureHeader;
use crate::memory::vmm::VmaType;
use crate::storage::add_block_device;

const MASS_STORAGE_DEVICE: BaseClass = 0x01;
const SATA_CONTROLLER: SubClass = 0x06;


struct AhciController{
    hba_regs: HBARegister,
    ports: Vec<HbaPort>,
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
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct HbaPort {
     commandListBaseAddress: u32,
     commandListBaseAddressUpper: u32,
     fisBaseAddress: u32,
     fisBaseAddressUpper: u32,
     interruptStatus: u32,
     interruptEnable: u32,
     command: u32,
     reserved1: u32,
     taskFileData: u32,
     DeviceSignature_signature: u32,
     sataStatus: u32,
     sataControl: u32,
     sataError: u32,
     sataActive: u32,
     commandIssue: u32,
     sataNotification: u32,
     fisBasedSwitchControl: u32,
     deviceSleep: u32,
     reserved2: [u32;10],
     vendorSpecific: [u32;4],

}

pub fn init(){
    info!("searching the bus for mass storage devices that use sata");
    let mut found_devices = pci_bus().search_by_class(MASS_STORAGE_DEVICE as BaseClass, SATA_CONTROLLER as SubClass);
    info!("habe die folgenden Geräte gefunden {:?}", found_devices.len());
    let mut device = found_devices.pop().unwrap();
    unsafe {
        let mut ahci_controller = Arc::new(AhciController::new(device));
        info!("der ahci controller hat die hba: {:?}", ahci_controller.hba_regs);
        ahci_controller.check_ports_for_device();
    }



    //die GHCR sind in Section 3 der Spezifikation zu finden. ich weiß noch nicht, wie man bis dahin kommt
}

impl AhciController {

    unsafe fn fill_hba_reg(ahci_base_addr: *mut u8) -> HBARegister{
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
        HBARegister {
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
        }
    }

    unsafe fn init_ports(ahci_base_addr: *mut u8, hba_ports: u32) ->Vec<HbaPort>{
        //aus der hba ports variable muss erst mal die Anzahl der Ports bestimmt werden. Dazu muss die Anzahl der 1 in der Binaerform gezaehlt werden.
        let mut port_nr = 0;
        let mut calc = hba_ports;
        while calc != 0{
            calc = calc & (calc -1);
            port_nr += 1;
        }
        info!("port anzahl = {:?}", port_nr);
        let mut output : Vec<HbaPort> = Vec::<HbaPort>::new();
        for i in 0..port_nr{
            output.push(Self::fill_port(ahci_base_addr,i));
        }
        output
    }

    unsafe fn fill_port(ahci_base_addr: *mut u8, nr_of_port: u64) -> HbaPort{
        let mut port_offset = (256 + (nr_of_port * 128))  as isize;
        let clb = ahci_base_addr.offset(port_offset) as *mut u32;
        port_offset += 4;
        let clbu = ahci_base_addr.offset(port_offset) as *mut u32;
        port_offset += 4;
        let fis_ba = ahci_base_addr.offset(port_offset) as *mut u32;
        port_offset += 4;
        let fis_bau = ahci_base_addr.offset(port_offset) as *mut u32;
        port_offset += 4;
        let istat = ahci_base_addr.offset(port_offset) as *mut u32;
        port_offset += 4;
        let ie = ahci_base_addr.offset(port_offset) as *mut u32;
        port_offset += 4;
        let cmd = ahci_base_addr.offset(port_offset) as *mut u32;
        port_offset += 4;
        let res1 = ahci_base_addr.offset(port_offset) as *mut u32;
        port_offset += 4;
        let tfd = ahci_base_addr.offset(port_offset) as *mut u32;
        port_offset += 4;
        let sig = ahci_base_addr.offset(port_offset) as *mut u32;
        port_offset += 4;
        let sata_stat = ahci_base_addr.offset(port_offset) as *mut u32;
        port_offset += 4;
        let sata_ctrl = ahci_base_addr.offset(port_offset) as *mut u32;
        port_offset += 4;
        let sata_err = ahci_base_addr.offset(port_offset) as *mut u32;
        port_offset += 4;
        let sata_act = ahci_base_addr.offset(port_offset) as *mut u32;
        port_offset += 4;
        let cmd_issue = ahci_base_addr.offset(port_offset) as *mut u32;
        port_offset += 4;
        let sata_not = ahci_base_addr.offset(port_offset) as *mut u32;
        port_offset += 4;
        let fis_bsc = ahci_base_addr.offset(port_offset) as *mut u32;
        port_offset += 4;
        let dev_sleep = ahci_base_addr.offset(port_offset) as *mut u32;
        port_offset += 4;
        let output = HbaPort{
            commandListBaseAddress: clb.read(),
            commandListBaseAddressUpper: clbu.read(),
            fisBaseAddress: fis_ba.read(),
            fisBaseAddressUpper: fis_bau.read(),
            interruptStatus: istat.read(),
            interruptEnable: ie.read(),
            command: cmd.read(),
            reserved1: res1.read(),
            taskFileData: tfd.read(),
            DeviceSignature_signature: sig.read(),
            sataStatus: sata_stat.read(),
            sataControl: sata_ctrl.read(),
            sataError: sata_err.read(),
            sataActive: sata_act.read(),
            commandIssue: cmd_issue.read(),
            sataNotification: sata_not.read(),
            fisBasedSwitchControl: fis_bsc.read(),
            deviceSleep: dev_sleep.read(),
            reserved2: [0;10],
            vendorSpecific: [0;4],
        };
        info!("bearbeite Port Nr {:?} mit den Feldern {:?}", nr_of_port, output);
        output
    }

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

        let ahci_base_addr = bar_mem.0 as *mut u8;

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
        let hba = Self::fill_hba_reg(ahci_base_addr);

        Self{
            hba_regs: hba,
            ports:Self::init_ports(ahci_base_addr,hba.portsImplemented)
        }

        // Todo:
        //bios Handoff implementieren
        // schauen, ob der ahci modus aktiviert ist


    }

    pub fn check_ports_for_device(& self){
        for current_port in &self.ports{
            let ssts = current_port.sataStatus;
            info!("der Port hat den Status {:?}", ssts);
        }
    }
}





