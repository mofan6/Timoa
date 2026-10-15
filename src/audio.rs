//! Embedded chimes and an offline audio-only decoder. No installed codecs are required.
use crate::data::{self, Folders};
use flate2::read::GzDecoder;
use std::{fs, io::{Cursor, Read}, path::{Path, PathBuf}, process::{Command, Stdio},
    sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc}, thread, time::{Duration, Instant}};
use std::os::windows::process::CommandExt;

const DECODER: &[u8] = include_bytes!("../assets/audio-tools.zip");
const MAX_BYTES: u64 = 512 * 1024 * 1024;
pub const NAMES: [&str; 8] = ["静音", "轻柔铃声", "马林巴琴", "钟声", "钢琴旋律", "电钢琴旋律", "铃声旋律", "自定义提示音"];
const WAVS: [&[u8]; 6] = [
    include_bytes!("../assets/sounds/soft-chime.wav.gz"),
    include_bytes!("../assets/sounds/marimba.wav.gz"),
    include_bytes!("../assets/sounds/bell.wav.gz"),
    include_bytes!("../assets/sounds/Piano-Jingle.wav.gz"),
    include_bytes!("../assets/sounds/Electric-Piano-Jingle.wav.gz"),
    include_bytes!("../assets/sounds/Bell-Jingle.wav.gz"),
];

#[derive(Default)]
pub struct Player { data: Vec<u8> }
impl Player {
    pub fn play(&mut self, choice: i32, folders: &Folders) -> Result<(), String> {
        self.stop();
        if choice == 0 { return Ok(()); }
        let mut bytes = Vec::new();
        if choice >= 7 {
            bytes = fs::read(folders.custom_sound(choice)).map_err(|_| "请先在设置中导入自定义提示音。")?;
        } else if (1..=6).contains(&choice) {
            GzDecoder::new(WAVS[choice as usize - 1]).read_to_end(&mut bytes).map_err(|_| "无法读取内置提示音。")?;
        }
        if bytes.len() < 44 { return Err("提示音内容为空或已损坏。".into()); }
        self.data = bytes;
        let played = unsafe { windows_sys::Win32::Media::Audio::PlaySoundW(self.data.as_ptr().cast(), std::ptr::null_mut(), 0x1 | 0x4 | 0x2) };
        if played == 0 { Err("Windows 无法播放这段提示音，请重新导入。".into()) } else { Ok(()) }
    }
    pub fn duration(&self) -> Duration {
        let mut position = 12usize; let mut rate = 0u32; let mut length = 0u32;
        while position + 8 <= self.data.len() {
            let size = u32::from_le_bytes(self.data[position+4..position+8].try_into().unwrap()) as usize;
            let start = position + 8;
            if size > self.data.len().saturating_sub(start) { break; }
            if &self.data[position..position+4] == b"fmt " && size >= 12 { rate = u32::from_le_bytes(self.data[start+8..start+12].try_into().unwrap()); }
            if &self.data[position..position+4] == b"data" { length = size as u32; }
            position = start + size + (size & 1);
        }
        if rate > 0 && length > 0 { Duration::from_secs_f64(length as f64 / rate as f64) } else { Duration::from_secs(5) }
    }
    pub fn stop(&mut self) {
        unsafe { windows_sys::Win32::Media::Audio::PlaySoundW(std::ptr::null(), std::ptr::null_mut(), 0); }
        self.data.clear();
    }
}
impl Drop for Player { fn drop(&mut self) { self.stop(); } }

pub struct ImportResult { pub id: i32, pub name: String, pub notice: String }
pub struct ImportJob {
    pub cancel: Arc<AtomicBool>,
    pub receiver: mpsc::Receiver<Result<ImportResult, String>>,
    pub started: Instant,
}
pub fn start_import(source: PathBuf, folders: Folders, id: i32) -> ImportJob {
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = cancel.clone();
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || { let result = import(&source, &folders, &flag, id); let _ = sender.send(result); });
    ImportJob { cancel, receiver, started: Instant::now() }
}

fn decoder(folders: &Folders, cancel: &AtomicBool) -> Result<PathBuf, String> {
    if let Ok(exe) = std::env::current_exe() {
        let replacement = exe.parent().unwrap_or(Path::new(".")).join("codecs/ffmpeg.exe");
        if replacement.is_file() { return Ok(replacement); }
    }
    let target = folders.local.join("codecs/audio-20261015");
    let executable = target.join("ffmpeg.exe");
    if executable.is_file() && target.join("complete").is_file() { return Ok(executable); }
    let staging = folders.local.join(format!("codecs/import-{}", data::new_id()));
    fs::create_dir_all(&staging).map_err(|e| format!("无法准备音频组件：{e}"))?;
    let result = (|| {
        let mut archive = zip::ZipArchive::new(Cursor::new(DECODER)).map_err(|_| "内置音频组件损坏，请重新下载程序。")?;
        for index in 0..archive.len() {
            if cancel.load(Ordering::Relaxed) { return Err("已取消导入。".into()); }
            let mut entry = archive.by_index(index).map_err(|e| e.to_string())?;
            let safe_name = entry.enclosed_name().filter(|p| p.components().count() == 1)
                .ok_or("音频组件路径无效。")?.to_owned();
            let mut output = fs::File::create(staging.join(safe_name)).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut output).map_err(|e| e.to_string())?;
        }
        fs::write(staging.join("complete"), b"TIMO audio-only 20261015").map_err(|e| e.to_string())?;
        if target.exists() { fs::remove_dir_all(&target).map_err(|e| e.to_string())?; }
        fs::rename(&staging, &target).map_err(|e| e.to_string())?;
        Ok(executable)
    })();
    if result.is_err() { let _ = fs::remove_dir_all(staging); }
    result
}

fn import(source: &Path, folders: &Folders, cancel: &AtomicBool, id: i32) -> Result<ImportResult, String> {
    let metadata = fs::metadata(source).map_err(|_| "找不到所选音频，请重新选择。")?;
    if metadata.len() == 0 || metadata.len() > MAX_BYTES { return Err("请选择非空、大小不超过 512 MB 的音频文件。".into()); }
    let executable = decoder(folders, cancel)?;
    fs::create_dir_all(folders.roaming.join("sounds")).map_err(|e| format!("无法创建提示音目录：{e}"))?;
    let staging = folders.roaming.join(format!("sounds/import-{}.tmp", data::new_id()));
    let result = (|| {
        let mut child = Command::new(executable)
            .args(["-nostdin", "-hide_banner", "-loglevel", "error", "-y", "-threads", "2",
                "-protocol_whitelist", "file,pipe", "-i"])
            .arg(source)
            .args(["-map", "0:a:0", "-vn", "-sn", "-dn", "-map_metadata", "-1",
                "-c:a", "pcm_s16le", "-ar", "44100", "-ac", "2", "-fs", "536870912", "-f", "wav"])
            .arg(&staging).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped())
            .creation_flags(0x08000000).spawn().map_err(|e| format!("无法启动内置音频组件：{e}"))?;
        let mut stderr = child.stderr.take().unwrap();
        let diagnostics = thread::spawn(move || {
            let mut tail = Vec::new(); let mut buffer = [0u8; 4096];
            while let Ok(count) = stderr.read(&mut buffer) {
                if count == 0 { break; }
                tail.extend_from_slice(&buffer[..count]);
                if tail.len() > 8192 { tail.drain(..tail.len() - 8192); }
            }
            String::from_utf8_lossy(&tail).into_owned()
        });
        let start = Instant::now();
        let mut cancelled = false;
        let mut timed_out = false;
        let status = loop {
            if cancel.load(Ordering::Relaxed) || start.elapsed() > Duration::from_secs(300) {
                cancelled = cancel.load(Ordering::Relaxed); timed_out = !cancelled;
                let _ = child.kill(); break child.wait().map_err(|e| e.to_string())?;
            }
            if let Some(status) = child.try_wait().map_err(|e| e.to_string())? { break status; }
            thread::sleep(Duration::from_millis(40));
        };
        let error_text = diagnostics.join().unwrap_or_default();
        folders.log(&format!("Audio import {}: {status}\n{error_text}", source.file_name().unwrap_or_default().to_string_lossy()));
        if cancelled || cancel.load(Ordering::Relaxed) { return Err("已取消导入。".into()); }
        if timed_out { return Err("转换超过 5 分钟，已停止。请先剪辑需要用作铃声的片段。".into()); }
        if !status.success() {
            let ext = source.extension().unwrap_or_default().to_string_lossy().to_lowercase();
            let protected = ["ncm", "mflac", "mflac0", "mgg", "mgg1", "qmcflac", "qmcogg", "qmc0", "qmc3", "kgm", "kgma", "kwm"].contains(&ext.as_str());
            return Err(if protected { "这是音乐平台的加密下载文件，请使用可通用播放的 MP3、FLAC 等文件。".into() }
                else { "无法解码这个文件，可能已严重损坏或使用不支持的编码。详细信息已保存到日志。".into() });
        }
        let size = fs::metadata(&staging).map_err(|_| "没有生成可播放音频。")?.len();
        if size <= 78 { return Err("所选文件没有可播放的音轨。".into()); }
        if size >= MAX_BYTES - 65536 { return Err("转换后的音频超过 512 MB，请先剪辑较短的铃声片段。".into()); }
        if cancel.load(Ordering::Relaxed) { return Err("已取消导入。".into()); }
        data::replace(&staging, &folders.custom_sound(id))?;
        Ok(ImportResult { id, name: source.file_name().unwrap_or_default().to_string_lossy().chars().take(200).collect(),
            notice: if error_text.trim().is_empty() { "已导入，可点击试听。".into() }
                else { "已导入。文件含可恢复的格式错误，请试听确认。".into() } })
    })();
    if staging.exists() { let _ = fs::remove_file(staging); }
    result
}
