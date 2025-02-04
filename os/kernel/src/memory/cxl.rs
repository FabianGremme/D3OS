use alloc::borrow::ToOwned;
use alloc::vec::Vec;
use core::ptr;
use acpi::{AcpiTable};
use acpi::sdt::{SdtHeader, Signature};
use log::info;
use crate::memory::PAGE_SIZE;
use x86_64::structures::paging::{Page, PageTableFlags};
use x86_64::structures::paging::page::PageRange;
use x86_64::VirtAddr;
use crate::{acpi_tables, pci_bus, process_manager};
use crate::device::pci::PciBus;
//use crate::memory::MemorySpace;
//use crate::memory::nvmem::NfitStructureHeader;

pub fn print_bus_devices(){
    pci_bus().dump_devices();
}

pub fn print_bus_devices_status(){
    pci_bus().dump_devices_status_registers();
}

pub fn print_bus_devices_command(){
    pci_bus().dump_devices_command_registers();
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct CEDT {
    header: SdtHeader,

}

#[allow(dead_code)]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum CEDTStructureType {
    CXLHostBridgeStructure = 0,
    CXLFixedMemoryWindowStructure = 1,
    CXLXORInterleaveMathStructure = 2,
    RCECDownstreamPortAssociationStructure = 3,
    CXLSystemDescriptionStructure = 4,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct CEDTStructureHeader {
    typ: CEDTStructureType,
    reserved_1: u8,
    record_length: u16,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct CXLHostBridgeStructure{
    header: CEDTStructureHeader,
    uid: u32,
    cxl_version: u32,
    reserved_2: u32,
    base: u64,
    length: u64,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct CXLFixedMemoryWindowStructure{
    header: CEDTStructureHeader,
    reserved_2: u32,
    base_hpa: u64,
    window_size: u64,
    encoded_nr_of_interleave_ways: u8,
    interleave_arithmetic: u8,
    reserved_3: u16,
    host_bridge_interleave_granularity: u64,
    window_restrictions: u16,
    qtg_id: u16,
    interleave_target_list: [u32; 2], //hier ist die groesse 4* Anzahl encodet interleave ways
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct CXLXORInterleaveMathStructure{
    header: CEDTStructureHeader,
    reserved_2: u16,
    nr_of_bitmap_entries: u8,
    xormap_list: u128, // hier muss 8*Anzahl vor nr_of_bitmap_entries
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct RCECDownstreamPortAssociationStructure{
    header: CEDTStructureHeader,
    rcec_segment_nr: u16,
    rcec_bdf: u16,
    protocol_type: u16,
    base_addr: u64,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct CXLSystemDescriptionStructure{
    header: CEDTStructureHeader,
    system_capabilities: u16,
    reserved_2: u16,
}

unsafe impl AcpiTable for CEDT {
    const SIGNATURE: Signature = Signature::CEDT;

    fn header(&self) -> &SdtHeader {
        &self.header
    }
}


impl CEDT {
    pub fn get_structures(&self) -> Vec<&CEDTStructureHeader> {
        let mut tables = Vec::<&CEDTStructureHeader>::new();

        //let help = self.header.length;
        //let help2 = size_of::<CEDT>();
        let mut remaining = self.header.length as usize - size_of::<CEDT>();
        let mut structure_ptr = unsafe { ptr::from_ref(self).add(1) } as *const CEDTStructureHeader;

        while remaining > 0 {
            unsafe {
                let structure = *structure_ptr;
                tables.push(structure_ptr.as_ref().expect("Invalid CEDT structure"));
                info!("gefundene Structure is {:?}", structure);

                structure_ptr = (structure_ptr as *const u8).add(structure.record_length as usize) as *const CEDTStructureHeader;
                info!("remaining = {:?} und recordlen = {:?}", remaining, structure.record_length as usize);
                remaining = remaining - structure.record_length as usize;
            }
            info!("Found CEDT Structure");
        }

        return tables;
    }
}

impl CEDTStructureHeader {
    pub fn as_structure<T>(&self) -> &T {
        unsafe {
            ptr::from_ref(self).cast::<T>().as_ref().expect("Invalid CEDT structure")
        }
    }
}


pub fn init() {
    if let Ok(cedt) = acpi_tables().lock().find_table::<CEDT>() {
        info!("Found CEDT table");
        let structures = cedt.get_structures();
        for structure in structures{
            if structure.typ == CEDTStructureType::CXLHostBridgeStructure{
                let current: &CXLHostBridgeStructure = structure.as_structure();
                info!("Host Bridge ist {:?}", current);
                info!("Host Bridge hat die folgenden Root Ports:");
                PciBus::scan_by_nr(current.uid as u8);
            }else if structure.typ == CEDTStructureType::CXLFixedMemoryWindowStructure{
                let current: &CXLFixedMemoryWindowStructure = structure.as_structure();
                info!("Memory Window ist ist {:?}", current);
            }else{
                info!("found different structure");
            }
        }
    }
}