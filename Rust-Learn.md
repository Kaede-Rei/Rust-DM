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

此外还需要添加其他依赖，则在 `<ropo_id>/Cargo.toml` 里添加：

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
cargo tree -d 1

# 安装 cargo-outdated 插件并查询最新版本
cargo install cargo-outdated
cargo outdated

# 当版本仅一个小数点时，表示允许当前大版本下的所有小版本，两个小数点时指定小版本
```

## 6. 编译优化 (opt-level = "s" panic = "abort")

相应的编译优化，也填在 `<ropo_id>/Cargo.toml` 里，默认使用 dev 模式，当 `cargo build/run --release` 时使用 release 模式：

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
ca
```



# Rust 学习

## 1. 实时传输库 - RTT_TARGET

- **作用：**在 没有 UART 的情况下，通过 SWD/JTAG 调试探针（probe）输出日志

- **安装：**`cargo add rtt-target`

- **API 表格：**

  | 类别              | 名称                             | 说明                                                         |
  | ----------------- | -------------------------------- | ------------------------------------------------------------ |
  | **宏 - 初始化**   | `rtt_init_print!()`              | 初始化默认 RTT Up channel0(用于 `rprintln!`)                 |
  |                   | `rtt_init_defmt!()`              | 初始化用于 `defmt` 格式化日志（需要 `defmt feature`）        |
  |                   | `rtt_init_log!()`                | 初始化 `log` 后端（需要 `log feature`）                      |
  |                   | `rtt_init_default!()`            | 默认初始化宏（panic 等场景也可用）                           |
  | **输出宏**        | `rprintln!()`                    | RTT 输出（println 形式，可格式化）                           |
  |                   | `rprint!()`                      | RTT 原始输出，不追加换行                                     |
  | **通道控制**      | `set_print_channel(chan)`        | 设置默认打印通道为指定通道                                   |
  |                   | `set_defmt_channel(chan)`        | 设置 defmt 日志通道                                          |
  | **Logger 初始化** | `init_logger()`                  | 初始化 `log` logger（默认级别）                              |
  |                   | `init_logger_with_level(level)`  | 初始化 `log` logger 并指定日志级别                           |
  | **底层写入**      | `write(bytes)` / `channel.write` | 向指定 RTT channel 写入原始字节                              |
  | **Feature Flags** | `"log"`                          | 启用 `log` 支持（requires once_cell 等）([Lib.rs](https://lib.rs/crates/rtt-target/features?utm_source=chatgpt.com)) |
  |                   | `"defmt"`                        | 启用 `defmt` 支持                                            |

## 2. 非阻塞操作库 - non-blocking

- **作用：**提供了一组 trait 和宏，让硬件抽象层（HAL）可以用统一的非阻塞风格书写接口，同时又能方便地转换成阻塞式调用，核心思想为操作尚未完成时，返回 Err(nb::Error::WouldBlock)，而不是一直阻塞等待

  > *trait: Rust 中定义接口（共享行为）的机制，相当于其他语言的 interface / protocol*
  >
  > *核心特点：*
  >
  > ​	*可以为类型 **事后添加** 行为（通过 blanket impl 或 upstream crate）*
  >
  > ​	*支持 **默认实现**（default method）*
  >
  > ​	*支持 **关联类型**（associated type）*
  >
  > ​	*支持 **泛型约束**（where 子句 / bound）*

- **安装：**`cargo add nb`

- **API 与 trait 表格：**

  | 名称               | 返回类型                      | 含义                                                         | 最常见用法场景                  |
  | ------------------ | ----------------------------- | ------------------------------------------------------------ | ------------------------------- |
  | `nb::Result<T, E>` | `Result<T, nb::Error<E>>`     | 非阻塞操作的标准返回类型（Either 成功、真实错误、或 WouldBlock） | 几乎所有 HAL 非阻塞方法         |
  | `Error<E>`         | enum { WouldBlock, Other(E) } | WouldBlock 表示“现在还不能完成，再试一次”                    | —                               |
  | `block!(expr)`     | 宏 → `Result<T, E>`           | 反复 poll 直到成功或出现真实错误（最常用阻塞转换宏）         | 临时想用阻塞风格时              |
  | `try_poll!(expr)`  | 宏 → `Poll<Option<T>>`        | 更底层的 poll 接口（类似 futures::Poll）                     | 想自己写 scheduler / reactor 时 |

