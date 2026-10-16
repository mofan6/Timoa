<p align="center"><img src="assets/logo.svg" width="88" height="88" alt="Timoa icon" /></p>
<h1 align="center">Timoa</h1>
<p align="center">A lightweight countdown timer, alarm clock and focus timer for Windows.</p>
<p align="center"><a href="README.md">简体中文</a> · <a href="https://github.com/mofan6/Timoa/releases/latest">Download</a> · <a href="https://github.com/mofan6/Timoa/issues">Issues</a></p>

![Timoa interface: settings, focus timer, alarms and weekly schedule](assets/screenshot.png)

## Download

**Windows 10 / 11, x64.** No separate .NET, VC++, Java or WebView2 runtime installation is required.

This source is version **1.1.0**. See [release notes](RELEASE-NOTES.md).

- [Timoa.exe](https://github.com/mofan6/Timoa/releases/download/v1.1.0/Timoa.exe): portable single EXE, **8.63 MiB**.
- [Timoa-Setup.exe](https://github.com/mofan6/Timoa/releases/download/v1.1.0/Timoa-Setup.exe): per-user installer, **8.92 MiB**.
- [Complete source archive](https://github.com/mofan6/Timoa/releases/download/v1.1.0/Timoa-1.1.0-source.zip), including third-party notices and the corresponding source for the audio decoder.

Publicly released versions are available from [GitHub Releases](https://github.com/mofan6/Timoa/releases).

Keep the portable EXE in a permanent location before enabling startup at sign-in. Re-enable startup if you move it.

## Features

- Multiple countdowns with pause, resume and restart; 5, 10, 25, 45 and 60-minute presets and custom durations down to seconds.
- Phone-style time wheels controlled with the mouse wheel, dragging or arrow keys.
- Optional Single focus and Stopwatch modes, both off by default. Starting or resuming either stopwatch or countdown pauses the other and preserves progress.
- A resizable, always-on-top mini window showing the active timer and alarms due within 30 minutes.
- One-time and recurring alarms, weekday/weekend/custom schedules, end times and optional five-minute warnings.
- Six built-in chimes and multiple custom sounds with individual preview and removal. Import common unencrypted MP3, FLAC, M4A, OGG, WAV and other audio files with offline conversion.
- Total completed focus time that remains counted after completed records are removed.
- Dark/light themes and three embedded languages: Simplified Chinese, Traditional Chinese and English. Simplified Chinese is the default.
- System tray, `Ctrl + Alt + T`, optional startup at sign-in, and JSON backup/restore.

Closing the window keeps timers running in the tray. Use the tray menu to exit the application.
Alarms cannot sound while the computer is shut down, asleep or the application is not running.

## Data and backups

JSON backups contain plans, settings and focus statistics. **Custom audio files are not included.** A pre-import snapshot is saved as `data-before-import.json` before restoring a backup.

For compatibility with earlier TIMO versions, user data remains in `%APPDATA%\TIMO`. Custom audio is stored in its `sounds` folder and may be backed up separately. Decoder files are cached in `%LOCALAPPDATA%\TIMO\codecs`; importing audio does not require a network connection.

Uninstallation preserves user data. Exit an older version from the tray before running the new one.

## Build and licenses

Built with Rust and Slint 1.18.1, with a statically linked language runtime. See [the build documentation](docs/BUILDING.zh-CN.md) for toolchain and packaging details.

Application code is licensed under [Apache-2.0](LICENSE). See [NOTICE.txt](NOTICE.txt), [licenses](licenses) and [third-party/FFmpeg](third-party/FFmpeg) for third-party notices and corresponding decoder source.

Backup and Windows startup paths were reviewed in source code before release. They were not verified by actually restoring backups, changing startup settings, installing/uninstalling or restarting Windows.

## Keywords

Windows countdown timer, alarm clock, desktop timer, focus timer, study timer, exam preparation, productivity, time management, lightweight, portable, multilingual.

倒计时、计时器、电脑闹钟、专注计时、学习计时、刷题限时、考研、提醒、时间管理。

## Acknowledgements

Thank you to [@malinfossum](https://github.com/malinfossum) and [Tidsro](https://github.com/malinfossum/tidsro) for the initial inspiration and the built-in chimes.

感谢 [@malinfossum](https://github.com/malinfossum) 的 Tidsro 项目，提供了 Timoa 最初的灵感与内置铃声。

<p align="center">Designed By MOTAN</p>
