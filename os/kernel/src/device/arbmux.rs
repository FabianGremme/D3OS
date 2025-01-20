use alloc::vec;
use alloc::vec::Vec;
use log::info;
use pci_types::EndpointHeader;
use spin::RwLock;
use crate::device::arbmux::vlsm_state::{Active, L1_1, L1_2, L1_3, L1_4, Reset, SleepL2};

#[derive(PartialEq, Eq, Clone)]
enum vlsm_state{
    Reset,
    Active,
    L1_1,
    L1_2,
    L1_3,
    L1_4,
    DAPM,
    SleepL2,
    LinkReset,
    LinkError,
    LinkDisable,
    Retrain,
}

pub struct vlsm{
    id:u8,          //nicht bekannt, ob in der arb/mux das über ids läuft. die Links müssen halt irgendwie voneinander getrennt werden
    current_state:vlsm_state,
}

pub struct arb_mux{
    connected_links:RwLock<Vec<vlsm>>,
    nr_connected_links:u16,
    combined_state_to_send:vlsm_state,
}


impl arb_mux {
    //Funktion, welche den combined state anhand der Tabelle berechnet
    fn state_calculator(A:vlsm_state, B: &vlsm) -> vlsm_state{
        if A == Reset{
            return B.current_state.clone();
        }else if B.current_state == Reset {
            return A;
        }else if A == Active || B.current_state == Active {
            return Active
        }else if A == L1_1 || B.current_state == L1_1{
            return L1_1
        }else if A == L1_2 || B.current_state == L1_2{
            return L1_2
        }else if A == L1_3 || B.current_state == L1_3{
            return L1_3
        }else if A == L1_4 || B.current_state == L1_4{
            return L1_4
        }else if A == SleepL2 || B.current_state == SleepL2{
            return SleepL2
        }else{
            info!("state type not found");
                return Reset
        }
    }

    //alle einzelnen Links werden nach dem oben gegebenen Muster mit dem Gesamtstate geprüft. Erst danach darf gesendet werden.
    fn calc_combined_state_to_send(&mut self){
        self.connected_links.read().iter().for_each(|elem|{
          self.combined_state_to_send = Self::state_calculator(self.combined_state_to_send.clone(),elem);
        });
    }





}
//Funktion, welche überprüft, welche Wechsel erlaubt ist
impl vlsm{
    // hier müssen die Messages angebunden werden. Vielleicht noch anders impl
    pub fn try_state_change (new_state:vlsm_state) -> bool{
        return false;
    }
}