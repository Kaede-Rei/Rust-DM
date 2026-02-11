use core::cell::RefCell;
use cortex_m::interrupt::Mutex;

use hal::pac;
use stm32h7xx_hal as hal;

use fdcan::{
    frame::{FrameFormat, RxFrameInfo, TxFrameHeader},
    id::StandardId,
};

use rtt_target::rprintln;

/// CAN1 实例类型别名（Normal 操作模式）
type CanInstance = fdcan::FdCan<hal::can::Can<pac::FDCAN1>, fdcan::NormalOperationMode>;

/// CAN1 全局静态实例（使用 Mutex 保护）
static CAN1: Mutex<RefCell<Option<CanInstance>>> = Mutex::new(RefCell::new(None));

/// ### 描述
/// - 初始化 DM 电机 CAN 驱动
///
/// ### 参数
/// - can: 已配置好的 FDCAN1 实例（Normal 模式）
///
pub fn init(can: CanInstance) {
    cortex_m::interrupt::free(|cs| {
        *CAN1.borrow(cs).borrow_mut() = Some(can);
    });
    rprintln!("DM 电机 CAN 驱动初始化完成");
}

/// ### 描述
/// - 通过 CAN1 发送标准数据帧
///
/// ### 参数
/// - id: 标准帧 ID (11-bit, 0x000~0x7FF)
/// - data: 发送数据切片（最多 8 字节）
///
pub fn can_send(id: u16, data: &[u8]) {
    let header = TxFrameHeader {
        len: data.len() as u8,
        id: StandardId::new(id).unwrap().into(),
        frame_format: FrameFormat::Standard,
        bit_rate_switching: false,
        marker: None,
    };

    cortex_m::interrupt::free(|cs| {
        if let Some(can) = &mut *CAN1.borrow(cs).borrow_mut() {
            match nb::block!(can.transmit(header, data)) {
                Ok(_) => {}
                Err(_) => rprintln!("CAN 发送失败"),
            }
        }
    });
}

/// ### 描述
/// - 从 CAN1 FIFO0 接收数据帧（非阻塞）
///
/// ### 参数
/// - buffer: 接收缓冲区（至少 8 字节）
///
/// ### 返回
/// - Some(RxFrameInfo): 成功接收，返回帧头信息，数据写入 buffer
/// - None: FIFO 中无待接收数据
///
pub fn can_rcvd(buffer: &mut [u8]) -> Option<RxFrameInfo> {
    cortex_m::interrupt::free(|cs| {
        if let Some(can) = &mut *CAN1.borrow(cs).borrow_mut() {
            match can.receive0(buffer) {
                Ok(result) => Some(result.unwrap()),
                Err(nb::Error::WouldBlock) => None,
                Err(_) => None,
            }
        } else {
            None
        }
    })
}

pub fn dm_enable(id: u16) {
    let data = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFC];

    can_send(id, &data);
}
