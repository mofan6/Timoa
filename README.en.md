<p align="center"><img src="assets/logo.svg" width="88" height="88" alt="Timoa icon" /></p>
<h1 align="center">Timoa</h1>
<p align="center">A lightweight countdown timer, alarm clock and focus timer for Windows.</p>
<p align="center"><a href="README.md">简体中文</a> · <a href="https://github.com/mofan6/Timoa/releases/latest">Download</a> · <a href="https://github.com/mofan6/Timoa/issues">Issues</a></p>

![Timoa interface: settings, focus timer, alarms and weekly schedule](assets/screenshot.png)

## Download

Version **1.1.5**. One source tree provides x64 and x86 builds with the same features and declarative UI.

| Architecture | Portable EXE | Installer |
| --- | --- | --- |
| x64 | [Timoa-1.1.5-windows-x64.exe](https://github.com/mofan6/Timoa/releases/download/v1.1.5/Timoa-1.1.5-windows-x64.exe) | [Timoa-1.1.5-windows-x64-setup.exe](https://github.com/mofan6/Timoa/releases/download/v1.1.5/Timoa-1.1.5-windows-x64-setup.exe) |
| x86 | [Timoa-1.1.5-windows-x86.exe](https://github.com/mofan6/Timoa/releases/download/v1.1.5/Timoa-1.1.5-windows-x86.exe) | [Timoa-1.1.5-windows-x86-setup.exe](https://github.com/mofan6/Timoa/releases/download/v1.1.5/Timoa-1.1.5-windows-x86-setup.exe) |

Targets Windows 7 SP1, 8, 8.1, 10 and 11. x86 requires SSE2 and also runs through WOW64 on x64 Windows. Windows 11 has no x86 OS edition. No separate .NET, VC++, Java or WebView2 runtime installation is required.

See [WINDOWS-SUPPORT.md](WINDOWS-SUPPORT.md) for actual verification coverage and runtime capability fallbacks. Systems that have not been run are not described as verified. The source ZIP contains one self-contained `Timoa/` tree for both architectures, licenses and corresponding decoder source. Earlier normal and Windows 7 source snapshots remain preserved.

Keep the portable EXE in a permanent location before enabling startup. Exit an older instance from the tray before upgrading.

## Features

- Multiple countdowns with pause, resume and restart; 5, 10, 25, 45 and 60-minute presets and custom durations down to seconds.
- Phone-style time wheels controlled with the mouse wheel, dragging or arrow keys.
- Optional Single focus and Stopwatch modes, both off by default. Starting or resuming either stopwatch or countdown pauses the other and preserves progress.
- A resizable, always-on-top mini window showing the active timer and alarms due within 30 minutes.
- One-time and recurring alarms, weekday/weekend/custom schedules, end times and optional five-minute warnings.
- Eight built-in sounds, including Fireworks Festival and At Twilight with 0.8-second fade-ins and fade-outs, and multiple custom sounds with individual preview and removal. Import common unencrypted MP3, FLAC, M4A, OGG, WAV and other audio files with offline conversion.
- Automatic countdown loops with adjustable breaks and three customizable next-round presets. Expected finish time is shown in countdown cards and the mini window.
- Daily completed focus duration, rounds and hourly charts with a fixed 60-minute scale, plus persistent lifetime totals. Paused time is excluded.
- Dark/light themes and three embedded languages: Simplified Chinese, Traditional Chinese and English. Simplified Chinese is the default.
- Focus-page shortcuts: tap `M` three times within one second to start, resume or begin another round; press `N` to pause.
- System tray, `Ctrl + Alt + T`, optional startup at sign-in, and JSON backup/restore.

Closing the window keeps timers running in the tray. Use the tray menu to exit the application.
Alarms cannot sound while the computer is shut down, asleep or the application is not running.

## Data and backups

JSON backups contain plans, settings and focus statistics. **Custom audio files are not included.** A pre-import snapshot is saved as `data-before-import.json` before restoring a backup.

For compatibility with earlier TIMO versions, user data remains in `%APPDATA%\TIMO`. Custom audio is stored in its `sounds` folder and may be backed up separately. Decoder files are cached in `%LOCALAPPDATA%\TIMO\codecs`; importing audio does not require a network connection.

Uninstallation preserves user data. Exit an older version from the tray before running the new one.

## Build and licenses

Built with Rust and Slint 1.18.1, with a statically linked language runtime. See [Windows support and builds](WINDOWS-SUPPORT.md) for the current toolchain and packaging instructions; [the development documentation](docs/BUILDING.zh-CN.md) preserves project structure and historical notes.

Application code is licensed under [Apache-2.0](LICENSE). See [NOTICE.txt](NOTICE.txt), [licenses](licenses) and [third-party/FFmpeg](third-party/FFmpeg) for third-party notices and corresponding decoder source.

Backup and Windows startup paths were reviewed in source code before release. They were not verified by actually restoring backups, changing startup settings, installing/uninstalling or restarting Windows.

## Keywords

Windows countdown timer, alarm clock, desktop timer, focus timer, study timer, exam preparation, productivity, time management, lightweight, portable, multilingual.

倒计时、计时器、电脑闹钟、专注计时、学习计时、刷题限时、考研、提醒、时间管理。

## Acknowledgements

Thank you to [@malinfossum](https://github.com/malinfossum) and [Tidsro](https://github.com/malinfossum/tidsro) for the initial inspiration and the six original chimes.

感谢 [@malinfossum](https://github.com/malinfossum) 的 Tidsro 项目，提供了 Timoa 最初的灵感与六段基础铃声。

<p align="center">Designed By MOTAN</p>
