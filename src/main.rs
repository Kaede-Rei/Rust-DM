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

    // motor_dm::set_spd(0x01, 1.57);
    // motor_dm::set_pos_spd(0x01, -3.1, 1.57);
    motor_dm::set_pos_spd_cur(0x01, 5.07, 1.57, 1.0);

    let mut pos = 0.0;
    let mut spd = 0.0;
    let mut torque = 0.0;

    loop {
        delay.delay_ms(1000u32);
        
        match motor_dm::get_pos(0x01) {
            Some(p) => pos = p,
            None => rprintln!("获取电机位置失败"),
        }
        match motor_dm::get_spd(0x01) {
            Some(s) => spd = s,
            None => rprintln!("获取电机速度失败"),
        }
        match motor_dm::get_torque(0x01) {
            Some(t) => torque = t,
            None => rprintln!("获取电机扭矩失败"),
        }
        rprintln!("当前电机位置: {:.2} rad, 速度: {:.2} rad/s, 扭矩: {:.2} Nm", pos, spd, torque);
    }
}
