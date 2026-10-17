# Timoa 1.1.5 · 2026-10-17

- 新增 Windows 7 SP1+ 64 位与 32 位兼容包，正常版继续面向 Win10/11 64 位。
- 专注页面支持 1 秒内三次 M 开始、继续或再开一轮，N 暂停。
- 倒计时卡片和精简悬浮窗显示预计结束时间。
- 自动循环倒计时，可用分钟:秒滚轮设置轮次间隔；00:00 按 1 秒处理。
- 三个可自定义的下一轮快捷时长，结束提醒保留“再来一轮”和“我知道了”，布局更紧凑。
- 一周页面增加每日专注时长、完成轮次和小时柱状图；使用 0、30、60 分钟固定刻度，悬浮查看具体时长。
- 更多设置增加易懂的悬浮说明，覆盖深浅色和简体、繁体、英文。
- 新增内置音乐《花火大会》《黄昏之时》，均带 0.8 秒淡入、0.8 秒淡出，可完整试听。
- 新增设置和每日统计随 JSON 备份保存；自定义音频仍不包含在备份中。旧累计值保留，每日明细从支持此功能的版本开始记录。

English: Windows 7 x64/x86 packages, focus keyboard shortcuts, expected finish times, automatic countdown loops, three next-round presets, compact reminders, daily/hourly focus charts with a fixed 60-minute scale, clearer tooltips, and two new built-in music clips with 0.8-second fades. New preferences and statistics are included in JSON backups; custom audio files are not.

感谢 [@malinfossum](https://github.com/malinfossum) 的 [Tidsro](https://github.com/malinfossum/tidsro) 提供最初灵感与六段基础铃声。

Designed By MOTAN

---

# Timoa V1.1.0

- 新增单一专注：设置与倒计时合并到首页卡片，提供进度、暂停、重置与移除。
- 新增正计时：支持暂停、继续、重置和完成本轮；完成后计入总专注时长。
- 正计时与倒计时互斥，开始或继续一项会自动暂停另一项并保留进度。
- 新增精简悬浮窗：显示当前计时和未来 30 分钟内的闹钟，支持置顶、拖动、边缘缩放与关闭到托盘。
- 设置新增“更多设置”，单一专注与正计时默认关闭，兼容已有数据与备份。
- 试听按钮改为圆形播放/暂停图标样式。
- 浅色计时卡片与其他卡片统一背景，页面内图标随主题切换；深色卡片与程序图标保持原样。
- 放大一周页面的日期切换箭头。
- 操作提示在 10 秒后自动消失。
- 新增界面覆盖简体中文、繁体中文、英文和深浅色主题。

## Timoa V1.0.1

音频选项增加独立试听按钮和悬浮反馈；闹钟新建/编辑、删除确认在页面内完成；
空状态主按钮居中；滚动条留出独立空间；统一按钮选中、按下与焦点处理；
新增可保存的深浅色模式、总专注时长；更新金黑色图标；矢量渲染平滑细线和圆角。

自定义提示音可连续添加多段，设置中独立试听和删除；删除声音后相关计划改为静音。
底部移除快捷键和托盘说明，只保留设置页的 Designed By MOTAN。

用户检查后调整：选择控件禁止焦点层拦截第一次点击，选项提交在弹层关闭前完成；
主题与语言合并为并排控件，菜单仅在需要滚动时保留滚动条空间；
累计专注时长居中，声音面板随内容高度收缩，主按钮明确使用微软雅黑 UI 常规字重。

导航按钮使用持续的选中底色，避免按下、悬浮和鼠标焦点在切页时反复切换色值。
底部署名抵消滚动区域左右内边距差，按完整窗口中心对齐。

总专注时长累计每轮自然完成的设定时长，删除完成记录后保留，清除计划或恢复设置不清零。
首次升级补入现存的已完成记录，之前已经删除的记录无法恢复。

针对偶发首启底部透明条增加初始重绘处理，因尚未复现，效果需在实际使用中确认。

发布前从代码与逻辑层面审查 JSON 备份和开机启动，修正导入设置应用、导入写盘错误处理、
启动路径核对、Windows 禁用状态识别与安装器启动项迁移。未实际操作这些功能进行验证。

## 致谢 / Acknowledgements

感谢 [@malinfossum](https://github.com/malinfossum) 的 [Tidsro](https://github.com/malinfossum/tidsro) 项目，提供 Timoa 最初的灵感与内置铃声。

Thank you to @malinfossum and Tidsro for the initial inspiration and the built-in chimes.
