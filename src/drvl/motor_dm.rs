use core::cell::RefCell;
use cortex_m::asm;
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

    asm::delay(300_000);
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

/// ### 描述
/// - 使能 DM 电机
///
/// ### 参数
/// - id: DM 电机 CAN ID (11-bit, 0x000~0x7FF)
///
pub fn dm_enable(id: u16) {
    let data = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFC];

    can_send(id, &data);
}

/// ### 描述
/// - 失能 DM 电机
///
/// ### 参数
/// - id: DM 电机 CAN ID (11-bit, 0x000~0x7FF)
///
pub fn dm_disable(id: u16) {
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
pub fn dm_set_mit(id: u16, pos: f32, spd: f32, kp: f32, kd: f32, torque: f32) {
    switch_mode(id, 1);

    let pos_bits = f32_to_u16(pos, -12.5, 12.5, 16);
    let spd_bits = f32_to_u16(spd, -45.0, 45.0, 12);
    let kp_bits = f32_to_u16(kp, 0.0, 500.0, 12);
    let kd_bits = f32_to_u16(kd, 0.0, 5.0, 12);
    let torque_bits = f32_to_u16(torque, -18.0, 18.0, 12);

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
pub fn dm_set_pos_spd(id: u16, pos: f32, spd: f32) {
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
pub fn dm_set_spd(id: u16, spd: f32) {
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
pub fn dm_set_pos_spd_cur(id: u16, pos: f32, spd: f32, cur: f32) {
    switch_mode(id, 4);

    let id = id + 0x300;
    let pos_bytes = pos.to_le_bytes();
    let spd_bytes = spd.to_le_bytes();
    let cur_bytes = cur.to_le_bytes();

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

    ((val as f32) * span / (((1 << bits) - 1) as f32) + offset)
}
