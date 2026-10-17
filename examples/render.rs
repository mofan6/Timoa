//! Offline UI artwork rendering. Does not open a desktop window or touch application data.
use slint::{ComponentHandle, ModelRc, VecModel};
use slint::platform::{Platform, WindowAdapter, software_renderer::{MinimalSoftwareWindow, RepaintBufferType}};
use std::{cell::RefCell, fs, io::Write, path::Path, rc::Rc};
#[allow(dead_code)]
#[path = "../src/i18n.rs"] mod i18n;
slint::include_modules!();

struct Canvas { windows: Rc<RefCell<Vec<Rc<MinimalSoftwareWindow>>>> }
impl Platform for Canvas {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        self.windows.borrow_mut().push(window.clone());
        Ok(window)
    }
}
fn save(window: &MinimalSoftwareWindow, path: &Path, width: u32, height: u32) {
    use slint::Rgb8Pixel;
    window.set_size(slint::PhysicalSize::new(width, height));
    slint::platform::update_timers_and_animations();
    window.request_redraw();
    let mut pixels = vec![Rgb8Pixel::default(); (width * height) as usize];
    window.draw_if_needed(|renderer| { renderer.render(&mut pixels, width as usize); });
    // Let decorative transitions settle for the exported artwork.
    std::thread::sleep(std::time::Duration::from_millis(170));
    slint::platform::update_timers_and_animations();
    window.request_redraw();
    window.draw_if_needed(|renderer| { renderer.render(&mut pixels, width as usize); });
    let stride = (width * 3 + 3) & !3;
    let mut bmp = Vec::new();
    bmp.extend(b"BM"); bmp.extend((54 + stride * height).to_le_bytes()); bmp.extend([0;4]); bmp.extend(54u32.to_le_bytes());
    bmp.extend(40u32.to_le_bytes()); bmp.extend(width.to_le_bytes()); bmp.extend(height.to_le_bytes());
    bmp.extend(1u16.to_le_bytes()); bmp.extend(24u16.to_le_bytes()); bmp.extend([0;24]);
    for y in (0..height).rev() {
        for x in 0..width { let p = pixels[(y * width + x) as usize]; bmp.extend([p.b, p.g, p.r]); }
        bmp.extend(vec![0; (stride - width * 3) as usize]);
    }
    fs::File::create(path).unwrap().write_all(&bmp).unwrap();
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::args().nth(1).unwrap_or("preview".into());
    let directory = Path::new(&directory); fs::create_dir_all(directory)?;
    let windows = Rc::new(RefCell::new(Vec::new()));
    slint::platform::set_platform(Box::new(Canvas { windows: windows.clone() }))?;
    let ui = MainWindow::new()?;
    let language = std::env::args().nth(2).and_then(|v| v.parse::<i32>().ok()).unwrap_or(0).clamp(0, 2);
    i18n::select(language); ui.set_language_index(language);
    let light = std::env::args().nth(3).is_some_and(|v| v == "light");
    ui.global::<Theme>().set_dark(!light); ui.set_theme_index(if light {1} else {0});
    ui.set_total_focus("126:45:30".into());
    ui.set_clock_text("09:41".into()); ui.set_date_text(format!("{} · {}", i18n::date(chrono::NaiveDate::from_ymd_opt(2026,10,15).unwrap()), i18n::weekday(3)).into());
    let sounds = || ModelRc::new(VecModel::from(["静音", "轻柔铃声", "马林巴琴", "钟声", "钢琴旋律", "电钢琴旋律", "铃声旋律", "自定义提示音"].iter().map(|s| i18n::t(s).into()).collect::<Vec<_>>()));
    ui.set_sounds(sounds()); ui.set_draft_sound(1);
    ui.set_sound_available(ModelRc::new(VecModel::from(vec![true; 10])));
    ui.set_custom_sounds(ModelRc::new(VecModel::from(vec![
        CustomSoundRow { id:7, name:"吉森信 - 橙色の時.flac".into(), available:true, menu_index:7 },
        CustomSoundRow { id:8, name:"轻柔雨声.mp3".into(), available:true, menu_index:8 },
        CustomSoundRow { id:9, name:"计划提醒.m4a".into(), available:false, menu_index:9 },
    ])));
    ui.set_timers(ModelRc::new(VecModel::from(vec![TimerRow { id:"a".into(), label:"英语阅读 · 第二篇".into(), remaining:"18:42".into(), status:i18n::t("进行中").into(), progress:0.252, paused:false, finished:false, waiting:false }])));
    let alarms = || ModelRc::new(VecModel::from(vec![AlarmRow { id:"b".into(), label:"数学真题 · 计时训练".into(), time:"14:00".into(), detail:format!("{}{}{}",i18n::t("工作日"),i18n::message(" · 至 16:00"),i18n::t(" · 提前提醒")).into(), meta:i18n::t("重复").into(), enabled:true }, AlarmRow { id:"c".into(), label:"英语复盘".into(), time:"19:30".into(), detail:i18n::t("每天").into(), meta:i18n::t("重复").into(), enabled:false } ]));
    ui.set_alarms(alarms()); ui.set_week_items(alarms()); ui.set_week_day(3);
    ui.set_week_title(if language == 2 { "Oct 12 — Oct 18".into() } else { "10月12日 — 10月18日".into() });
    ui.set_week_days(ModelRc::new(VecModel::from(["周一", "周二", "周三", "周四", "周五", "周六", "周日"].iter().enumerate().map(|(i,s)| DayRow { label:i18n::t(s).into(), date:(12+i).to_string().into(), count:if i==3 {i18n::t("今天").into()} else {i18n::message("2 项").into()}, today:i==3 }).collect::<Vec<_>>())));
    ui.set_next_time("14:00".into()); ui.set_next_label(format!("{} · 数学真题 · 计时训练",i18n::t("今天")).into());
    ui.show()?;
    for page in 0..4 {
        ui.set_page(page);
        save(&windows.borrow()[0], &directory.join(format!("page-{page}.bmp")), 480, 760);
    }
    ui.set_page(3); ui.set_settings_scroll_y(-480.);
    save(&windows.borrow()[0], &directory.join("sound-library.bmp"), 480, 760);
    ui.set_settings_scroll_y(0.);
    ui.set_page(0); save(&windows.borrow()[0], &directory.join("compact.bmp"), 420, 620);
    ui.set_page(1); ui.set_editor_visible(true);
    ui.set_editor_label("数学真题 · 计时训练".into()); ui.set_editor_hour(14); ui.set_editor_minute(30); ui.set_editor_date("2026-10-16".into());
    ui.set_editor_repeat_index(4); ui.set_editor_selected_days(ModelRc::new(VecModel::from(vec![true,true,true,true,true,false,false])));
    ui.set_editor_use_end(true); ui.set_editor_end_hour(16); ui.set_editor_end_minute(30); ui.set_editor_warn_before(true);
    save(&windows.borrow()[0], &directory.join("alarm-editor.bmp"), 480, 760);
    save(&windows.borrow()[0], &directory.join("alarm-editor-compact.bmp"), 420, 620);
    ui.set_editor_visible(false); ui.set_pending_delete_id("b".into());
    save(&windows.borrow()[0], &directory.join("alarm-delete.bmp"), 480, 760);
    ui.set_pending_delete_id("".into()); ui.set_alarms(ModelRc::default());
    save(&windows.borrow()[0], &directory.join("alarm-empty.bmp"), 480, 760);
    let about = AboutWindow::new()?; about.global::<Theme>().set_dark(!light); about.show()?;
    save(&windows.borrow()[1], &directory.join("about.bmp"), 400, 348);
    Ok(())
}
