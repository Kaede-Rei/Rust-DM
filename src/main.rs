#![no_std]
#![no_main]

use cortex_m::prelude::_embedded_hal_blocking_delay_DelayMs;
use cortex_m_rt::entry;
use panic_halt as _;

use rtt_target::rprintln;

use code_02_stm32h7_dm::prelude::*;

#[entry]
fn main() -> ! {
    // 初始化板卡
    let mut delay = board::init_board();


    loop {
        delay.delay_ms(1000u32);
        rprintln!("主循环运行中...");
    }
}
