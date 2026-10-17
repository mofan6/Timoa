> 本文件保留早期版本的项目结构与历史开发资料。当前统一 x64 / x86 的目标系统、构建命令和检查范围以 [WINDOWS-SUPPORT.md](../WINDOWS-SUPPORT.md) 为准；下文旧构建步骤不适用于当前统一发布。

# Timoa 1.1.0

面向 Windows 10 / 11 x64 的中文倒计时与闹钟。

## 技术与体积

- Rust + Slint 1.18.1，声明式 `.slint` 组件、单向数据模型与独立调度层。
- 原生编译、静态链接语言运行时。最终用户无需安装 .NET、VC++、WebView2 或 Java。
- 使用 Windows 自带微软雅黑 UI 与 Bahnschrift 矢量字体，避免字体副本与栅格字库。
- winit + FemtoVG 矢量抗锯齿渲染，显卡不支持时回退软件渲染，保留键盘与辅助功能。
- 6 段原版 WAV 无损压缩嵌入。离线 FFmpeg 音频解码组件按需展开。
- 保留深灰黑底与 `#E3B341` 金色，可保存的深浅色主题、DWM 原生窗口边框与 Windows 11 圆角。
- 发布版以官方 UPX 5.1.0 压缩原生 PE，保留图标与重定位。启动时在内存中解压；用户无需安装 UPX。
- 简体中文、繁體中文与 English 内置于 EXE，默认简体中文，在设置中切换并保存；无需下载语言包。

## 功能

多个倒计时、暂停/继续/重置、快捷时长、滚轮时间选择；
一次或每周重复闹钟、提前五分钟提醒、每日安排；
系统托盘、全局快捷键、开机启动、数据导入导出、多格式音频导入。

关闭窗口保留后台托盘；右键托盘 → 退出才结束程序。
系统关机、睡眠或退出程序后无法响铃。恢复时有五分钟补报宽限。
计时使用单调时钟，保存快照包含墙钟截止时间，重启后可恢复未结束倒计时。
Windows 自带字体由系统动态使用，不是需要额外安装的运行库。

## 1.1.0

- 设置 → 更多设置：单一专注和正计时分别开关，默认关闭，原有四个页面编号不变。
- 单一专注把当前计时合并到首页卡片；开始后锁定设置，移除后回到原时长。已有其他计时不会删除。
- 正计时独立保存，暂停不计时，完成本轮才计入总专注时长，重置不计入；重新启动软件时恢复为暂停状态。开始或继续正计时会暂停倒计时，开始或继续倒计时会暂停正计时，均保留进度。
- 精简窗显示当前倒计时或正计时，以及未来 30 分钟内的提醒；可拖动和从边缘/四角调整大小、返回完整窗口或关闭到托盘，调度继续运行。
- 主窗口标题栏新增精简模式入口，保留原生 DWM 边框、圆角、阴影和窗口缩放行为。
- 页面提示条与内容卡片对齐，10 秒后自动消失。
- 新字段使用默认值兼容 1.0.1 JSON 数据和备份，维持原数据目录与单实例标识。

## 1.0.1

- 音频菜单独立试听/停止按钮，试听最多五秒；试听不更改选择。
- 自定义提示音支持连续导入多段音频，每段可分别试听、删除；删除后原本使用它的计划改为静音。
- 闹钟新增/编辑与删除确认在页面内完成，滚动条有独立预留空间。
- 统一按钮悬浮、按下和选中反馈，调整焦点与点击事件顺序。
- 设置提供深浅色主题和总专注时长；新图标采用原有金色与深灰色。
- 总时长只在一轮计时自然完成时累计原设定时长，未完成的重置/移除不累计。
  已完成后再次开启新一轮可再次累计；暂停时间不计入。
  首次升级只能补入当前还保留的已完成记录，之前已删除记录无法还原。
  清除计划和恢复设置保留累计值；JSON 备份包含统计，导入时以备份统计为准。
- 首次显示追加重绘以减轻偶发底部透明条；未复现的情况下不能确认彻底解决。

## 数据

发布前备份与开机启动的静态审查记录见 `docs/代码审查.md`；未实际执行备份恢复、修改启动注册表或安装验证。

数据目录 `%APPDATA%\TIMO`；解码缓存 `%LOCALAPPDATA%\TIMO\codecs`。
首次运行复制旧版 `%APPDATA%\Tidsro\data.json` 与 `sounds/custom.wav`。
不会修改 WPF 版的文件或开机启动项。旧版计时器只在其进程中存在，无法迁移正在运行的计时。
请先退出旧版再用新版，避免重复闹钟以及全局快捷键争用。

导出 JSON 保存设置、计划与累计时长；自定义音频在 `sounds` 目录，需整体另行备份。
导入前保存 `data-before-import.json`。迁移与导入都限制条目数并校验字段。

导入备份不会替代 Windows 的开机启动设置。启用启动项会注册当前 EXE 的完整路径，
单 EXE 请放在最终位置后再启用；已有启动项在安装正式版时会自动接续到安装路径。

## 结构

| 文件 | 职责 |
| --- | --- |
| `ui/components.slint` | 主题、按钮、输入、开关、选择器、滚轮与时间弹层 |
| `ui/app.slint` | 主页面、闹钟编辑、提醒、确认与关于窗口 |
| `src/main.rs` | UI 与业务命令连接、视图模型更新、导入工作线程 |
| `src/engine.rs` | 倒计时与闹钟调度，不依赖 UI 类型 |
| `src/data.rs` | 校验、旧版数据迁移、原子保存 |
| `src/audio.rs` | 内置声音、Windows 播放、离线音频转换 |
| `src/native.rs` | Windows 托盘、热键、单实例、DWM、开机启动 |
| `src/i18n.rs`、`i18n/messages.json` | 统一三语文案、系统提示与日期；构建时生成 Slint 内置翻译表 |

## 构建

本次构建：Rust 1.99.0 / `x86_64-pc-windows-gnu`，w64devkit 2.10.1，Rust 自带 LLD。
本版安装包使用 w64devkit 随附 NSIS（版本标识 `11-Oct-2026.cvs`）；发布 EXE 使用 UPX 5.1.0。
Slint 1.18.1 需要 Rust 1.92 或更新。源码包含 `Cargo.lock`；首次构建需联网下载 Rust 依赖。

1. 安装 Rust 的 GNU x64 工具链，或 MSVC x64 工具链加 Visual Studio C++ Build Tools。
2. GNU 构建将 w64devkit `bin` 加到 `PATH`，提供 `windres` 与 `libshlwapi.a`。
3. 在源码根目录运行 `powershell -ExecutionPolicy Bypass -File .\build.ps1`。
4. 安装包：准备 NSIS 3.11+，运行 `powershell -ExecutionPolicy Bypass -File .\build.ps1 -Setup`。
   可用 `-NsisCompiler '完整路径\makensis.exe'` 指定官方便携编译器。
   安装器使用 zlib 压缩，用户安装时同样不依赖额外运行库。
5. 复现压缩发布版，在构建命令加 `-Upx '完整路径\upx.exe'`。
   使用官方 UPX 5.1.0，下载包 `upx-5.1.0-win64.zip` 的 SHA256 为
   `41c7492edeed0f7e67d347ca9e8d2131e2af7ca7a7695f92283a8e655c251c13`。

GNU 构建使用 Rust 自带的静态 CRT，只从 w64devkit 取 Windows shell 导入库，避免混用 CRT。
MSVC 构建同样通过 `+crt-static` 静态链接语言运行时。只发布 `dist` 的两个 EXE。

`cargo build --release --example render` 生成离屏 UI 排版渲染工具；
`target\release\examples\render.exe preview 0` 输出 BMP，不打开窗口、不读写用户资料。
第二个参数为语言：0 简体、1 繁体、2 英文；第三个参数 `light` 渲染浅色，默认深色。
这只是设计预览，不等同于对真实桌面交互的测试。

## 许可

应用代码 Apache-2.0。界面设计署名：Designed By MOTAN。
Slint 采用 Royalty-free Desktop, Mobile, and Web Applications License 2.0，
设置 → 关于中包含官方 `AboutSlint` 组件，请保留署名。

FFmpeg 为 LGPL 2.1+ 的音频构建。对应源代码、配置与脚本在 `third-party/FFmpeg`，
允许将兼容 FFmpeg 放到 EXE 旁 `codecs/ffmpeg.exe` 替换内置组件。
所有第三方资料可在设置 → 关于 → 第三方许可证查看，源码也包含其副本。
安装器使用 NSIS Modern UI 2，许可见 `licenses/NSIS.txt`；官方工具来源：
https://sourceforge.net/projects/nsis/files/NSIS%203/3.11/nsis-3.11.zip/download
沿用素材的 Apache 版权声明保留在 `licenses/Apache-2.0.txt`。

显示名称为 Timoa，沿用 `%APPDATA%\TIMO` 数据目录、JSON 格式与系统注册标识，保证从 TIMO 升级不丢数据。
