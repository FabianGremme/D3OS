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

enum BiosHandoffFlags {
    BIOS_OWNED_SEMAPHORE = 1 << 0,
    OS_OWNED_SEMAPHORE = 1 << 1,
    SMI_ON_OWNERSHIP_CHANGE = 1 << 2,
    OS_OWNERSHIP_CHANGE = 1 << 3,
    BIOS_BUSY = 1 << 4
}


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
     signature: u32,
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
/*
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct HbaCommandHeader {
    // DWORD 0
    uint8_t commandFisLength: 5;
    uint8_t atapi: 1;
    uint8_t write: 1;
    uint8_t prefetchable: 1;

    uint8_t reset: 1;
    uint8_t bist: 1;
    uint8_t clearBusyOnOK: 1;
    uint8_t reserved1: 1;
    uint8_t portMultiplierPort: 4;

     physicalRegionDescriptorTableLength: u16,

    // DWORD 1
    physicalRegionDescriptorByteCount: u32,

    // DWORD 2-3
    commandTableDescriptorBaseAddress: u32,
    commandTableDescriptorBaseAddressUpper: u32,

    // DWORD 4-7
    reserved: [u32;4],
}*/


#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct HbaCommandHeader {
    // DWORD 0
    cmd_ctrl: u8,
    cmd_ctrl2: u8,
    physicalRegionDescriptorTableLength: u16,

    // DWORD 1
    physicalRegionDescriptorByteCount: u32,

    // DWORD 2-3
    commandTableDescriptorBaseAddress: u32,
    commandTableDescriptorBaseAddressUpper: u32,

    // DWORD 4-7
    reserved: [u32;4],
}


pub fn init(){
    info!("searching the bus for mass storage devices that use sata");
    let mut found_devices = pci_bus().search_by_class(MASS_STORAGE_DEVICE as BaseClass, SATA_CONTROLLER as SubClass);
    info!("habe die folgenden Geräte gefunden {:?}", found_devices.len());
    let mut device = found_devices.pop().unwrap();
    unsafe {
        let mut ahci_controller = Arc::new(AhciController::new(device));
        info!("der ahci controller hat die hba: {:?}", ahci_controller.hba_regs);
        ahci_controller.check_bios_handoff();
        ahci_controller.check_ports_for_device();
        ahci_controller.check_ahci_mode_enabled();
        ahci_controller.check_only_ahci();
        ahci_controller.check_64_bit_addr_supported();
        ahci_controller.check_cap_nr_of_ports();
        ahci_controller.check_nr_of_command_slots();
        ahci_controller.map_command_components();

        //info!("teste die Funktion um mehrere Bitfelder auszulesen");
        //let testoutput = ahci_controller.general_bitlen_reader(57105, 7, 5); // hier sollte 30 rauskommen, das passt
        //info!("testoutput ist {}", testoutput);

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
            signature: sig.read(),
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

    unsafe fn fill_cmd_header(start: *mut u8) ->HbaCommandHeader{
        let dword0 = start as *mut u32;
        let mut offset = 4;
        let dword1 = start.offset(offset) as *mut u32;
        offset += 4;
        let dword2 = start.offset(offset) as *mut u32;
        offset += 4;
        let dword3 = start.offset(offset) as *mut u32;
        offset += 4;
        let dword4 = start.offset(offset) as *mut u32;
        offset += 4;
        let dword5 = start.offset(offset) as *mut u32;
        offset += 4;
        let dword6 = start.offset(offset) as *mut u32;
        offset += 4;
        let dword7 = start.offset(offset) as *mut u32;

        info!("dword0 = {:?}, dword1 = {:?}, dword2 = {:?}, dword3 = {:?}, dword4 = {:?}, dword5 = {:?}, dword6 = {:?}, dword7 = {:?}"
                ,dword0.read(), dword1.read(), dword2.read(), dword3.read(), dword4.read(), dword5.read(), dword6.read(), dword7.read());
        let phys_table_len = (dword0.read() >> 16) as u16;
        let cmd = dword0.read() as u16;
        let cmd1 = (cmd >> 8) as u8;
        let cmd2 = dword0.read()  as u8;

        HbaCommandHeader{
            // DWORD 0
            cmd_ctrl: cmd2,
            cmd_ctrl2: cmd1,
            physicalRegionDescriptorTableLength: phys_table_len,

            // DWORD 1
            physicalRegionDescriptorByteCount: dword1.read(),

            // DWORD 2-3
            commandTableDescriptorBaseAddress: dword2.read(),
            commandTableDescriptorBaseAddressUpper: dword3.read(),

            // DWORD 4-7
            reserved: [dword4.read(), dword5.read(), dword6.read(), dword7.read()],
        }
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
        Self::map_general(bar_mem.0 as u64, bar_mem.1 as u64, "ahci");
        let hba = Self::fill_hba_reg(ahci_base_addr);

        Self{
            hba_regs: hba,
            ports:Self::init_ports(ahci_base_addr,hba.portsImplemented)
        }



    }


    //length is in bytes
    pub unsafe fn map_general(address: u64, length: u64, tag: &str){
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
                tag,
            );
    }

    pub fn general_bit_check(register: u32, bit_position: u8)->bool{
        let mask = 1<<bit_position;
        return register & mask != 0;
    }

    pub fn general_bitlen_reader(register: u32, bit_position: u8, len: u8)-> u32{
        let mut mask = 1<<bit_position;
        for i in 0..len{
            mask = mask | 1<<(bit_position + i)
        }
        return (register & mask)>>bit_position;
    }

    pub fn check_ports_for_device(& self){
        for current_port in &self.ports{
            if Self::check_port_usable(current_port.clone()){
                let signature = current_port.signature;
                info!("the device signature is {}", signature);
            }

        }
    }

    pub fn check_port_usable(port:HbaPort)-> bool{
        let ssts = port.sataStatus;
        let ipm = (ssts >> 8) & 0x0F;
        let det = ssts & 0x0F;

        if ipm != 0x01 {    //0x01 means that the interface of the device is active. only then the device can be accessed
            //info!("ERR: interface is not active");
            return false;
        }
        if det != 0x03 {    //0x03 means that the device is detected and a physical communication is established
            //info!("ERR: device is not detected, or physical communication not established");
            return false;
        }
        true
    }

    pub fn check_ahci_mode_enabled(&self){
        let ghc = self.hba_regs.globalHostControl;
        let output = Self::general_bit_check(ghc, 31);
        if output{
            info!("der Controller läuft im ahci modus");
        }else{
            info!("der Controller läuft nicht im ahci modus");
        }
    }

    pub fn check_only_ahci(&self){
        let sam = self.hba_regs.hostCapabilities;
        let output = Self::general_bit_check(sam, 18);
        if output{
            info!("der Controller unterstützt nur ahci");
        }else{
            info!("der Controller unterstützt nicht nur ahci");
        }
    }

    pub fn check_bios_handoff(&self){
        //check if the version is high enough
        if self.hba_regs.version >= 0x10200{
            info!("Version ist hoch genug");
            let ext_cap = self.hba_regs.extendedHostCapabilities;
            info!("ext_cap sind {}", ext_cap);
            if ext_cap & 1 != 0{
                info!("BIOS Handoff wird vom Controller unterstützt")
            }
        }else{
            info!("Version ist nicht hoch genug")
        }
        let handoff = self.hba_regs.biosHandoffControl;
        if handoff == 0{
            info!("the bios has no control over the hba, so the os can use it");
        }
    }

    pub fn check_64_bit_addr_supported(&self){
        let cap = self.hba_regs.hostCapabilities;
        let output = Self::general_bit_check(cap, 31);
        if output{
            info!("es werden 64 bit adressen unterstützt");
        }else{
            info!("es werden 32 bit adressen unterstützt");
        }

    }

    pub fn check_cap_nr_of_ports(&self){
        let cap = self.hba_regs.hostCapabilities;
        let nr_of_ports = Self::general_bitlen_reader(cap, 0, 5);
        info!("laut capabilities werden {} Ports unterstützt.", nr_of_ports);
    }

    pub fn check_nr_of_command_slots(&self){
        let cap = self.hba_regs.hostCapabilities;
        let nr_of_cmds = Self::general_bitlen_reader(cap, 8, 5);
        info!("laut capabilities werden {} Command slots unterstützt.", nr_of_cmds);
    }

    pub fn map_command_components(&self){
        //der Port muss noch zurückgesetzt werden, aber das kommt noch
        for port in &self.ports{
            if Self::check_port_usable(port.clone()){
                self.map_command_for_port(*port);
            }

        }

    }

    pub fn map_command_for_port(&self, port: HbaPort){
        //baue die Adresse für die 32 cmd header
        let cmd_header_addr:u64 = port.commandListBaseAddress as u64 | ((port.commandListBaseAddressUpper as u64) << 32);
        let size_cmd_header = 1024;

        //baue die Adresse für die received FIS
        let received_fis: u64 = port.fisBaseAddress as u64 | ((port.fisBaseAddressUpper as u64) << 32);
        let size_received_fis = 256;
        info!("die Addressen sind: cmd_header: {:x}, received_fis: {:x}", cmd_header_addr, received_fis);
        unsafe {
            Self::map_general(cmd_header_addr, size_cmd_header, "cmd_hd");
            Self::map_general(received_fis, size_received_fis, "rc_fis");


            //test if there is actual memory
            let cmd_header1 = Self::fill_cmd_header(cmd_header_addr as *mut u8);
            let cmd_header2 = Self::fill_cmd_header((cmd_header_addr + 8*32) as *mut u8);
            let cmd_header3 = Self::fill_cmd_header((cmd_header_addr + 16*32) as *mut u8);


        }

    }


}

// Todo:
//Erkennung der verschiedenen Geräte (ata und atapi) (Signaturen müssen nur noch gematched werden)
//Comand Liste anschauen (es werden 31 command slots unterstützt) (vector of command headers?)
//Reset vom Port impl
//command Table als structur festlegen und einmappen

//tock registers (anschauen)

//alles mal in ein ganz frisches neues D3OS reinkopieren (übers Wochenende fertig)




