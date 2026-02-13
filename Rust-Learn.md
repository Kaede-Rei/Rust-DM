[TOC]

# STM32H7 Rust 项目构建

## 1. 编译器配置 (.cargo/config.toml)

- 终端输入 `cargo new <repo_id>` 

- 新建 `<repo_id>/.cargo/config.toml` 

  在 [Rust 包管理仓库](https://crates.io/) 里搜索 `stm32h7` : [stm32h7 - crates.io: Rust Package Registry](https://crates.io/crates/stm32h7)，选择 `Dependents` 找到 `stm32h7xx-hal` 找到对应的 `github` 仓库，复制其中的 `.cargo/config.toml` 内容粘贴过来（主要需要的是 [target.thumbv7em-none-xxx] 部分和 [build] 部分），注意 [build] 中 `target` 对应的编译器下载: `rustup target add <target>` 

- 但是对于本教程的情况，要按以下格式来配置，主要是调试器需使用 `probe-rs` ：

  ```toml
  # 指向带硬件浮点单元（FPU）的 Cortex-M4/M7 指令集
  [target.thumbv7em-none-eabihf]
  # 调用 probe-rs 工具通过调试器（如 ST-Link/DAP-Link）将程序烧录进芯片并直接启动运行
  runner = "probe-rs run --chip STM32H723VGTx"
  rustflags = [
  	# 告诉链接器使用 link.x 脚本（由 cortex-m-rt 提供，配合 memory.x 使用）
      "-C", "link-arg=-Tlink.x",
      # 针对 M7 内核优化指令
      "-C", "target-cpu=cortex-m7",
  ]
  
  [build]
  # Cargo 默认就会按这个架构进行交叉编译
  target = "thumbv7em-none-eabihf"
  ```

## 2. 修改链接脚本 (memory.x)

新建 `<repo_id>/memory.x` 同理在仓库里找到 `memory.x` 文件将其内容复制粘贴到本地，但是注意要按实际情况，如达妙开发板 STM32H723VGT6：

```toml
MEMORY
{
    FLASH (rx) : ORIGIN = 0x08000000, LENGTH = 1024K
    RAM (xrw)  : ORIGIN = 0x20000000, LENGTH = 128K
}
```

> *可以用该 CLI 来查询目标芯片的内存分配：probe-rs chip info STM32H723VGTx*

## 3. 错误处理 (panic-handler)

为了让单片机在遇到无法恢复的故障时能够进行错误处理，需要添加 `panic-halt` 软件包：`cargo add panic-halt` 并在主函数里导入：`use panic_halt as _;`

## 4. 主函数入口 (no_std,no_main,fn 0->!{loop{}})

在主函数开头添加无标准库，无主函数（从自定义入口函数进入），添加自定义入口函数（先添加 `cortex_m_rt` 包：`cargo add cortex_m_rt`）

```rust
#![no_std]
#![no_main]

use cortex_m_rt::entry;
use panic_halt as _;

use stm32h7xx_hal as hal;
use hal::prelude::*;

#[entry]
fn main() -> ! {
    loop {}
}

```

## 5. 添加 HAL 软件包

添加对应的 HAL 软件包（如这里就是 `cargo add stm32h7xx-hal`），在主函数里添加 `use stm32h7xx_hal as hal; use hal::prelude::*;` ，编译：`cargo build`

此外还需要添加其他依赖，则在 `<repo_id>/Cargo.toml` 里添加：

```toml
[package]
name = "Code-02-STM32H7-DM"
version = "0.1.0"
# Rust 版本规范
edition = "2024"

[dependencies]
# 嵌入式开发的基础设施（如启动代码、中断向量表）
cortex-m = "0.7"
# 嵌入式开发的基础设施（如启动代码、中断向量表）
cortex-m-rt = "0.7"
# 让单片机在遇到无法恢复的故障时能够进行错误处理
panic-halt = "0.2"
# stm32h7xx_hal 里用 stm32h735 通用 h723, h733, h730
stm32h7xx-hal = { version = "0.15", features = ["stm32h735"] }
```

当需要查询当前依赖使用了什么版本以及哪些可更新时，用以下命令行：

```bash
# 查看当前依赖使用了什么版本
cargo tree
# 查看当前依赖使用了什么版本（只看顶层）
cargo tree --depth 1

# 安装 cargo-outdated 插件并查询最新版本
cargo install cargo-outdated
cargo outdated

# 当版本仅一个小数点时，表示允许当前大版本下的所有小版本，两个小数点时指定小版本
```

## 6. 编译优化 (opt-level = "s" panic = "abort")

相应的编译优化，也填在 `<repo_id>/Cargo.toml` 里，默认使用 dev 模式，当 `cargo build/run --release` 时使用 release 模式：

```toml
[profile.dev]
# 调试模式
debug = true
# 开启时只重编修改的代码，速度快，代价是编译器无法进行全局跨函数的性能优化
incremental = true
# 默认 Rust 在 panic 时会尝试“堆栈展开”，这需要大量的库支持
# 改为 abort 后，遇到错误直接自杀（进入死循环），能省下几 KB 的 Flash 空间
panic = "abort"
# 链接时优化，将未用到的函数去除
lto = true
# 优化等级
opt-level = 0

[profile.release]
# 强制编译器只使用一个处理单元来生成代码，生成的二进制文件体积更小
codegen-units = 1
# 调试模式
debug = true
# 关闭时始终重编所有代码，可以对全局跨函数进行性能优化，代价是速度慢
incremental = false
# 默认 Rust 在 panic 时会尝试“堆栈展开”，这需要大量的库支持
# 改为 abort 后，遇到错误直接自杀（进入死循环），能省下几 KB 的 Flash 空间
panic = "abort"
# 链接时优化，将未用到的函数去除
lto = true
# 优化等级
opt-level = "s"
```

| opt-level | 性能  | 体积 | 编译时间 | 典型场景     |
| --------- | ----- | ---- | -------- | ------------ |
| 0         | ⭐     | 大   | 最快     | debug        |
| 1         | ⭐⭐    | 大   | 快       | 测试         |
| 2         | ⭐⭐⭐⭐  | 中   | 中       | 通用 release |
| 3         | ⭐⭐⭐⭐⭐ | 大   | 慢       | 极致性能     |
| s         | ⭐⭐⭐   | 小   | 中       | 嵌入式推荐   |
| z         | ⭐⭐    | 极小 | 中       | 极小固件     |

## 7. 上传固件 (runner)

在配置里选择了 `runner = "probe-rs run --chip STM32H723VGTx"` ，可支持 st-link ，直接烧录：`cargo run`

## 8. 其他常用 CLI

```bash
# 用于检查语法问题，比编译快非常多
cargo check
```

## 9. 工程结构与模块职责

用简单的分层模块组织，便于把板级初始化、驱动、业务逻辑解耦

```
src/
  main.rs        # 入口：调用 init_board + 演示驱动 API
  lib.rs         # 库入口：公开各层模块与 prelude
  apl/           # 应用层 
  drvl/          # 驱动层（DM 电机、WS2812）
  srvl/          # 业务层（预留）
  tools/         # 工具层（预留）
```

- `main.rs` 只保留主流程：初始化板卡、调度任务、打印关键日志
- `lib.rs` 负责公开模块与 `prelude`，把常用导入聚合起来
- `tools` 常用工具
- `drvl` 存放外设驱动与协议适配层
- `srvl` 基于外设驱动与协议封装好通信与算法
- `apl` 将任务逻辑封装在一个任务函数里供 `main.rs` 调度
- `prelude` 统一 re-export，便于在 `main.rs` 里少写路径

## 10. 板级初始化关键点（只关注时钟与电源）

- **电源域**：通过 `PWR.constrain()` 与 `freeze()` 固化供电配置，保证后续时钟配置可用
- **时钟源**：启用 24MHz HSE，作为 PLL 输入
- **PLL 配置**：PLL1 P/Q/R 输出分别用于 SYSCLK 与外设内核时钟
- **总线分频**：SYSCLK=300MHz，AHB=150MHz，APB1~4=75MHz
- **原则**：先电源后时钟，再进入外设初始化与业务逻辑

# Rust 学习

## 1. 运行时与目标约束

- `#![no_std]`：禁用标准库，改用 `core`/`alloc`；嵌入式通常无 OS、无堆或堆受限
- `#![no_main]`：禁用默认入口，交给运行时宏生成入口
- `#[entry]`：由 `cortex-m-rt` 提供，生成复位向量入口
- `panic` 策略：`panic-halt` 直接死循环，避免堆栈展开带来的体积与依赖成本

> *最小运行时、可控的二进制体积与启动流程*

## 2. 属性与宏

- `#[entry]` / `#[interrupt]`：将函数放入中断向量表或入口表
- `#[inline]` / `#[inline(always)]`：在 ISR 或热路径上强制内联，减少函数调用开销（压栈/出栈）
- `#[no_mangle]`：防止符号名被混淆，常用于需要被 C 调用的接口或中断处理函数
- `#[cfg(feature = "...")]`：用特性开关裁剪功能，控制固件体积
- `macro_rules!`：在无反射场景下生成样板代码（如寄存器映射、协议打包）

## 3. 嵌入式 Rust 设计思维

### 3.1 类型状态编程 (Typestate Programming)
Rust 及其 HAL 库的核心优势，将硬件状态编码进类型系统中，**在编译期杜绝运行时错误**

*   **例子**：GPIO 引脚的配置
    *   `PA5<Input<Floating>>`：输入浮空状态
    *   `PA5<Output<PushPull>>`：推挽输出状态
    *   **优势**：无法将一个配置为 Input 的引脚传给需要 Output 引脚的驱动函数；如果不匹配，代码**无法编译通过**，而不是在运行时崩溃

### 3.2 零成本抽象 (Zero-Cost Abstractions)
Rust 的高级抽象（如迭代器、闭包、Future）编译后生成的汇编代码，通常与手写的优化 C / 汇编一样高效，没有任何运行时额外开销

*   **例子**：
    ```rust
    // 这种写法编译后往往等价于简单的汇编循环指令
    iterator.map(|x| x + 1).filter(|x| x > 10).collect()
    ```

### 3.3 RAII 与单例模式
*   **RAII (资源获取即初始化)**：驱动结构体创建时初始化硬件，销毁时自动关闭（Drop trait）
*   **单例模式 (Singleton)**：`Peripherals::take().unwrap()` 确保外设实例在全剧中只能被获取一次，从根本上防止了多个驱动同时操作同一个硬件寄存器的竞态条件

## 4. 所有权、生命周期与 static

- **所有权**：保证外设句柄在初始化后只被一个模块持有，避免多处同时写硬件寄存器
- **生命周期**：外设句柄通常是“活到整个程序结束”的资源；可用 `static` 或全局单例持有
- `static` + `Mutex<RefCell<...>>`：在中断与前台共享资源时，常用安全模式

## 5. 临界区与中断安全

- `cortex_m::interrupt::free(|cs| { ... })`：关闭中断的临界区
- `Mutex<RefCell<T>>`：借助临界区完成“运行期可变性”
- ISR 中尽量短小：只做缓存/置位，复杂处理放到主循环

> *避免竞态条件，确保实时性*

## 6. nb 非阻塞模型与 async/await

- `nb` crate：通过返回 `WouldBlock` 错误码来实现非阻塞轮询
- `async/await` (Embassy)：更现代的方案，利用 Rust 的状态机生成能力，让单片机能像写同步代码一样写异步代码，由 Executor 在空闲时自动休眠（WFI），极致省电

## 7. 外设知识

### 7.2 RCC (复位与时钟控制)
MCU 的心脏；STM32H7 的时钟树极其复杂：
*   **HSE/HSI**: 外部高速/内部高速时钟源
*   **PLL (锁相环)**: 将低频输入倍频到高频 (如 300MHz)；包含 P/Q/R 分频输出
*   **AHB/APB**: 总线时钟，外设挂载在不同总线上，需配置对应的分频系数

### 7.3 SPI (串行外设接口)
全双工同步通信
*   **四线**: SCLK (时钟), MOSI (主出从入), MISO (主入从出), CS (片选)
*   **模式 (Mode 0-3)**: 由 CPOL (时钟极性) 和 CPHA (时钟相位) 决定；WS2812 驱动中常用模拟 SPI 时序

### 7.4 CAN-FD (灵活数据速率控制器局域网)
*   **仲裁**: ID 小的优先级高，非破坏性仲裁
*   **FD 特性**: 
    *   **可变速率**: 控制段维持标称波特率，由于数据段加速 (如 5Mbps)
    *   **长载荷**: 数据段最大支持 64 字节 (标准 CAN 仅 8 字节)
*   **FIFO & Filter**: 硬件过滤无关 ID，减轻 CPU 负担

## 8. 依赖包

### 8.1 `cortex-m` & `cortex-m-rt`

- **作用**：提供 Core 内核访问、启动代码、中断管理
- **常用**：

```rust
use cortex_m::asm;
use cortex_m::interrupt;

// 临界区：关闭中断，防止数据竞争
interrupt::free(|cs| {
    // 安全访问 Mutex 保护的 static 资源
});
```

```rust
// 属性宏：指定入口点
#![no_main]
#[entry]
fn main() -> ! { loop {} }
```

### 8.2 `stm32h7xx-hal`

- **作用**：芯片级 HAL，封装着 STM32H7 的复杂寄存器操作；核心模式为 Extension Trait 与 Builder Pattern
- **用法**：

```rust
// 获取外设单例 (Singleton)
let dp = hal::pac::Peripherals::take().unwrap();

// 约束 (Constrain) - 将寄存器块转换为 HAL 对象
let rcc = dp.RCC.constrain();

// 配置与冻结 (Freeze) - 时钟树配置一旦生效，便不可变
let clocks = rcc.use_hse(24.MHz()).sys_ck(300.MHz()).freeze(pwrcfg, &dp.SYSCFG);

// 类型转换 (Type State) - 改变引脚模式
let gpioa = dp.GPIOA.split(clocks.peripheral.GPIOA);
let mut sck = gpioa.pa5.into_alternate(); // PA5 变身为复用功能
```

### 8.3 `rtt-target`

- **作用**：通过 JLink/STLink 直接打印日志到主机终端，不占用 UART，速度极快
- **用法**：

```rust
rtt_init_print!();
rprintln!("Hello Embedded Rust!");
```

### 8.4 `panic-halt`

- **作用**：最简单的 Panic 处理——死循环；保留该包可确保程序在异常时立即停止，保留现场

### 8.5 `fdcan`

- **作用**：FDCAN 协议抽象
- **用法**：

```rust
use fdcan::frame::{FrameFormat, TxFrameHeader};
use fdcan::id::StandardId;

let header = TxFrameHeader {
    len: 8,
    id: StandardId::new(0x123).unwrap().into(),
    frame_format: FrameFormat::Standard,
    bit_rate_switching: false,
    marker: None,
};
```

### 8.6 `nb`

- **作用**：非阻塞结果类型与宏
- **用法**：

```rust
// 将非阻塞调用转换为阻塞调用，直到操作完成
let result = nb::block!(can.transmit(header, &data));
```
