#![no_std]
#![no_main]

use cortex_m_rt::entry;
use panic_halt as _;

use rtt_target::{rprintln, rtt_init_print};

use code_02_stm32h7_dm::prelude::*;

#[entry]
fn main() -> ! {
    rtt_init_print!();

    // 初始化板卡
    board::init_board();

    rgb_ws2812::blue();
    rprintln!("蓝色");

    loop {
        
    }
}
