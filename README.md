<p align="center"><img src="assets/logo.svg" width="88" height="88" alt="Timoa icon" /></p>
<h1 align="center">Timoa</h1>
<p align="center">轻量级 Windows 倒计时、闹钟与专注计时工具<br/>A lightweight countdown timer, alarm clock and focus timer for Windows.</p>
<p align="center"><a href="README.en.md">English</a> · <a href="https://github.com/mofan6/Timoa/releases/latest">下载 Download</a> · <a href="https://github.com/mofan6/Timoa/issues">反馈 Issues</a></p>

![Timoa 界面预览：设置、专注计时、闹钟与每周计划](assets/screenshot.png)

## 下载与使用

支持 **Windows 10 / 11，64 位**。无需另装 .NET、VC++、Java 或 WebView2 运行库。

本源码版本为 **1.1.0**；打包文件如下，更新内容见 [版本说明](RELEASE-NOTES.md)。

| 文件 | 用途 | 大小 |
| --- | --- | --- |
| [Timoa.exe](https://github.com/mofan6/Timoa/releases/download/v1.1.0/Timoa.exe) | 单文件便携版，直接运行 | 8.63 MiB |
| [Timoa-Setup.exe](https://github.com/mofan6/Timoa/releases/download/v1.1.0/Timoa-Setup.exe) | 安装版，安装到当前用户目录 | 8.92 MiB |
| [完整源码包](https://github.com/mofan6/Timoa/releases/download/v1.1.0/Timoa-1.1.0-source.zip) | 含第三方许可和音频解码组件对应源码 | — |

已公开发布的版本可从 [GitHub Releases](https://github.com/mofan6/Timoa/releases) 下载。

单 EXE 请先放在固定位置，再开启开机启动；移动程序后需要重新开启。

## 功能

- **刷题限时与专注计时**：5、10、25、45、60 分钟快捷时长，也可自定义到秒；支持多个计时、暂停、继续和重新开始。
- **手机式时间选择**：点击时间数字，用鼠标滚轮、上下拖动或方向键选择时间。
- **单一专注与正计时**：在更多设置中按需开启，默认关闭；正计时与倒计时自动互斥并保留进度。
- **精简悬浮窗**：显示当前计时与未来 30 分钟内的闹钟，支持置顶、拖动和从边缘调整大小。
- **闹钟与每周计划**：单次、每天、工作日、周末或自定义重复；支持结束时间和提前五分钟提醒。
- **提示音管理**：内置六段铃声；可导入多段自定义音频，逐个试听、选择和删除。支持 MP3、FLAC、M4A、OGG、WAV 等常见非加密音频格式，离线转换。
- **总专注时长**：累计已完成的专注计时，删除完成记录后仍保留累计值。
- **深浅色模式与三种语言**：简体中文、繁體中文、English，默认简体中文。
- **桌面常驻**：托盘、`Ctrl + Alt + T` 快捷键、可选开机启动、JSON 备份导出与导入。

关闭主窗口后会继续在托盘中计时。右键托盘图标选择退出才会结束程序。
电脑关机、睡眠或程序退出时不能响铃。

## 数据与备份

- JSON 备份包含计划、设置和专注累计值，**不包含自定义音频**。
- 恢复备份前会保存当前数据到 `data-before-import.json`。
- 数据目录沿用 `%APPDATA%\TIMO`，以便从 TIMO 升级后继续使用原来的数据。
- 自定义音频位于上述目录中的 `sounds` 文件夹；需要时可单独备份。
- 音频解码缓存位于 `%LOCALAPPDATA%\TIMO\codecs`；首次导入音频无需联网。
- 卸载保留用户数据。请先从托盘退出旧版，再使用新版。

## 构建与许可

使用 **Rust + Slint 1.18.1**，原生编译并静态链接语言运行时。
构建方法、项目结构和体积说明见 [开发文档](docs/BUILDING.zh-CN.md)。
发布前对备份及开机启动的代码审查记录见 [代码审查](docs/代码审查.md)；该记录不代表实际功能验证通过。

应用代码采用 [Apache-2.0](LICENSE)。第三方许可及素材版权声明见 [NOTICE.txt](NOTICE.txt) 和 [licenses](licenses)。
FFmpeg 音频组件的对应源码与构建说明包含在 [third-party/FFmpeg](third-party/FFmpeg)。

## 搜索关键词 / Keywords

倒计时、正计时、计时器、电脑闹钟、桌面闹钟、专注计时、悬浮计时、学习计时、刷题限时、考研、提醒、时间管理、轻量软件；
Windows countdown timer, stopwatch, desktop alarm clock, focus timer, floating timer, study timer, exam preparation, productivity, time management, lightweight, portable, multilingual.

## 致谢 / Acknowledgements

感谢 [@malinfossum](https://github.com/malinfossum) 的 [Tidsro](https://github.com/malinfossum/tidsro) 项目：Timoa 的最初灵感与内置铃声来自这个项目。

Thank you to [@malinfossum](https://github.com/malinfossum) and [Tidsro](https://github.com/malinfossum/tidsro) for the initial inspiration and the built-in chimes.

<p align="center">Designed By MOTAN</p>
