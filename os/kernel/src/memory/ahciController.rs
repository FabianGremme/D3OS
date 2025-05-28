use alloc::sync::Arc;
use core::any::Any;
use log::info;
use pci_types::{BaseClass, EndpointHeader, SubClass};
use spin::RwLock;
use crate::device::ide::IdeDrive;
use crate::pci_bus;
use crate::storage::add_block_device;

const MASS_STORAGE_DEVICE: BaseClass = 0x01;
const SATA_CONTROLLER: SubClass = 0x06;


struct AhciController{

}

pub fn init(){
    info!("searching the bus for mass storage devices that use sata");
    let mut found_devices = pci_bus().search_by_class(MASS_STORAGE_DEVICE as BaseClass, SATA_CONTROLLER as SubClass);
    info!("habe die folgenden Geräte gefunden {:?}", found_devices.len());
    let mut device = found_devices.pop().unwrap();
    let device_header = device.read();
    let id = device.read().header().id(&pci_bus().config_space());
    let ahci_controller = Arc::new(AhciController::new(device));


    // bei base address register (bar5) stehen die wichtigen Daten für die pci capabilities, register, etc.
    let bar5 = device_header.bar(5,&pci_bus().config_space());
    // bei bar4 findet sich ein io port
    let bar4 = device_header.bar(4,&pci_bus().config_space());
    info!("bar with slot one has the following info: {:?}", bar5);
    let bar_io = bar4.unwrap().unwrap_io();
    let bar_mem = bar5.unwrap().unwrap_mem();

    info!("bar io is {:?} and bar mem is {:?}", bar_io, bar_mem);

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
    // chatgpt schlägt vor an der BAR 5 Adresse des Enpoint headers weiter nachzuschauen.(muss noch überprüft werden)
}

impl AhciController {
    fn new(device: &RwLock<EndpointHeader>) -> Self {
        Self{}
    }
}





