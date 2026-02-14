use core::cell::RefCell;
use cortex_m::asm;
use cortex_m::interrupt::Mutex;

use hal::pac;
use hal::pac::interrupt;
use stm32h7xx_hal as hal;

use fdcan::{
    frame::{FrameFormat, RxFrameInfo, TxFrameHeader},
    id::StandardId,
    interrupt::{Interrupt as CanInterrupt, InterruptLine},
};

use rtt_target::rprintln;

// ! ========================= 变 量 声 明 ========================= ! //

/// CAN1 实例类型别名（Normal 操作模式）
type CanInstance = fdcan::FdCan<hal::can::Can<pac::FDCAN1>, fdcan::NormalOperationMode>;

/// CAN1 全局静态实例（使用 Mutex 保护）
static CAN1: Mutex<RefCell<Option<CanInstance>>> = Mutex::new(RefCell::new(None));

/// CAN 接收缓冲区（中断版本）
/// - 索引 0: 有效标志 (0=无数据, 1=有新数据)
/// - 索引 1..9: 接收到的 8 字节数据
static CAN_RX_BUF: Mutex<RefCell<[u8; 9]>> = Mutex::new(RefCell::new([0u8; 9]));

/// CAN 接收帧信息（中断版本）
static CAN_RX_INFO: Mutex<RefCell<Option<RxFrameInfo>>> = Mutex::new(RefCell::new(None));

// ! ========================= 接 口 函 数 实 现 ========================= ! //

/// ### 描述
/// - 初始化 DM 电机 CAN 驱动
///
/// ### 参数
/// - can: 已配置好的 FDCAN1 实例（Normal 模式）
///
pub fn init(mut can: CanInstance) {
    // 启用 FIFO0 新消息中断，映射到中断线 0 (FDCAN1_IT0)
    can.enable_interrupt(CanInterrupt::RxFifo0NewMsg);
    can.enable_interrupt_line(InterruptLine::_0, true);

    cortex_m::interrupt::free(|cs| {
        *CAN1.borrow(cs).borrow_mut() = Some(can);
    });

    // 使能 NVIC 中的 FDCAN1_IT0 中断
    unsafe {
        cortex_m::peripheral::NVIC::unmask(pac::Interrupt::FDCAN1_IT0);
    }

    rprintln!("DM 电机 CAN 驱动初始化完成（已启用 FIFO0 接收中断）");
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
    
    asm::delay(300_000);
}

/// ### 描述
/// - 从中断接收缓冲区读取最新 CAN 数据帧
///
/// ### 参数
/// - buffer: 接收缓冲区（至少 8 字节）
///
/// ### 返回
/// - Some(RxFrameInfo): 中断已收到新数据，返回帧头信息，数据写入 buffer
/// - None: 无新数据
/// 
pub fn can_rcvd(buffer: &mut [u8]) -> Option<RxFrameInfo> {
    cortex_m::interrupt::free(|cs| {
        let mut rx_buf = CAN_RX_BUF.borrow(cs).borrow_mut();
        let mut rx_info = CAN_RX_INFO.borrow(cs).borrow_mut();

        if rx_buf[0] == 1 {
            // 有新数据，拷贝到 buffer
            let len = core::cmp::min(buffer.len(), 8);
            buffer[..len].copy_from_slice(&rx_buf[1..1 + len]);

            // 清除有效标志
            rx_buf[0] = 0;

            rx_info.take()
        } else {
            None
        }
    })
}

/// ### 描述
/// - 使能 DM 电机
///
/// ### 参数
/// - id: DM 电机 CAN ID (11-bit, 0x000~0x7FF)
///
pub fn enable(id: u16) {
    let data = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFC];

    can_send(id, &data);
}

/// ### 描述
/// - 失能 DM 电机
///
/// ### 参数
/// - id: DM 电机 CAN ID (11-bit, 0x000~0x7FF)
///
pub fn disable(id: u16) {
    let data = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFD];

    can_send(id, &data);
}

/// ### 描述
/// - MIT 模式: 设置 DM 电机位置、速度、Kp、Kd、转矩
///
/// ### 参数
/// - id: DM 电机 CAN ID (11-bit, 0x000~0x7FF)
/// - pos: 目标位置 (16-bit)
/// - spd: 目标速度 (12-bit)
/// - kp: 位置环增益 (12-bit)
/// - kd: 速度环增益 (12-bit)
/// - torque: 转矩 (12-bit)
///
pub fn set_mit(id: u16, pos: f32, spd: f32, kp: f32, kd: f32, torque: f32) {
    switch_mode(id, 1);

    let pos_bits = f32_to_u16(pos, -12.5, 12.5, 16);
    let spd_bits = f32_to_u16(spd, -10.0, 10.0, 12);
    let kp_bits = f32_to_u16(kp, 0.0, 500.0, 12);
    let kd_bits = f32_to_u16(kd, 0.0, 5.0, 12);
    let torque_bits = f32_to_u16(torque, -28.0, 28.0, 12);

    let data = [
        (pos_bits >> 8) as u8,
        (pos_bits) as u8,
        (spd_bits >> 4) as u8,
        (((spd_bits & 0xF) << 4) as u8 | (kp_bits >> 8) as u8) as u8,
        kp_bits as u8,
        (kd_bits >> 4) as u8,
        (((kd_bits & 0xF) << 4) as u8 | (torque_bits >> 8) as u8) as u8,
        torque_bits as u8,
    ];

    can_send(id, &data);
}

/// ### 描述
/// - 位置速度模式: 设置 DM 电机位置和速度
///
/// ### 参数
/// - id: DM 电机 CAN ID (11-bit, 0x000~0x7FF)
/// - pos: 目标位置 (float, 4 字节)
/// - spd: 目标速度 (float, 4 字节)
///
pub fn set_pos_spd(id: u16, pos: f32, spd: f32) {
    switch_mode(id, 2);

    let id = id + 0x100;
    let pos_bytes = pos.to_le_bytes();
    let spd_bytes = spd.to_le_bytes();

    let data = [
        pos_bytes[0],
        pos_bytes[1],
        pos_bytes[2],
        pos_bytes[3],
        spd_bytes[0],
        spd_bytes[1],
        spd_bytes[2],
        spd_bytes[3],
    ];

    can_send(id, &data);
}

/// ### 描述
/// - 速度模式: 设置 DM 电机速度
///
/// ### 参数
/// - id: DM 电机 CAN ID (11-bit, 0x000~0x7FF)
/// - spd: 目标速度 (float, 4 字节)
///
pub fn set_spd(id: u16, spd: f32) {
    switch_mode(id, 3);

    let id = id + 0x200;
    let spd_bytes = spd.to_le_bytes();

    let data = [
        spd_bytes[0],
        spd_bytes[1],
        spd_bytes[2],
        spd_bytes[3],
        0,
        0,
        0,
        0,
    ];

    can_send(id, &data);
}

/// ### 描述
/// - 位置速度电流模式: 设置 DM 电机位置、速度和电流
///
/// ### 参数
/// - id: DM 电机 CAN ID (11-bit, 0x000~0x7FF)
/// - pos: 目标位置 (float, 4 字节)
/// - spd: 目标速度 (float, 4 字节)
/// - cur: 目标电流 (float, 4 字节)
///
pub fn set_pos_spd_cur(id: u16, pos: f32, spd: f32, cur: f32) {
    switch_mode(id, 4);

    let id = id + 0x300;
    let pos_bytes = pos.to_le_bytes();
    let spd_bytes = ((spd * 100.0) as u16).to_le_bytes();
    let cur_bytes = ((cur * 10000.0) as u16).to_le_bytes();

    let data = [
        pos_bytes[0],
        pos_bytes[1],
        pos_bytes[2],
        pos_bytes[3],
        spd_bytes[0],
        spd_bytes[1],
        cur_bytes[0],
        cur_bytes[1],
    ];

    can_send(id, &data);
}

/// ### 描述
/// - 获取 DM 电机反馈数据
///
/// ### 参数
/// - id: DM 电机 CAN ID (11-bit, 0x000~0x7FF)
/// - feedback: 反馈数据缓冲区（8 字节）:
///
pub fn get_feedback(id: u16, feedback: &mut [u8; 8]) -> Option<()> {
    let can_id_l = id & 0xFF;
    let can_id_h = (id >> 8) & 0x07;

    let data = [can_id_l as u8, can_id_h as u8, 0xCC, 0x00, 0, 0, 0, 0];
    can_send(0x7FF, &data);

    loop {
        let has_data = cortex_m::interrupt::free(|_| can_rcvd(feedback).is_some());

        if has_data {
            let excepted_id = (can_id_l & 0x0F) as u8;
            let rcvd_id = (feedback[0] & 0x0F) as u8;
            if excepted_id == rcvd_id {
                return Some(());
            } else {
                return None;
            }
        }
        cortex_m::asm::wfi();
    }
}

/// ### 描述
/// - 获取 DM 电机错误代码
///
/// ### 参数
/// - id: DM 电机 CAN ID (11-bit, 0x000~0x7FF)
///
/// ### 返回
/// - Some(u8): 错误代码 (0~15)
/// - None: 获取失败
///
pub fn get_err_code(id: u16) -> Option<u8> {
    let mut feedback = [0u8; 8];
    get_feedback(id, &mut feedback)?;

    let err_code = (feedback[0] >> 4) as u8;
    Some(err_code)
}

/// ### 描述
/// - 获取 DM 电机位置反馈
///
/// ### 参数
/// - id: DM 电机 CAN ID (11-bit, 0x000~0x7FF)
///
/// ### 返回
/// - Some(u8): 位置反馈 (float, 4 字节)
/// - None: 获取失败
///
pub fn get_pos(id: u16) -> Option<f32> {
    let mut feedback = [0u8; 8];
    get_feedback(id, &mut feedback)?;

    let pos_bytes = ((feedback[1] as u16) << 8) | (feedback[2] as u16);
    Some(u16_to_f32(pos_bytes, -12.5, 12.5, 16))
}

/// ### 描述
/// - 获取 DM 电机速度反馈
///
/// ### 参数
/// - id: DM 电机 CAN ID (11-bit, 0x000~0x7FF)
///
/// ### 返回
/// - Some(u8): 速度反馈 (float, 4 字节)
/// - None: 获取失败
///
pub fn get_spd(id: u16) -> Option<f32> {
    let mut feedback = [0u8; 8];
    get_feedback(id, &mut feedback)?;

    let spd_bytes = ((feedback[3] as u16) << 4) | (((feedback[4] & 0xF0) >> 4) as u16);
    Some(u16_to_f32(spd_bytes, -10.0, 10.0, 12))
}

/// ### 描述
/// - 获取 DM 电机转矩反馈
///
/// ### 参数
/// - id: DM 电机 CAN ID (11-bit, 0x000~0x7FF)
///
/// ### 返回
/// - Some(u8): 转矩反馈 (float, 4 字节)
/// - None: 获取失败
///
pub fn get_torque(id: u16) -> Option<f32> {
    let mut feedback = [0u8; 8];
    get_feedback(id, &mut feedback)?;

    let torque_bytes = (((feedback[4] & 0x0F) as u16) << 8) | (feedback[5] as u16);
    Some(u16_to_f32(torque_bytes, -28.0, 28.0, 12))
}

// ! ========================= 私 有 函 数 实 现 ========================= ! //

/// ### 描述
/// - FDCAN1 中断处理函数
/// - 当 FIFO0 收到新消息时触发，自动读取数据到全局缓冲区
/// 
#[interrupt]
fn FDCAN1_IT0() {
    cortex_m::interrupt::free(|cs| {
        if let Some(can) = &mut *CAN1.borrow(cs).borrow_mut() {
            if can.has_interrupt(CanInterrupt::RxFifo0NewMsg) {
                can.clear_interrupt(CanInterrupt::RxFifo0NewMsg);

                let mut rx_buf = CAN_RX_BUF.borrow(cs).borrow_mut();
                match can.receive0(&mut rx_buf[1..9]) {
                    Ok(result) => {
                        rx_buf[0] = 1; // 标记有新数据
                        *CAN_RX_INFO.borrow(cs).borrow_mut() = Some(result.unwrap());
                    }
                    Err(_) => {}
                }
            }
        }
    });
}

/// ### 描述
/// - 发送写寄存器命令
///
/// ### 参数
/// - id: DM 电机 CAN ID (11-bit, 0x000~0x7FF)
/// - rid: 寄存器地址
/// - d0~d3: 寄存器数据
///
fn write_register(id: u16, rid: u8, d0: u8, d1: u8, d2: u8, d3: u8) {
    let can_id_l = id & 0xFF;
    let can_id_h = (id >> 8) & 0x07;

    let data = [can_id_l as u8, can_id_h as u8, 0x55, rid, d0, d1, d2, d3];
    can_send(0x7FF, &data);
}

/// ### 描述
/// - 切换 DM 电机控制模式
///
/// ### 参数
/// - id: DM 电机 CAN ID (11-bit, 0x000~0x7FF)
/// - mode: 控制模式 (1: MIT, 2: pos+spd, 3: spd, 4:psi)
///
fn switch_mode(id: u16, mode: u8) {
    write_register(id, 10, mode, 0, 0, 0);
}

/// ### 描述
/// - f32 转 u16
///
/// ### 参数
/// - val: f32 值
/// - min: 最小值
/// - max: 最大值
/// - bits: 位数
///
/// ### 返回
/// - u16 值
///
fn f32_to_u16(val: f32, min: f32, max: f32, bits: u8) -> u16 {
    let span = max - min;
    let offset = min;

    ((val - offset) * (((1 << bits) - 1) as f32) / span) as u16
}

/// ### 描述
/// - u16 转 f32
///
/// ### 参数
/// - val: u16 值
/// - min: 最小值
/// - max: 最大值
/// - bits: 位数
///
/// ### 返回
/// - f32 值
///
fn u16_to_f32(val: u16, min: f32, max: f32, bits: u8) -> f32 {
    let span = max - min;
    let offset = min;

    (val as f32) * span / (((1 << bits) - 1) as f32) + offset
}
