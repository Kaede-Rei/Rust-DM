use hal::{
    pac,
    prelude::*,
    spi,
};
use stm32h7xx_hal as hal;

use rtt_target::rprintln;

use crate::drvl::*;

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
    // 对于 500 MHz，需要最高电压域 VOS0
    let pwrcfg = pwr.vos0(&dp.SYSCFG).freeze();

    rprintln!("正在初始化板卡...");

    // 启用外部晶振
    let rcc = dp.RCC.constrain();
    let clocks = rcc
        .use_hse(24.MHz())
        .sys_ck(300.MHz())
        .pll1_p_ck(300.MHz())
        .pll1_q_ck(100.MHz())
        .pll1_r_ck(150.MHz())
        .hclk(150.MHz())                // AHB = 150 MHz（SYSCLK / 2）
        .pclk1(75.MHz())                // APB1
        .pclk2(75.MHz())                // APB2
        .pclk3(75.MHz())                // APB3
        .pclk4(75.MHz())                // APB4
        .freeze(pwrcfg, &dp.SYSCFG);

    rprintln!("时钟配置完成，正在配置外设...");

    // 配置 GPIO
    let gpioa = dp.GPIOA.split(clocks.peripheral.GPIOA);

    // PA5 作为 SPI6 SCK
    let sck = gpioa.pa5.into_alternate::<8>();

    // PA7 作为 SPI6 MOSI
    let mosi = gpioa.pa7.into_alternate::<8>();

    // 配置 SPI6 - 6MHz, MODE_1 (CPOL=Low, CPHA=2 Edge)
    let spi = dp.SPI6.spi(
        (sck, spi::NoMiso, mosi),
        spi::MODE_1,
        6.MHz(),
        clocks.peripheral.SPI6,
        &clocks.clocks,
    );

    // 初始化 WS2812 驱动
    rgb_ws2812::init(spi);

    rprintln!("SYSCLK: {} Hz", clocks.clocks.sys_ck().raw());
    rprintln!("板卡初始化完成");
}
