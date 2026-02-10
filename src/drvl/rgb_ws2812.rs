use core::cell::RefCell;
use cortex_m::interrupt::Mutex;

use hal::{pac, prelude::*, spi};
use stm32h7xx_hal as hal;

const LOW_LEVEL: u8 = 0xC0; // 表示 WS2812 的 0
const HIGH_LEVEL: u8 = 0xF0; // 表示 WS2812 的 1

/// WS2812 全局静态SPI实例（使用Mutex保护）
static WS2812_SPI: Mutex<RefCell<Option<spi::Spi<pac::SPI6, spi::Enabled, u8>>>> =
    Mutex::new(RefCell::new(None));

/// ### 描述
/// - 初始化 WS2812 驱动
/// 
/// ### 参数
/// - spi: 已配置好的SPI6实例
/// 
pub fn init(spi: spi::Spi<pac::SPI6, spi::Enabled, u8>) {
    cortex_m::interrupt::free(|cs| {
        *WS2812_SPI.borrow(cs).borrow_mut() = Some(spi);
    });
}

/// ### 描述
/// - 设置WS2812颜色
/// 
/// ### 参数
/// - r: 红色分量 (0-255)
/// - g: 绿色分量 (0-255)
/// - b: 蓝色分量 (0-255)
///
pub fn set_color(r: u8, g: u8, b: u8) {
    let mut txbuf = [0u8; 24];

    // 将RGB转换为GRB格式的SPI数据
    for i in 0..8 {
        // Green
        txbuf[7 - i] = if (g >> i) & 0x01 != 0 {
            HIGH_LEVEL >> 1
        } else {
            LOW_LEVEL >> 1
        };

        // Red
        txbuf[15 - i] = if (r >> i) & 0x01 != 0 {
            HIGH_LEVEL >> 1
        } else {
            LOW_LEVEL >> 1
        };

        // Blue
        txbuf[23 - i] = if (b >> i) & 0x01 != 0 {
            HIGH_LEVEL >> 1
        } else {
            LOW_LEVEL >> 1
        };
    }

    cortex_m::interrupt::free(|cs| {
        if let Some(spi) = &mut *WS2812_SPI.borrow(cs).borrow_mut() {
            // 发送复位信号
            let reset = [0u8];
            let _ = spi.write(&reset);

            // 发送颜色数据
            let _ = spi.write(&txbuf);

            // 发送额外的低电平以确保复位时间足够
            let reset_buf = [0u8; 100];
            let _ = spi.write(&reset_buf);
        }
    });
}

/// 清除LED（关闭）
pub fn clear() {
    set_color(0, 0, 0);
}

/// 设置LED为红色
pub fn red() {
    set_color(255, 0, 0);
}

/// 设置LED为绿色
pub fn green() {
    set_color(0, 255, 0);
}

/// 设置LED为蓝色
pub fn blue() {
    set_color(0, 0, 255);
}

/// 黄色
pub fn yellow() {
    set_color(255, 255, 0);
}

/// 青色
pub fn cyan() {
    set_color(0, 255, 255);
}

/// 品红色
pub fn magenta() {
    set_color(255, 0, 255);
}

/// 白色
pub fn white() {
    set_color(255, 255, 255);
}
