use anyhow::{Context, Result};
use rusqlite::{params, Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub struct KeyboardStore {
    pub db_path: PathBuf,
}

static CAPTURE_ACTIVE: AtomicBool = AtomicBool::new(false);
static HOOK_SENDER: OnceLock<Sender<KeyEvent>> = OnceLock::new();
static PRESSED: Mutex<Option<HashSet<u32>>> = Mutex::new(None);

const WH_KEYBOARD_LL: i32 = 13;
const HC_ACTION: i32 = 0;
const WM_KEYDOWN: u32 = 0x0100;
const WM_KEYUP: u32 = 0x0101;
const WM_SYSKEYDOWN: u32 = 0x0104;
const WM_SYSKEYUP: u32 = 0x0105;
const WM_TIMER: u32 = 0x0113;
const LLKHF_INJECTED: u32 = 0x10;
const VK_PACKET: u32 = 0xE7;
const VK_PROCESSKEY: u32 = 0xE5;
const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
const ACTIVITY_WINDOW_MS: i64 = 5_000;
const FLUSH_INTERVAL: Duration = Duration::from_secs(2);
const FLUSH_BATCH: usize = 500;

thread_local! {
    static APP_CACHE: RefCell<Option<(isize, Instant, Option<String>)>> = const { RefCell::new(None) };
}

type HHOOK = *mut c_void;
type HWND = *mut c_void;
type HINSTANCE = *mut c_void;
type HANDLE = *mut c_void;

#[repr(C)]
struct Point {
    x: i32,
    y: i32,
}

#[repr(C)]
struct Msg {
    hwnd: HWND,
    message: u32,
    wparam: usize,
    lparam: isize,
    time: u32,
    pt: Point,
    private: u32,
}

#[repr(C)]
struct KbdLlHookStruct {
    vk_code: u32,
    scan_code: u32,
    flags: u32,
    time: u32,
    extra_info: usize,
}

#[link(name = "user32")]
extern "system" {
    fn SetWindowsHookExW(
        id_hook: i32,
        lpfn: Option<unsafe extern "system" fn(i32, usize, isize) -> isize>,
        hmod: HINSTANCE,
        thread_id: u32,
    ) -> HHOOK;
    fn UnhookWindowsHookEx(hook: HHOOK) -> i32;
    fn CallNextHookEx(hook: HHOOK, code: i32, wparam: usize, lparam: isize) -> isize;
    fn GetMessageW(msg: *mut Msg, hwnd: HWND, min: u32, max: u32) -> i32;
    fn TranslateMessage(msg: *const Msg) -> i32;
    fn DispatchMessageW(msg: *const Msg) -> isize;
    fn SetTimer(hwnd: HWND, id: usize, elapse: u32, proc: *mut c_void) -> usize;
    fn GetForegroundWindow() -> HWND;
    fn GetWindowThreadProcessId(hwnd: HWND, process_id: *mut u32) -> u32;
    fn GetAsyncKeyState(vk: i32) -> i16;
}

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(name: *const u16) -> HINSTANCE;
    fn OpenProcess(access: u32, inherit: i32, process_id: u32) -> HANDLE;
    fn QueryFullProcessImageNameW(
        process: HANDLE,
        flags: u32,
        name: *mut u16,
        size: *mut u32,
    ) -> i32;
    fn CloseHandle(handle: HANDLE) -> i32;
}

struct KeyEvent {
    ts_ms: i64,
    keycode: u32,
    down: bool,
    at: Instant,
    app: Option<String>,
}

pub fn start(db_path: PathBuf) -> Result<()> {
    init_db(&db_path)?;

    let (sender, receiver) = mpsc::channel::<KeyEvent>();
    let writer_path = db_path.clone();
    std::thread::Builder::new()
        .name("toccata-keyboard-writer".into())
        .spawn(move || writer_main(writer_path, receiver))
        .context("无法启动键盘写入线程")?;

    let _ = HOOK_SENDER.set(sender.clone());
    std::thread::Builder::new()
        .name("toccata-keyboard-hook".into())
        .spawn(move || hook_main(sender))
        .context("无法启动键盘监听线程")?;

    Ok(())
}

fn init_db(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)
        .with_context(|| format!("无法打开键盘数据库 {}", path.display()))?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         CREATE TABLE IF NOT EXISTS key_events (
           ts_ms   INTEGER NOT NULL,
           keycode INTEGER NOT NULL,
           kind    INTEGER NOT NULL,
           hold_ms INTEGER,
           app     TEXT
         );
         CREATE INDEX IF NOT EXISTS idx_key_events_ts ON key_events(ts_ms);
         CREATE TABLE IF NOT EXISTS app_sessions (
           app       TEXT NOT NULL,
           start_ms  INTEGER NOT NULL,
           end_ms    INTEGER NOT NULL,
           key_count INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_app_sessions_start ON app_sessions(start_ms);",
    )?;
    Ok(())
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

fn pressed() -> std::sync::MutexGuard<'static, Option<HashSet<u32>>> {
    PRESSED.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn writer_main(path: PathBuf, receiver: Receiver<KeyEvent>) {
    let mut conn = match Connection::open(&path) {
        Ok(conn) => conn,
        Err(error) => {
            eprintln!("[toccata] 无法打开键盘数据库：{error}");
            return;
        }
    };
    let _ = conn.execute_batch("PRAGMA journal_mode=WAL;");

    let mut open_keys: HashMap<u32, Instant> = HashMap::new();
    let mut buffer: Vec<(i64, u32, i64, Option<i64>, Option<String>)> = Vec::new();
    let mut next_flush = Instant::now() + FLUSH_INTERVAL;

    loop {
        let wait = next_flush.saturating_duration_since(Instant::now());
        match receiver.recv_timeout(wait) {
            Ok(event) => {
                if event.down {
                    if open_keys.contains_key(&event.keycode) {
                        continue;
                    }
                    open_keys.insert(event.keycode, event.at);
                    buffer.push((event.ts_ms, event.keycode, 0, None, event.app));
                } else {
                    let hold = open_keys
                        .remove(&event.keycode)
                        .map(|start| event.at.saturating_duration_since(start).as_millis() as i64);
                    buffer.push((event.ts_ms, event.keycode, 1, hold, event.app));
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                flush(&mut conn, &mut buffer);
                return;
            }
        }

        if buffer.len() >= FLUSH_BATCH || Instant::now() >= next_flush {
            flush(&mut conn, &mut buffer);
            next_flush = Instant::now() + FLUSH_INTERVAL;
        }
    }
}

fn flush(conn: &mut Connection, buffer: &mut Vec<(i64, u32, i64, Option<i64>, Option<String>)>) {
    if buffer.is_empty() {
        return;
    }
    let result = (|| -> Result<()> {
        let transaction = conn.transaction()?;
        {
            let mut statement = transaction.prepare(
                "INSERT INTO key_events (ts_ms, keycode, kind, hold_ms, app)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;
            for (ts, keycode, kind, hold, app) in buffer.iter() {
                statement.execute(params![ts, i64::from(*keycode), kind, hold, app])?;
            }
        }
        transaction.commit()?;
        Ok(())
    })();
    if result.is_ok() {
        buffer.clear();
    } else if let Err(error) = result {
        eprintln!("[toccata] 键盘数据写入失败：{error}");
    }
}

fn hook_main(sender: Sender<KeyEvent>) {
    unsafe {
        *pressed() = Some(HashSet::new());
        let module = GetModuleHandleW(std::ptr::null());
        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), module, 0);
        if hook.is_null() {
            eprintln!("[toccata] 键盘钩子安装失败");
            CAPTURE_ACTIVE.store(false, Ordering::SeqCst);
            return;
        }
        CAPTURE_ACTIVE.store(true, Ordering::SeqCst);
        SetTimer(null_mut(), 1, 5_000, null_mut());

        let mut message: Msg = std::mem::zeroed();
        while GetMessageW(&mut message, null_mut(), 0, 0) > 0 {
            if message.message == WM_TIMER {
                close_stale_keys(&sender);
                continue;
            }
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        UnhookWindowsHookEx(hook);
        CAPTURE_ACTIVE.store(false, Ordering::SeqCst);
    }
}

unsafe extern "system" fn hook_proc(code: i32, wparam: usize, lparam: isize) -> isize {
    if code == HC_ACTION {
        let keyboard = &*(lparam as *const KbdLlHookStruct);
        let message = wparam as u32;
        let down = matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN);
        let up = matches!(message, WM_KEYUP | WM_SYSKEYUP);
        let vk = keyboard.vk_code;

        if (down || up)
            && keyboard.flags & LLKHF_INJECTED == 0
            && vk != VK_PACKET
            && vk != VK_PROCESSKEY
        {
            let should_emit = {
                let mut guard = pressed();
                let set = guard.get_or_insert_with(HashSet::new);
                if down {
                    set.insert(vk)
                } else {
                    set.remove(&vk);
                    true
                }
            };

            if should_emit {
                if let Some(sender) = HOOK_SENDER.get() {
                    let _ = sender.send(KeyEvent {
                        ts_ms: now_ms(),
                        keycode: vk,
                        down,
                        at: Instant::now(),
                        app: foreground_app(),
                    });
                }
            }
        }
    }
    CallNextHookEx(null_mut(), code, wparam, lparam)
}

fn close_stale_keys(sender: &Sender<KeyEvent>) {
    let stale: Vec<u32> = {
        let guard = pressed();
        match guard.as_ref() {
            Some(set) => set
                .iter()
                .copied()
                .filter(|vk| unsafe { (GetAsyncKeyState(*vk as i32) as u16) & 0x8000 == 0 })
                .collect(),
            None => Vec::new(),
        }
    };
    if stale.is_empty() {
        return;
    }
    {
        let mut guard = pressed();
        if let Some(set) = guard.as_mut() {
            for vk in &stale {
                set.remove(vk);
            }
        }
    }
    for vk in stale {
        let _ = sender.send(KeyEvent {
            ts_ms: now_ms(),
            keycode: vk,
            down: false,
            at: Instant::now(),
            app: foreground_app(),
        });
    }
}

fn foreground_app() -> Option<String> {
    unsafe {
        let window = GetForegroundWindow();
        APP_CACHE.with(|cache| {
            if let Some((cached_window, at, name)) = cache.borrow().as_ref() {
                if *cached_window == window as isize && at.elapsed() < Duration::from_millis(500) {
                    return name.clone();
                }
            }
            let name = query_foreground_app(window);
            *cache.borrow_mut() = Some((window as isize, Instant::now(), name.clone()));
            name
        })
    }
}

unsafe fn query_foreground_app(window: HWND) -> Option<String> {
    if window.is_null() {
        return None;
    }
    let mut process_id: u32 = 0;
    GetWindowThreadProcessId(window, &mut process_id);
    if process_id == 0 {
        return None;
    }
    let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id);
    if handle.is_null() {
        return None;
    }
    let mut buffer = [0u16; 1024];
    let mut size = buffer.len() as u32;
    let ok = QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut size);
    CloseHandle(handle);
    if ok == 0 || size == 0 {
        return None;
    }
    let path = String::from_utf16_lossy(&buffer[..size as usize]);
    Path::new(&path)
        .file_name()
        .map(|name| name.to_string_lossy().to_lowercase())
}

pub fn capture_status() -> bool {
    CAPTURE_ACTIVE.load(Ordering::SeqCst)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DayRange {
    pub start_ms: i64,
    pub end_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bucket {
    pub start_ms: i64,
    pub end_ms: i64,
    pub active_ms: i64,
    pub key_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayStat {
    pub start_ms: i64,
    pub end_ms: i64,
    pub active_ms: i64,
    pub key_count: i64,
    pub buckets: Vec<Bucket>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStat {
    pub app: String,
    pub active_ms: i64,
    pub key_count: i64,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub active_ms: i64,
    pub key_count: i64,
    pub key_counts: HashMap<i64, i64>,
    pub days: Vec<DayStat>,
    pub apps: Vec<AppStat>,
}

struct EventRow {
    ts_ms: i64,
    keycode: i64,
    kind: i64,
    hold_ms: Option<i64>,
    app: Option<String>,
}

#[derive(Clone)]
struct Span {
    start: i64,
    end: i64,
    app: Option<String>,
}

pub fn snapshot(store: &KeyboardStore, mut days: Vec<DayRange>, now_ms: i64) -> Snapshot {
    days.retain(|day| day.end_ms > day.start_ms);
    days.sort_by_key(|day| day.start_ms);
    if days.is_empty() {
        return Snapshot::default();
    }
    let now_ms = if now_ms > 0 { now_ms } else { now_ms_now() };
    let window_start = days.first().map(|day| day.start_ms).unwrap_or(0);
    let window_end = days
        .iter()
        .map(|day| day.end_ms)
        .max()
        .unwrap_or(window_start)
        .min(now_ms);

    let rows = match read_events(&store.db_path, window_start.saturating_sub(3_600_000), now_ms) {
        Ok(rows) => rows,
        Err(_) => return Snapshot::default(),
    };

    let mut spans: Vec<Span> = Vec::new();
    let mut key_counts: HashMap<i64, i64> = HashMap::new();
    let mut app_spans: HashMap<String, Vec<Span>> = HashMap::new();
    let mut app_keys: HashMap<String, i64> = HashMap::new();

    for row in &rows {
        let span = if row.kind == 0 {
            Some(Span {
                start: row.ts_ms,
                end: row.ts_ms.saturating_add(ACTIVITY_WINDOW_MS),
                app: row.app.clone(),
            })
        } else {
            row.hold_ms
                .filter(|hold| *hold > 0)
                .map(|hold| Span {
                    start: row.ts_ms.saturating_sub(hold),
                    end: row.ts_ms,
                    app: row.app.clone(),
                })
        };

        if let Some(span) = span {
            let app = span
                .app
                .clone()
                .unwrap_or_else(|| "未知应用".to_string());
            app_spans.entry(app).or_default().push(span.clone());
            spans.push(span);
        }

        if row.kind == 0 && row.ts_ms >= window_start && row.ts_ms < window_end {
            *key_counts.entry(row.keycode).or_insert(0) += 1;
            let app = row
                .app
                .clone()
                .unwrap_or_else(|| "未知应用".to_string());
            *app_keys.entry(app).or_insert(0) += 1;
        }
    }

    let merged = merge_spans(spans);
    let active_ms = merged
        .iter()
        .map(|span| overlap(span.start, span.end, window_start, window_end))
        .sum();

    let mut day_stats = Vec::new();
    for day in &days {
        let start = day.start_ms.max(window_start);
        let end = day.end_ms.min(window_end);
        let day_active: i64 = merged
            .iter()
            .map(|span| overlap(span.start, span.end, start, end))
            .sum();
        let mut buckets = Vec::new();
        let mut bucket_start = start;
        while bucket_start < end {
            let bucket_end = (bucket_start + 900_000).min(end);
            let bucket_active: i64 = merged
                .iter()
                .map(|span| overlap(span.start, span.end, bucket_start, bucket_end))
                .sum();
            let bucket_keys = rows
                .iter()
                .filter(|row| {
                    row.kind == 0
                        && row.ts_ms >= bucket_start
                        && row.ts_ms < bucket_end
                })
                .count() as i64;
            buckets.push(Bucket {
                start_ms: bucket_start,
                end_ms: bucket_end,
                active_ms: bucket_active,
                key_count: bucket_keys,
            });
            bucket_start = bucket_end;
        }
        let day_keys = rows
            .iter()
            .filter(|row| row.kind == 0 && row.ts_ms >= start && row.ts_ms < end)
            .count() as i64;
        day_stats.push(DayStat {
            start_ms: day.start_ms,
            end_ms: day.end_ms,
            active_ms: day_active,
            key_count: day_keys,
            buckets,
        });
    }

    let mut apps: Vec<AppStat> = app_spans
        .into_iter()
        .map(|(app, spans)| {
            let merged = merge_spans(spans);
            let active = merged
                .iter()
                .map(|span| overlap(span.start, span.end, window_start, window_end))
                .sum();
            AppStat {
                key_count: *app_keys.get(&app).unwrap_or(&0),
                app,
                active_ms: active,
            }
        })
        .filter(|stat| stat.active_ms > 0 || stat.key_count > 0)
        .collect();
    apps.sort_by(|left, right| {
        right
            .active_ms
            .cmp(&left.active_ms)
            .then_with(|| right.key_count.cmp(&left.key_count))
    });

    let key_count: i64 = key_counts.values().sum();
    Snapshot {
        active_ms,
        key_count,
        key_counts,
        days: day_stats,
        apps,
    }
}

fn now_ms_now() -> i64 {
    now_ms()
}

fn overlap(start: i64, end: i64, range_start: i64, range_end: i64) -> i64 {
    (end.min(range_end) - start.max(range_start)).max(0)
}

fn merge_spans(mut spans: Vec<Span>) -> Vec<Span> {
    spans.retain(|span| span.end > span.start);
    spans.sort_by_key(|span| span.start);
    let mut merged: Vec<Span> = Vec::new();
    for span in spans {
        if let Some(last) = merged.last_mut() {
            if span.start <= last.end {
                last.end = last.end.max(span.end);
                continue;
            }
        }
        merged.push(span);
    }
    merged
}

fn read_events(path: &Path, start_ms: i64, end_ms: i64) -> Result<Vec<EventRow>> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut statement = conn.prepare(
        "SELECT ts_ms, keycode, kind, hold_ms, app
         FROM key_events
         WHERE ts_ms >= ?1 AND ts_ms <= ?2
         ORDER BY ts_ms, rowid",
    )?;
    let rows = statement.query_map(params![start_ms, end_ms], |row| {
        Ok(EventRow {
            ts_ms: row.get(0)?,
            keycode: row.get(1)?,
            kind: row.get(2)?,
            hold_ms: row.get(3)?,
            app: row.get(4)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyCount {
    pub day: String,
    pub key_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(start: i64, end: i64) -> Span {
        Span {
            start,
            end,
            app: None,
        }
    }

    #[test]
    fn overlapping_spans_are_merged_once() {
        let merged = merge_spans(vec![span(0, 5), span(3, 8), span(10, 12)]);
        assert_eq!(merged.len(), 2);
        assert_eq!((merged[0].start, merged[0].end), (0, 8));
        assert_eq!((merged[1].start, merged[1].end), (10, 12));
    }

    #[test]
    fn overlap_is_clipped_to_both_ranges() {
        assert_eq!(overlap(0, 10, 5, 8), 3);
        assert_eq!(overlap(0, 5, 5, 10), 0);
        assert_eq!(overlap(8, 12, 0, 10), 2);
    }

    #[test]
    fn empty_and_reversed_spans_are_ignored() {
        let merged = merge_spans(vec![span(5, 5), span(8, 2), span(1, 3)]);
        assert_eq!(merged.len(), 1);
        assert_eq!((merged[0].start, merged[0].end), (1, 3));
    }
}

pub fn daily_counts(store: &KeyboardStore, start_ms: i64, end_ms: i64) -> Vec<DailyCount> {
    let conn = match Connection::open_with_flags(
        &store.db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    ) {
        Ok(conn) => conn,
        Err(_) => return Vec::new(),
    };
    let mut statement = match conn.prepare(
        "SELECT date(ts_ms / 1000, 'unixepoch', 'localtime') AS day, COUNT(*)
         FROM key_events
         WHERE kind = 0 AND ts_ms >= ?1 AND ts_ms < ?2
         GROUP BY day
         ORDER BY day",
    ) {
        Ok(statement) => statement,
        Err(_) => return Vec::new(),
    };
    let rows = statement.query_map(params![start_ms, end_ms], |row| {
        Ok(DailyCount {
            day: row.get(0)?,
            key_count: row.get(1)?,
        })
    });
    match rows {
        Ok(rows) => rows.filter_map(|row| row.ok()).collect(),
        Err(_) => Vec::new(),
    }
}
