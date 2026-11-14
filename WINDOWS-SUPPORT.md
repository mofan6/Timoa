# Windows 支持与构建

## 同一套应用，两种架构

- x64：普通 Intel / AMD 64 位电脑。
- x86：32 位电脑，或 x64 Windows 的 WOW64 环境；要求 SSE2。
- 最低系统基线为 Windows 7 SP1。目标覆盖 Windows 7 SP1、8、8.1、10、11；ARM64 原生程序不在本次目标内。
- 程序页面、计时引擎、音频目录、三种语言、备份格式和数据目录统一，用户无需选择“普通版”或“Win7 版”。

## 保留新系统体验

`src/native.rs` 缓存动态解析的 GetDpiForWindow / GetSystemMetricsForDpi。新系统使用对应原生接口，缺失时才读取 GDI / 传统系统度量。

页面继续由同一套 Slint 组件绘制。DWM 暗色边框优先使用属性 20，较早 Win10 使用属性 19 回退；Win11 继续请求标题颜色、边框颜色、圆角与原生缩放边框。旧系统不支持的属性不会阻止程序运行，系统外框随操作系统能力变化。

Slint 的系统字体查询同样动态检测 SystemParametersInfoForDpi；新系统调用 96 DPI 字体度量接口，旧系统将系统像素换算为逻辑像素。

音频解码器按 x64 / x86 嵌入相应架构，程序静态链接语言运行时。现代系统能力保持在运行时检测路径中。

## 固定工具链

Rust 1.99.0 x86_64-pc-windows-gnu、rust-src、i686-pc-windows-gnu 启动对象与完整 x86 MinGW CRT；Slint 1.18.1 与锁定依赖；w64devkit 2.10.1、NSIS。

```powershell
$env:CARGO_TARGET_DIR = 'D:\TimoaBuild\target'
.\build.ps1 -Architecture All -Setup -ToolchainBin 'D:\w64devkit\bin' -NsisCompiler 'D:\w64devkit\share\nsis\Bin\makensis.exe'
```

内部以 x86_64-win7-windows-gnu / i686-win7-windows-gnu 目标从 rust-src 编译标准库。目标名说明最低系统基线，对外文件只按 windows-x64 / windows-x86 分类。

`Cargo.toml` 是应用版本号的统一来源：Rust 资源与关于页从包版本读取，build.ps1 将同一版本传给安装器和文件名。

依赖升级时须复核旧系统硬导入与回退路径，保留 vendor 补丁的上游许可证。历史来源见 COMPATIBILITY.md，当前支持范围以本文件及 docs/unified-windows-review.json 为准。

## 渲染、字体与发布体积

1.3.0 默认启用 Slint 软件渲染器并保留 accessibility。中文使用相同的更纱黑体 Regular / Bold 子集，软件渲染器通过 DirectWrite 生成覆盖图；Windows 7 使用基础工厂与灰度转换，新接口通过 COM 查询动态使用。失败或不支持的字体回退 Swash。原生字体面缓存限制为 16 项，字形缓存沿用 1 MiB 上限，字库数据共享。

主题渐变与名称菜单阴影使用可供两种渲染路径使用的 SVG 资源。1.3.1 的主题按钮按实际物理像素尺寸生成 SVG，并根据屏幕缩放固定圆角和边线；只缓存最近 32 份矢量图，避免拖动窗口时持续积累资源。默认 EXE 不包含 OpenGL/FemtoVG，且不进行 UPX 压缩；安装包通过 LZMA 压缩下载内容。普通 PE 可以按需映射代码页，避免启动时将整个压缩映像解到专用内存中；因此便携文件会比旧版更大。

源码保留可选的 `gpu` 特性，开发者可在相同目标与链接参数的 Cargo 构建中加入 `--features gpu`，再用 `Timoa.exe --renderer=gpu` 启用。默认四份成品没有该特性，不能通过命令行加载一个未编译的 GPU 后端。

## 验证范围

本机是 Windows 11 x64。执行结果记录在 docs/unified-windows-review.json。
1.2.0 的架构、PE 导入、压缩前后代码段与安装包内容的静态核对记录在 docs/release-1.2.0-package-review.json；未执行安装器或新增实际窗口验证。
1.3.0 的两架构编译、PE 导入及安装包内嵌一致性记录见 docs/release-1.3.0-package-review.json。离屏绘制用于页面输出对比，不视为完整交互测试。
1.3.1 的两架构编译与静态包检查记录见 docs/release-1.3.1-package-review.json；未新增实际窗口或旧系统实机验证。
Windows 7、8、8.1、10 当前没有可用真机或虚拟机环境，仅完成编译 / 静态兼容检查；不将其写成已实机验证。
安装器进行静态架构、系统门槛和内嵌文件一致性核对，避免测试安装改动用户已有安装和启动项。

## 源码与原版备份

当前 main 分支及 v1.2.0 / v1.3.0 / v1.3.1 标签采用统一源码；v1.1.5 及合并前的 main / windows7 提交保留在 Git 历史中，原始源码目录和源码包另有本地备份。
后续功能修改只维护本目录的业务和界面源码，再编译两个架构。
