#![no_std]

pub mod dsmr;
pub mod wifi;

macro_rules! mk_static {
    ($t:ty) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        STATIC_CELL.uninit()
    }};
    ($t:ty,$val:expr) => {{ mk_static!($t).write($val) }};
}

pub(crate) use mk_static;

pub const DSMR_BUFFER_SIZE: usize = 4096;
