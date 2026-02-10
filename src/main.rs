#![no_std]
#![no_main]

use cortex_m_rt::entry;
use panic_halt as _;

use hal::prelude::*;
use stm32h7xx_hal as hal;

#[entry]
fn main() -> ! {
    loop {}
}
