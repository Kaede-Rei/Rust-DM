use hal::{pac, prelude::*, spi};
use stm32h7xx_hal as hal;

use rtt_target::rprintln;

use crate::drvl::rgb_ws2812;

/// ### 描述
/// - 初始化板卡及所有外设
///
/// ### 功能
/// - 配置系统时钟为250MHz
/// - 初始化SPI6用于WS2812
///
pub fn init_board() {
    let dp = pac::Peripherals::take().unwrap();
    let pwr = dp.PWR.constrain();
    let pwrcfg = pwr.freeze();

    let rcc = dp.RCC.constrain();

    // 系统时钟配置为 250MHz
    let ccdr = rcc.sys_ck(250.MHz()).freeze(pwrcfg, &dp.SYSCFG);

    // 配置 GPIO
    let gpioa = dp.GPIOA.split(ccdr.peripheral.GPIOA);

    // PA5 作为 SPI6 SCK
    let sck = gpioa.pa5.into_alternate::<8>();

    // PA7 作为 SPI6 MOSI
    let mosi = gpioa.pa7.into_alternate::<8>();

    // 配置 SPI6 - 6MHz, MODE_1 (CPOL=Low, CPHA=2 Edge)
    let spi = dp.SPI6.spi(
        (sck, spi::NoMiso, mosi),
        spi::MODE_1,
        6.MHz(),
        ccdr.peripheral.SPI6,
        &ccdr.clocks,
    );

    // 初始化 WS2812 驱动
    rgb_ws2812::init(spi);

    rprintln!("板卡初始化完成");
}
