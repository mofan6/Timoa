# Timoa 1.1.5 · Windows 兼容版本

此目录是从正式版提交 `03be438d34ab1592a5226d40401ee71b9d93ec18` 独立复制的兼容源码。
不与正常版共享源文件，不修改正常版分支。后续兼容修改应继续在此目录维护。

## 版本与系统

| 包 | 目标系统 | CPU |
|---|---|---|
| Win7-x64 | Windows 7 SP1 及以上的 64 位系统 | x86-64 |
| Win7-x86 | Windows 7 SP1 及以上的 32 / 64 位系统 | x86，需 SSE2 |

保留正常版全部的倒计时、正计时、单一专注、悬浮窗、闹钟、一周计划、三种语言、
明暗模式、自定义音频、备份导入导出、开机启动及托盘交互。
计时引擎、数据结构、页面布局、配色、字体声明和图标素材均与当前正常版同步。
关于页仅将系统说明从 Windows 10 / 11 改为 Windows 7+。

Windows 7 缺少部分新系统字体和 Windows 11 的 DWM 圆角属性，实际字体回退、
系统外框由操作系统决定，无法保证与 Windows 11 像素级一致。
Slint 官方桌面支持范围主要为 Windows 10 / 11；本兼容适配不代表其官方支持声明。

## 兼容修改

- 使用 Rust 的 `x86_64-win7-windows-gnu` / `i686-win7-windows-gnu` 目标，
  从 `rust-src` 重编译标准库；不使用普通 Windows 目标伪装最低版本。
- 使用 `windows_slim_errors`，避免 WinRT 扩展错误 API 的硬导入。
- `vendor/windows-link` 保留上游许可证，将三个 COM 接口链接到 Win7 的 `ole32.dll`。
- `vendor/i-slint-backend-winit` 保留上游许可证，仅将系统字体 DPI 查询改为动态检测；
  旧系统使用 `SystemParametersInfoW` 并换算到 96 DPI。新系统沿用原 API。
- `src/native.rs` 动态检测窗口 DPI / 系统边框 DPI API，Win7 使用传统 GDI 和系统度量。
- x86 内嵌独立编译的 32 位 FFmpeg，音频格式配置保持一致；按架构隔离解码器缓存。
- manifest、资源编译器、链接器和安装器按系统与 CPU 架构适配。

业务数据位置和单实例标识沿用正式版，避免多个版本同时写数据。
安装器沿用相同安装目录，安装不同架构会替换已安装的程序。
建议切换前从设置中导出备份；备份维持原规则，不包含自定义音频。

## 构建

在 Windows x64 上安装：

1. Rust `1.99.0-x86_64-pc-windows-gnu`、`rust-src` 与 `i686-pc-windows-gnu` target
   （只取其中的 `crt2.o`、`dllcrt2.o` 启动对象，不链接其中的预编译 Rust 标准库）。
2. w64devkit 2.10.1（包含 i686 / x86_64 GCC、windres）。
3. 打包时使用有 x86 / amd64 Unicode stubs 和 System 插件的 NSIS；UPX 5.1.0 可选。

```powershell
rustup toolchain install 1.99.0-x86_64-pc-windows-gnu --profile minimal --component rust-src
rustup target add --toolchain 1.99.0-x86_64-pc-windows-gnu i686-pc-windows-gnu
rustup override set 1.99.0-x86_64-pc-windows-gnu
$env:CARGO_TARGET_DIR = 'D:\TimoaBuild\target-compat'
.\build.ps1 -Architecture All -Setup -ToolchainBin 'D:\w64devkit\bin' -NsisCompiler 'D:\w64devkit\share\nsis\Bin\makensis.exe' -Upx 'D:\upx\upx.exe'
```

`build.ps1` 使用 build-std / host-config 所需的 Cargo 开关，因此仅在脚本进程中设置
`RUSTC_BOOTSTRAP=1`。复制的 CRT 文件仅为启动与链接库，标准库始终从源码构建。
仅安装 x86 cross target 不包含完整 MinGW CRT，因此脚本按固定 SHA256 获取
Rust 官方同版本的 `rust-mingw` x86 原生组件，缓存到构建目录的 `native-crt-x86`。
此组件约 5.3 MiB，只有构建输入；不会安装到用户系统，也不包含预编译 Rust 标准库。
构建仍由 x64 编译器完成，最终用户无需另装运行库。
依赖由 Cargo.lock 固定；输出位于 `dist/Win7-x64` 与 `dist/Win7-x86`。

### 重新编译 x86 音频组件

解压 `third-party/FFmpeg/FFmpeg-source.zip` 到 D 盘后执行：

```powershell
.\third-party\FFmpeg\build-audio-compat.ps1 -ToolchainBin 'D:\w64devkit\bin' -SourceDirectory 'D:\FFmpeg-46d899d7a02bc63237472cc4897258d5ddf274ac' -BuildDirectory 'D:\ffmpeg-build-x86' -Architecture x86
```

将生成的 ffmpeg.exe 与 `assets/audio-tools.zip` 中保留的上游许可、
`licenses/FFmpeg-Win7-x86-NOTICE.txt` 一起压缩为 `assets/audio-tools-x86.zip`；
文件直接放 ZIP 根目录。上游 FFmpeg 源码未修改。

## 检查范围

本次交付检查编译结果、PE 架构、子系统版本、硬导入接口和源码差异。
未运行实际功能测试、窗口检查或 Windows 7 真机 / 虚拟机测试。
静态兼容检查不能替代目标系统运行验证，下载说明应保留此限制。

参考：
- https://doc.rust-lang.org/rustc/platform-support/win7-windows-gnu.html
- https://docs.slint.dev/latest/docs/slint/guide/platforms/desktop/windows/general/

## 快捷键源码增补（2026-10-16）

本独立源码已与正常版同步 M 三连按开始 / 继续 / 再开一轮、N 暂停逻辑。
详见 `docs/KEYBOARD-SHORTCUTS.md`。本次不编译、不重新打包 Win7 EXE 或安装器；
此前的 PE 检查报告和二进制哈希仅对应快捷键加入之前的兼容版成品。
本源码包加入了 Slint winit 事件访问功能，编译方式仍使用本目录的 build.ps1。

## 提醒与自动循环源码增补（2026-10-17）

已同步正常版的即时提示音、预计结束时间、默认 / 快捷延时与自动循环功能。
新设置随 JSON 备份保存，新增翻译和深浅色界面一并同步。
本次仅正常版编译供用户检查，兼容版仍只更新独立源码。历史 PE 检查报告对应此前成品。
详见 `docs/REMINDER-CYCLE-UPDATE.md`。

## 1.1.5 同步发布（2026-10-17）

本次同步所有近期功能、固定刻度统计图和两段淡入淡出音乐，并重新构建 x64 / x86 单 EXE 和安装包。上方“仅源码同步”的记录为历史过程；本次成品以 artifact-manifest.json 与发布校验值为准。正常版和兼容版继续独立维护。
