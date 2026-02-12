use core::num::{NonZeroU8, NonZeroU16};
use fdcan::{
    config::{FrameTransmissionConfig, NominalBitTiming},
    filter::{StandardFilter, StandardFilterSlot},
};
use hal::{delay::Delay, gpio::Speed, pac, prelude::*, rcc::rec, spi};
use stm32h7xx_hal as hal;

use rtt_target::{rprint, rprintln, rtt_init_print};

use crate::drvl::*;

/// ### 描述
/// - 初始化板卡及所有外设
///
/// ### 功能
/// - 配置系统时钟为 300MHz (HSE 24MHz)
/// - 初始化 SPI6 用于 WS2812
/// - 初始化 CAN1 用于 DM 电机 (PD0 RX, PD1 TX, PC14 使能, 1Mbps)
///
pub fn init_board() -> Delay {
    // ========== 初始化板卡及所有外设 ========== //

    rtt_init_print!();
    rprintln!("========== 正在初始化板卡... ==========");

    // 获取外设句柄
    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();
    let pwr = dp.PWR.constrain();
    let pwrcfg = pwr.freeze();

    // 启用外部晶振
    let rcc = dp.RCC.constrain();
    let clocks = rcc
        .use_hse(24.MHz())
        .sys_ck(300.MHz())
        .pll1_p_ck(300.MHz())
        .pll1_q_ck(100.MHz())
        .pll1_r_ck(150.MHz())
        .hclk(150.MHz()) // AHB = 150 MHz（SYSCLK / 2）
        .pclk1(75.MHz()) // APB1
        .pclk2(75.MHz()) // APB2
        .pclk3(75.MHz()) // APB3
        .pclk4(75.MHz()) // APB4
        .freeze(pwrcfg, &dp.SYSCFG);

    rprintln!("- 时钟配置完成，正在配置外设...");

    let mut delay = Delay::new(cp.SYST, clocks.clocks);

    // ========== SPI6 - WS2812 ========== //

    rprintln!("- 正在配置 SPI6 - WS2812...");

    // 配置 GPIO
    let gpioa = dp.GPIOA.split(clocks.peripheral.GPIOA);

    // PA5 作为 SPI6 SCK
    let sck = gpioa.pa5.into_alternate();

    // PA7 作为 SPI6 MOSI
    let mosi = gpioa.pa7.into_alternate();

    // 配置 SPI6 - 6MHz, MODE_1 (CPOL=Low, CPHA=2 Edge)
    let spi = dp.SPI6.spi(
        (sck, spi::NoMiso, mosi),
        spi::MODE_1,
        6.MHz(),
        clocks.peripheral.SPI6,
        &clocks.clocks,
    );

    // 初始化 WS2812 驱动
    rprint!("- ");
    rgb_ws2812::init(spi);

    // ========== CAN1 - DM 电机 ========== //

    rprintln!("- 正在配置 CAN1 - DM 电机...");

    // FDCAN 时钟源选择: PLL1Q = 100MHz
    let fdcan_prec = clocks
        .peripheral
        .FDCAN
        .kernel_clk_mux(rec::FdcanClkSel::Pll1Q);

    // PC14 - CAN 收发器使能引脚，置高使能
    let gpioc = dp.GPIOC.split(clocks.peripheral.GPIOC);
    let mut can_en = gpioc.pc14.into_push_pull_output();
    can_en.set_high();

    // PD0 - CAN1 RX, PD1 - CAN1 TX (AF9)
    let gpiod = dp.GPIOD.split(clocks.peripheral.GPIOD);
    let rx = gpiod.pd0.into_alternate().speed(Speed::VeryHigh);
    let tx = gpiod.pd1.into_alternate().speed(Speed::VeryHigh);

    // 创建 FDCAN1 实例（进入 ConfigMode）
    let mut can1 = dp.FDCAN1.fdcan(tx, rx, fdcan_prec);

    // 标称位时序: 100MHz / (10 × (1 + 6 + 3)) = 1Mbps
    can1.set_nominal_bit_timing(NominalBitTiming {
        prescaler: NonZeroU16::new(10).unwrap(),
        seg1: NonZeroU8::new(6).unwrap(),
        seg2: NonZeroU8::new(3).unwrap(),
        sync_jump_width: NonZeroU8::new(1).unwrap(),
    });

    // 过滤器: 接受所有标准帧到 FIFO0
    can1.set_standard_filter(
        StandardFilterSlot::_0,
        StandardFilter::accept_all_into_fifo0(),
    );

    // 应用 CAN 配置:
    //   - 禁用自动重传 (auto retransmission disable)
    //   - 禁用发送暂停 (transmit pause disable)
    //   - 禁用协议异常处理 (protocol exception disable)
    //   - Classic CAN 模式
    let config = can1
        .get_config()
        .set_automatic_retransmit(false)
        .set_transmit_pause(false)
        .set_protocol_exception_handling(false)
        .set_frame_transmit(FrameTransmissionConfig::ClassicCanOnly);
    can1.apply_config(config);

    // 切换到 Normal Mode
    let can1 = can1.into_normal();

    // 初始化 DM 电机 CAN 驱动
    rprint!("- ");
    motor_dm::init(can1);

    // ========== 启用各驱动 ========== //

    // 等待系统稳定
    delay.delay_ms(1500u32);

    // 使能 DM 电机
    motor_dm::enable(0x01);

    // 蓝色常亮 - 系统正常
    rgb_ws2812::blue();

    // ========== 完成并返回 ========== //

    rprintln!("========== 板卡初始化完成 ==========");

    delay
}
