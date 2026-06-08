#![no_std]

use crate::{dsmr::DSMRReading, zaptec::ZaptecSettings};

pub mod dsmr;
pub mod error;
pub mod led;
pub mod wifi;
pub mod zaptec;

macro_rules! mk_static {
    ($t:ty) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        STATIC_CELL.uninit()
    }};
    ($t:ty,$val:expr) => {{ mk_static!($t).write($val) }};
}
use esp_metadata_generated::memory_range;
pub(crate) use mk_static;

pub const DSMR_BUFFER_SIZE: usize = 4096;
// pub const HEAP_SIZE: usize = 1024 * 64;
pub const HEAP_SIZE: usize =
    memory_range!("DRAM2_UNINIT").end - memory_range!("DRAM2_UNINIT").start;

pub fn calc_voltage(dsmr_reading: DSMRReading) -> ZaptecSettings {
    let leftover_wattage = dsmr_reading.power_received - dsmr_reading.power_delivered;

    ZaptecSettings {
        charging_power: leftover_wattage,
    }
}
