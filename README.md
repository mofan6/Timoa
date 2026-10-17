<p align="center"><img src="assets/logo.svg" width="88" height="88" alt="Timoa icon" /></p>
<h1 align="center">Timoa</h1>
<p align="center">轻量级 Windows 倒计时、闹钟与专注计时工具<br/>A lightweight countdown timer, alarm clock and focus timer for Windows.</p>
<p align="center"><a href="README.en.md">English</a> · <a href="https://github.com/mofan6/Timoa/releases/latest">下载 Download</a> · <a href="https://github.com/mofan6/Timoa/issues">反馈 Issues</a></p>

![Timoa 界面预览：设置、专注计时、闹钟与每周计划](assets/screenshot.png)

## 下载与使用

提供 **Win10/11 64 位、Win7 SP1+ 64 位、Win7 SP1+ 32 位** 三种版本。无需另装 .NET、VC++、Java 或 WebView2 运行库。

当前版本为 **1.1.5**，更新内容见 [版本说明](RELEASE-NOTES.md)。

| 系统 | 单文件便携版 | 安装包 |
| --- | --- | --- |
| Windows 10 / 11 · 64 位 | [Timoa.exe](https://github.com/mofan6/Timoa/releases/download/v1.1.5/Timoa.exe) | [Timoa-Setup.exe](https://github.com/mofan6/Timoa/releases/download/v1.1.5/Timoa-Setup.exe) |
| Windows 7 SP1+ · 64 位 | [Win7-x64.exe](https://github.com/mofan6/Timoa/releases/download/v1.1.5/Timoa-1.1.5-Win7-x64.exe) | [Win7-x64-Setup.exe](https://github.com/mofan6/Timoa/releases/download/v1.1.5/Timoa-1.1.5-Win7-x64-Setup.exe) |
| Windows 7 SP1+ · 32 位 | [Win7-x86.exe](https://github.com/mofan6/Timoa/releases/download/v1.1.5/Timoa-1.1.5-Win7-x86.exe) | [Win7-x86-Setup.exe](https://github.com/mofan6/Timoa/releases/download/v1.1.5/Timoa-1.1.5-Win7-x86-Setup.exe) |

[完整源码包](https://github.com/mofan6/Timoa/releases/download/v1.1.5/Timoa-1.1.5-source.zip) 包含正常版与 Win7 独立源码、第三方许可和音频组件对应源码。Win7 源码也在独立的 [windows7 分支](https://github.com/mofan6/Timoa/tree/windows7) 维护。

Win7 包已完成编译和静态架构检查，尚未在 Windows 7 真机或虚拟机上验证；32 位版需要 SSE2。旧版本见 [GitHub Releases](https://github.com/mofan6/Timoa/releases)。

单 EXE 请先放在固定位置，再开启开机启动；移动程序后需要重新开启。

## 功能

- **刷题限时与专注计时**：5、10、25、45、60 分钟快捷时长，也可自定义到秒；支持多个计时、暂停、继续和重新开始。
- **手机式时间选择**：点击时间数字，用鼠标滚轮、上下拖动或方向键选择时间。
- **单一专注与正计时**：在更多设置中按需开启，默认关闭；正计时与倒计时自动互斥并保留进度。
- **精简悬浮窗**：显示当前计时与未来 30 分钟内的闹钟，支持置顶、拖动和从边缘调整大小。
- **闹钟与每周计划**：单次、每天、工作日、周末或自定义重复；支持结束时间和提前五分钟提醒。
- **提示音管理**：内置八段提示音，含《花火大会》《黄昏之时》，两段音乐均做 0.8 秒淡入淡出；可导入多段自定义音频，逐个试听、选择和删除。支持 MP3、FLAC、M4A、OGG、WAV 等常见非加密音频格式，离线转换。
- **自动循环与下一轮快捷时长**：可设置轮次间隔和三个快捷时长；倒计时及精简窗显示预计结束时间，完成提醒支持快捷开启下一轮。
- **每日与总专注时长**：一周页展示每日完成时长、轮次和小时分布，以每小时 60 分钟为满格；暂停时间不计入，原有累计总时长保留。
- **深浅色模式与三种语言**：简体中文、繁體中文、English，默认简体中文。
- **专注快捷键**：倒计时或正计时页面内，1 秒内连按三次 `M` 开始、继续或再开一轮；按 `N` 暂停。
- **桌面常驻**：托盘、`Ctrl + Alt + T` 快捷键、可选开机启动、JSON 备份导出与导入。

关闭主窗口后会继续在托盘中计时。右键托盘图标选择退出才会结束程序。
电脑关机、睡眠或程序退出时不能响铃。

## 数据与备份

- JSON 备份包含计划、设置、下一轮快捷时长、循环偏好和专注统计，**不包含自定义音频**。
- 恢复备份前会保存当前数据到 `data-before-import.json`。
- 数据目录沿用 `%APPDATA%\TIMO`，以便从 TIMO 升级后继续使用原来的数据。
- 自定义音频位于上述目录中的 `sounds` 文件夹；需要时可单独备份。
- 音频解码缓存位于 `%LOCALAPPDATA%\TIMO\codecs`；首次导入音频无需联网。
- 卸载保留用户数据。请先从托盘退出旧版，再使用新版。

## 构建与许可

使用 **Rust + Slint 1.18.1**，原生编译并静态链接语言运行时。
兼容目标的编译与打包方法见 [COMPATIBILITY.md](COMPATIBILITY.md)，一般项目结构见 [开发文档](docs/BUILDING.zh-CN.md)。
发布前对备份及开机启动的代码审查记录见 [代码审查](docs/代码审查.md)；该记录不代表实际功能验证通过。

应用代码采用 [Apache-2.0](LICENSE)。第三方许可及素材版权声明见 [NOTICE.txt](NOTICE.txt) 和 [licenses](licenses)。
FFmpeg 音频组件的对应源码与构建说明包含在 [third-party/FFmpeg](third-party/FFmpeg)。

## 搜索关键词 / Keywords

倒计时、正计时、计时器、电脑闹钟、桌面闹钟、专注计时、悬浮计时、学习计时、刷题限时、考研、提醒、时间管理、轻量软件；
Windows countdown timer, stopwatch, desktop alarm clock, focus timer, floating timer, study timer, exam preparation, productivity, time management, lightweight, portable, multilingual.

## 致谢 / Acknowledgements

感谢 [@malinfossum](https://github.com/malinfossum) 的 [Tidsro](https://github.com/malinfossum/tidsro) 项目：Timoa 的最初灵感与六段基础铃声来自这个项目。

Thank you to [@malinfossum](https://github.com/malinfossum) and [Tidsro](https://github.com/malinfossum/tidsro) for the initial inspiration and the six original chimes.

<p align="center">Designed By MOTAN</p>
