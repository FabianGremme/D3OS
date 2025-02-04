use alloc::vec::Vec;
use core::ptr;
use acpi::sdt::{SdtHeader, Signature};
use log::info;
use crate::acpi_tables;
use acpi::{AcpiTable};
use crate::device::pci::PciBus;
use crate::memory::cxl::{CEDT, CEDTStructureHeader, CXLFixedMemoryWindowStructure, CXLHostBridgeStructure};

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct SRAT {
    header: SdtHeader,

}
#[allow(dead_code)]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum SratStructureType {
    ProcessorLocalApicAffinityStructure = 0,
    MemoryAffinityStructure = 1,
    ProcessorLocalX2apicAffinityStructure = 2,
    GiccAffinityStructure = 3,
    ArchitectureSpecificAffinityStructure = 4,
    GenericInitiatorAffinityStructure = 5,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct SratStructureHeader {
    typ: SratStructureType,
    typ_2: u8,
    length: u16,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
//srat steht für static resource affinity table und zeigt alle ressourcen an, die das system kennt
pub struct SratFormat {
    //laut spezifikation ist da ein header, aber osdev hat diesen nicht. nur zur Info
    signature: u32,
    length: u32,
    revision: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: u64,
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
    reserved_1: u32,
    reserved_2: u64,
    //srat_structures: muss ich noch genauer schauen, wie das läuft
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
//srat steht für static resource affinity table und zeigt alle ressourcen an, die das system kennt
pub struct ProcessorLocalApicAffinityStructure {
    header: SratStructureHeader,
    proximility_domain: u8,
    apic_id: u8,
    flags: u32,
    local_sapic_eid: u8,
    proximility_domain_2: [u8; 3],
    clock_domain: u32,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
//srat steht für static resource affinity table und zeigt alle ressourcen an, die das system kennt
pub struct MemoryAffinityStructure {
    header: SratStructureHeader,
    proximility_domain: u32,
    reserved: u16,
    base_addr_low: u32,
    base_addr_high: u32,
    length_low: u32,
    length_high: u32,
    reserved_2: u32,
    flags: u32,
    reserved_3: u64,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
//srat steht für static resource affinity table und zeigt alle ressourcen an, die das system kennt
pub struct ProcessorLocalX2apicAffinityStructure {
    header: SratStructureHeader,
    reserved: u16,
    proximity_domain: u32,
    x2apic_id: u32,
    flags: u32,
    clock_domain: u32,
    reserved_2: u32,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
//srat steht für static resource affinity table und zeigt alle ressourcen an, die das system kennt
pub struct GiccAffinityStructure {
    header: SratStructureHeader,
    proximity_domain: u32,
    acpi_processor_uid: u32,
    flags: u32,
    clock_domain: u32,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
//srat steht für static resource affinity table und zeigt alle ressourcen an, die das system kennt
pub struct ArchitectureSpecificAffinityStructure {
    header: SratStructureHeader,
    proximity_domain: u32,
    reserved: u16,
    its_id: u32,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
//srat steht für static resource affinity table und zeigt alle ressourcen an, die das system kennt
pub struct GenericInitiatorAffinityStructure {
    header: SratStructureHeader,
    reserved: u8,
    device_handle_type: u8,
    proximity_domain: u32,
    device_handle: u128,
    flags: u32,
    reserved_2:u32,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
//srat steht für static resource affinity table und zeigt alle ressourcen an, die das system kennt
pub struct DeviceHandleAcpi {
    acpi_hid: u64,
    acpi_uid: u32,
    reserved: u32,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
//srat steht für static resource affinity table und zeigt alle ressourcen an, die das system kennt
pub struct DeviceHandlePci {
    pci_segment: u16,
    pci_bdf_nr: u16,
    reserved: [u8; 12],
}

unsafe impl AcpiTable for SRAT {
    const SIGNATURE: Signature = Signature::CEDT;

    fn header(&self) -> &SdtHeader {
        &self.header
    }
}

impl SRAT {
    pub fn get_structures(&self) -> Vec<&SratStructureHeader> {
        let mut tables = Vec::<&SratStructureHeader>::new();

        let mut remaining = self.header.length as usize - size_of::<SRAT>();
        let mut structure_ptr = unsafe { ptr::from_ref(self).add(1) } as *const SratStructureHeader;

        while remaining > 0 {
            unsafe {
                let structure = *structure_ptr;
                tables.push(structure_ptr.as_ref().expect("Invalid Srat structure"));
                info!("gefundene Structure is {:?}", structure);

                structure_ptr = (structure_ptr as *const u8).add(structure.length as usize) as *const SratStructureHeader;
                info!("remaining = {:?} und recordlen = {:?}", remaining, structure.length as usize);
                remaining = remaining - structure.length as usize;
            }
            info!("Found Srat Structure");
        }

        return tables;
    }
}

impl SratStructureHeader {
    pub fn as_structure<T>(&self) -> &T {
        unsafe {
            ptr::from_ref(self).cast::<T>().as_ref().expect("Invalid Srat structure")
        }
    }
}


pub fn init() {
    if let Ok(srat) = acpi_tables().lock().find_table::<SRAT>() {
        info!("Found SRAT table");
        let structures = srat.get_structures();
        for structure in structures{
            if structure.typ == SratStructureType::ProcessorLocalApicAffinityStructure{
                let current: &ProcessorLocalApicAffinityStructure = structure.as_structure();
                info!("ProcessorLocalApicAffinityStructure ist {:?}", current);
            }else if structure.typ == SratStructureType::MemoryAffinityStructure{
                let current: &MemoryAffinityStructure = structure.as_structure();
                info!("MemoryAffinityStructure ist {:?}", current);
            }else if structure.typ == SratStructureType::ProcessorLocalX2apicAffinityStructure {
                let current: &ProcessorLocalX2apicAffinityStructure = structure.as_structure();
                info!("ProcessorLocalX2apicAffinityStructure ist {:?}", current);
            }else if structure.typ == SratStructureType::GiccAffinityStructure {
                let current: &GiccAffinityStructure = structure.as_structure();
                info!("GiccAffinityStructure ist {:?}", current);
            }else if structure.typ == SratStructureType::ArchitectureSpecificAffinityStructure {
                let current: &ArchitectureSpecificAffinityStructure = structure.as_structure();
                info!("ArchitectureSpecificAffinityStructure ist {:?}", current);
            }else if structure.typ == SratStructureType::GenericInitiatorAffinityStructure {
                let current: &GenericInitiatorAffinityStructure = structure.as_structure();
                info!("GenericInitiatorAffinityStructure ist {:?}", current);
            }else{
                info!("unknown structure");
            }
        }
    }
}