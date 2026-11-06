use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use windows::core::{Interface, IUnknown};
use windows::Win32::Foundation::HWND as UiaHwnd;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};
use windows::Win32::System::Variant::{VARIANT, VT_I4};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationValuePattern, TreeScope_Descendants,
    UIA_ControlTypePropertyId, UIA_EditControlTypeId, UIA_ValuePatternId,
};

pub struct KeyboardStore {
    pub db_path: PathBuf,
}

static CAPTURE_ACTIVE: AtomicBool = AtomicBool::new(false);
static HOOK_SENDER: OnceLock<Sender<CaptureEvent>> = OnceLock::new();
static PRESSED: Mutex<Option<HashSet<u32>>> = Mutex::new(None);

const WH_KEYBOARD_LL: i32 = 13;
const WH_MOUSE_LL: i32 = 14;
const HC_ACTION: i32 = 0;
const WM_KEYDOWN: u32 = 0x0100;
const WM_KEYUP: u32 = 0x0101;
const WM_SYSKEYDOWN: u32 = 0x0104;
const WM_SYSKEYUP: u32 = 0x0105;
const WM_TIMER: u32 = 0x0113;
const WM_LBUTTONDOWN: u32 = 0x0201;
const WM_RBUTTONDOWN: u32 = 0x0204;
const LLKHF_INJECTED: u32 = 0x10;
const LLMHF_INJECTED: u32 = 0x01;
const VK_PACKET: u32 = 0xE7;
const VK_PROCESSKEY: u32 = 0xE5;
const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
const ACTIVITY_WINDOW_MS: i64 = 5_000;
const FLUSH_INTERVAL: Duration = Duration::from_secs(2);
const FLUSH_BATCH: usize = 500;

#[derive(Clone, Default)]
struct ForegroundContext {
    app: Option<String>,
    title: Option<String>,
    url: Option<String>,
}

static CONTEXT: Mutex<Option<ForegroundContext>> = Mutex::new(None);

const BROWSER_EXES: &[&str] = &[
    "chrome.exe",
    "msedge.exe",
    "brave.exe",
    "vivaldi.exe",
    "opera.exe",
    "firefox.exe",
];

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

#[repr(C)]
struct MsllHookStruct {
    pt: Point,
    mouse_data: u32,
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
    fn GetWindowTextW(hwnd: HWND, text: *mut u16, max: i32) -> i32;
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
    title: Option<String>,
    url: Option<String>,
}

struct MouseEvent {
    ts_ms: i64,
    button: i64,
    app: Option<String>,
    title: Option<String>,
    url: Option<String>,
}

enum CaptureEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
}

pub fn start(db_path: PathBuf) -> Result<()> {
    init_db(&db_path)?;

    let (sender, receiver) = mpsc::channel::<CaptureEvent>();
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

    std::thread::Builder::new()
        .name("toccata-context-sampler".into())
        .spawn(context_sampler)
        .context("无法启动窗口上下文线程")?;

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
           app     TEXT,
           title   TEXT,
           url     TEXT
         );
         CREATE INDEX IF NOT EXISTS idx_key_events_ts ON key_events(ts_ms);
         CREATE TABLE IF NOT EXISTS mouse_events (
           ts_ms  INTEGER NOT NULL,
           button INTEGER NOT NULL,
           app    TEXT,
           title  TEXT,
           url    TEXT
         );
         CREATE INDEX IF NOT EXISTS idx_mouse_events_ts ON mouse_events(ts_ms);
         CREATE TABLE IF NOT EXISTS site_aliases (
           domain     TEXT PRIMARY KEY NOT NULL,
           label      TEXT NOT NULL,
           updated_ms INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS app_sessions (
           app       TEXT NOT NULL,
           start_ms  INTEGER NOT NULL,
           end_ms    INTEGER NOT NULL,
           key_count INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_app_sessions_start ON app_sessions(start_ms);",
    )?;
    for (table, column) in [
        ("key_events", "title"),
        ("key_events", "url"),
        ("mouse_events", "title"),
        ("mouse_events", "url"),
    ] {
        ensure_column(&conn, table, column)?;
    }
    Ok(())
}

fn ensure_column(conn: &Connection, table: &str, column: &str) -> Result<()> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == column {
            return Ok(());
        }
    }
    conn.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {column} TEXT;"))?;
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

fn writer_main(path: PathBuf, receiver: Receiver<CaptureEvent>) {
    let mut conn = match Connection::open(&path) {
        Ok(conn) => conn,
        Err(error) => {
            eprintln!("[toccata] 无法打开键盘数据库：{error}");
            return;
        }
    };
    let _ = conn.execute_batch("PRAGMA journal_mode=WAL;");

    let mut open_keys: HashMap<u32, Instant> = HashMap::new();
    let mut key_buffer: Vec<(
        i64,
        u32,
        i64,
        Option<i64>,
        Option<String>,
        Option<String>,
        Option<String>,
    )> = Vec::new();
    let mut mouse_buffer: Vec<(
        i64,
        i64,
        Option<String>,
        Option<String>,
        Option<String>,
    )> = Vec::new();
    let mut next_flush = Instant::now() + FLUSH_INTERVAL;

    loop {
        let wait = next_flush.saturating_duration_since(Instant::now());
        match receiver.recv_timeout(wait) {
            Ok(CaptureEvent::Key(event)) => {
                if event.down {
                    if open_keys.contains_key(&event.keycode) {
                        continue;
                    }
                    open_keys.insert(event.keycode, event.at);
                    key_buffer.push((
                        event.ts_ms,
                        event.keycode,
                        0,
                        None,
                        event.app,
                        event.title,
                        event.url,
                    ));
                } else {
                    let hold = open_keys
                        .remove(&event.keycode)
                        .map(|start| event.at.saturating_duration_since(start).as_millis() as i64);
                    key_buffer.push((
                        event.ts_ms,
                        event.keycode,
                        1,
                        hold,
                        event.app,
                        event.title,
                        event.url,
                    ));
                }
            }
            Ok(CaptureEvent::Mouse(event)) => {
                mouse_buffer.push((
                    event.ts_ms,
                    event.button,
                    event.app,
                    event.title,
                    event.url,
                ));
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                flush(&mut conn, &mut key_buffer, &mut mouse_buffer);
                return;
            }
        }

        if key_buffer.len() + mouse_buffer.len() >= FLUSH_BATCH
            || Instant::now() >= next_flush
        {
            flush(&mut conn, &mut key_buffer, &mut mouse_buffer);
            next_flush = Instant::now() + FLUSH_INTERVAL;
        }
    }
}

fn flush(
    conn: &mut Connection,
    key_buffer: &mut Vec<(
        i64,
        u32,
        i64,
        Option<i64>,
        Option<String>,
        Option<String>,
        Option<String>,
    )>,
    mouse_buffer: &mut Vec<(
        i64,
        i64,
        Option<String>,
        Option<String>,
        Option<String>,
    )>,
) {
    if key_buffer.is_empty() && mouse_buffer.is_empty() {
        return;
    }
    let result = (|| -> Result<()> {
        let transaction = conn.transaction()?;
        {
            let mut statement = transaction.prepare(
                "INSERT INTO key_events (ts_ms, keycode, kind, hold_ms, app, title, url)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )?;
            for (ts, keycode, kind, hold, app, title, url) in key_buffer.iter() {
                statement.execute(params![
                    ts,
                    i64::from(*keycode),
                    kind,
                    hold,
                    app,
                    title,
                    url
                ])?;
            }
        }
        {
            let mut statement = transaction.prepare(
                "INSERT INTO mouse_events (ts_ms, button, app, title, url)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;
            for (ts, button, app, title, url) in mouse_buffer.iter() {
                statement.execute(params![ts, button, app, title, url])?;
            }
        }
        transaction.commit()?;
        Ok(())
    })();
    if result.is_ok() {
        key_buffer.clear();
        mouse_buffer.clear();
    } else if let Err(error) = result {
        eprintln!("[toccata] 键盘数据写入失败：{error}");
    }
}

fn hook_main(sender: Sender<CaptureEvent>) {
    unsafe {
        *pressed() = Some(HashSet::new());
        let module = GetModuleHandleW(std::ptr::null());
        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), module, 0);
        if hook.is_null() {
            eprintln!("[toccata] 键盘钩子安装失败");
            CAPTURE_ACTIVE.store(false, Ordering::SeqCst);
            return;
        }
        let mouse_hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), module, 0);
        if mouse_hook.is_null() {
            eprintln!("[toccata] 鼠标钩子安装失败，鼠标点击不会记录");
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
        if !mouse_hook.is_null() {
            UnhookWindowsHookEx(mouse_hook);
        }
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
                    let context = current_context();
                    let _ = sender.send(CaptureEvent::Key(KeyEvent {
                        ts_ms: now_ms(),
                        keycode: vk,
                        down,
                        at: Instant::now(),
                        app: context.app,
                        title: context.title,
                        url: context.url,
                    }));
                }
            }
        }
    }
    CallNextHookEx(null_mut(), code, wparam, lparam)
}

unsafe extern "system" fn mouse_hook_proc(code: i32, wparam: usize, lparam: isize) -> isize {
    if code == HC_ACTION {
        let message = wparam as u32;
        if message == WM_LBUTTONDOWN || message == WM_RBUTTONDOWN {
            let mouse = &*(lparam as *const MsllHookStruct);
            if mouse.flags & LLMHF_INJECTED == 0 {
                if let Some(sender) = HOOK_SENDER.get() {
                    let context = current_context();
                    let _ = sender.send(CaptureEvent::Mouse(MouseEvent {
                        ts_ms: now_ms(),
                        button: if message == WM_LBUTTONDOWN { 0 } else { 1 },
                        app: context.app,
                        title: context.title,
                        url: context.url,
                    }));
                }
            }
        }
    }
    CallNextHookEx(null_mut(), code, wparam, lparam)
}

fn close_stale_keys(sender: &Sender<CaptureEvent>) {
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
        let context = current_context();
        let _ = sender.send(CaptureEvent::Key(KeyEvent {
            ts_ms: now_ms(),
            keycode: vk,
            down: false,
            at: Instant::now(),
            app: context.app,
            title: context.title,
            url: context.url,
        }));
    }
}

fn current_context() -> ForegroundContext {
    CONTEXT
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
        .unwrap_or_default()
}

fn context_sampler() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let automation: Option<IUIAutomation> = unsafe {
        CoCreateInstance(&CUIAutomation, None::<&IUnknown>, CLSCTX_INPROC_SERVER).ok()
    };
    let mut cached_window: isize = 0;
    let mut cached_url: Option<String> = None;
    let mut last_url_check = Instant::now() - Duration::from_secs(5);

    loop {
        let (app, title, url) = unsafe {
            let window = GetForegroundWindow();
            let app = query_foreground_app(window);
            let title = window_title(window);
            let browser = app
                .as_deref()
                .map(|name| BROWSER_EXES.contains(&name))
                .unwrap_or(false);
            let mut url = None;
            if browser {
                if window as isize != cached_window
                    || last_url_check.elapsed() >= Duration::from_millis(1_500)
                {
                    cached_url = automation
                        .as_ref()
                        .and_then(|automation| browser_url(automation, window));
                    cached_window = window as isize;
                    last_url_check = Instant::now();
                }
                url = cached_url.clone();
            } else {
                cached_window = 0;
                cached_url = None;
            }
            (app, title, url)
        };

        *CONTEXT
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(ForegroundContext {
            app,
            title,
            url,
        });
        std::thread::sleep(Duration::from_millis(700));
    }
}

unsafe fn window_title(window: HWND) -> Option<String> {
    if window.is_null() {
        return None;
    }
    let mut buffer = [0u16; 512];
    let length = GetWindowTextW(window, buffer.as_mut_ptr(), buffer.len() as i32);
    if length <= 0 {
        None
    } else {
        Some(String::from_utf16_lossy(&buffer[..length as usize]))
    }
}

fn variant_i4(value: i32) -> VARIANT {
    let mut variant = VARIANT::default();
    unsafe {
        let inner = &mut *variant.Anonymous.Anonymous;
        inner.vt = VT_I4;
        inner.Anonymous.lVal = value;
    }
    variant
}

unsafe fn browser_url(automation: &IUIAutomation, window: HWND) -> Option<String> {
    if window.is_null() {
        return None;
    }
    let control_type = variant_i4(UIA_EditControlTypeId.0);
    let condition = automation
        .CreatePropertyCondition(UIA_ControlTypePropertyId, &control_type)
        .ok()?;
    let root = automation.ElementFromHandle(UiaHwnd(window)).ok()?;
    let edits = root.FindAll(TreeScope_Descendants, &condition).ok()?;
    let length = edits.Length().ok()?;
    let mut address_candidate = None;
    for index in 0..length {
        let Ok(element) = edits.GetElement(index) else {
            continue;
        };
        let name = element
            .CurrentName()
            .map(|value| value.to_string())
            .unwrap_or_default();
        let Ok(pattern) = element.GetCurrentPattern(UIA_ValuePatternId) else {
            continue;
        };
        let Ok(value_pattern) = pattern.cast::<IUIAutomationValuePattern>() else {
            continue;
        };
        let Ok(value) = value_pattern.CurrentValue() else {
            continue;
        };
        let value = value.to_string();
        if value.starts_with("http://") || value.starts_with("https://") {
            return Some(value);
        }
        if address_candidate.is_none()
            && (name.to_lowercase().contains("address") || name.contains("地址"))
            && !value.is_empty()
        {
            address_candidate = Some(value);
        }
    }
    address_candidate
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
    pub label: String,
    pub active_ms: i64,
    pub key_count: i64,
    pub parent: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub active_ms: i64,
    pub key_count: i64,
    pub mouse_left: i64,
    pub mouse_right: i64,
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
    title: Option<String>,
    url: Option<String>,
}

struct Classification {
    parent_key: String,
    parent_label: String,
    key: String,
    label: String,
}

fn classify_context(
    app: Option<&str>,
    url: Option<&str>,
    title: Option<&str>,
    aliases: &HashMap<String, String>,
) -> Classification {
    let app = app.unwrap_or("").trim().to_lowercase();
    let app_key = if app.is_empty() {
        "app:unknown".to_string()
    } else {
        format!("app:{app}")
    };
    let app_display = app_label(&app, title);

    if BROWSER_EXES.contains(&app.as_str()) {
        if let Some(host) = url.as_deref().and_then(url_host) {
            let (key, label) = if let Some((key, label)) = site_label(&host) {
                (format!("site:{key}"), label.to_string())
            } else if let Some(label) = alias_for_host(&host, aliases) {
                (format!("web:{host}"), label.to_string())
            } else {
                (format!("web:{host}"), host)
            };
            return Classification {
                parent_key: app_key,
                parent_label: app_display,
                key,
                label,
            };
        }
    }

    Classification {
        parent_key: app_key.clone(),
        parent_label: app_display.clone(),
        key: app_key,
        label: app_display,
    }
}

fn alias_for_host<'a>(
    host: &str,
    aliases: &'a HashMap<String, String>,
) -> Option<&'a str> {
    let mut candidate = host;
    loop {
        if let Some(label) = aliases.get(candidate) {
            return Some(label.as_str());
        }
        match candidate.find('.') {
            Some(index) => candidate = &candidate[index + 1..],
            None => return None,
        }
    }
}

const DEEPSEEK_ENDPOINT: &str = "https://api.deepseek.com/chat/completions";
const DEEPSEEK_MODEL: &str = "deepseek-flash";

fn deepseek_key() -> Result<String> {
    if let Ok(value) = std::env::var("DEEPSEEK_API_KEY") {
        let value = value.trim();
        if value.starts_with("sk-") {
            return Ok(value.to_string());
        }
    }
    let appdata = std::env::var("APPDATA").context("无法定位 APPDATA")?;
    let path = Path::new(&appdata)
        .join("xiaoman")
        .join("Toccata")
        .join("cookie")
        .join("deepseek.txt");
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("无法读取 DeepSeek Key：{}", path.display()))?;
    let key = content
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("");
    if !key.starts_with("sk-") {
        bail!("DeepSeek Key 格式不正确");
    }
    Ok(key.to_string())
}

fn collect_unknown_domains(path: &Path, aliases: &HashMap<String, String>) -> Result<Vec<String>> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let since = now_ms() - 182 * 86_400_000;
    let mut statement = conn.prepare(
        "SELECT url FROM key_events WHERE url IS NOT NULL AND ts_ms >= ?1
         UNION
         SELECT url FROM mouse_events WHERE url IS NOT NULL AND ts_ms >= ?1
         LIMIT 50000",
    )?;
    let mut rows = statement.query(params![since])?;
    let mut domains = BTreeSet::new();
    while let Some(row) = rows.next()? {
        let url: String = row.get(0)?;
        let Some(host) = url_host(&url) else {
            continue;
        };
        if host == "localhost" || host.starts_with("127.") {
            continue;
        }
        if site_label(&host).is_some() || alias_for_host(&host, aliases).is_some() {
            continue;
        }
        domains.insert(host);
        if domains.len() >= 100 {
            break;
        }
    }
    Ok(domains.into_iter().collect())
}

async fn request_ai_grouping(key: &str, domains: &[String]) -> Result<String> {
    let prompt = format!(
        "请把下面的网站域名归类为简洁的站点名称。相同产品或服务的域名必须使用同一个名称，\
         例如 chat.deepseek.com 和 platform.deepseek.com 都归为 DeepSeek，chatgpt.com 归为 ChatGPT，\
         bilibili.com 归为哔哩哔哩。名称优先使用官方中文名或通用品牌名，未知站点可使用域名主体。\
         只输出一个 JSON 对象，键为输入中的域名，值为站点名称；不要 Markdown、不要解释。\n域名列表：{}",
        serde_json::to_string(domains)?
    );
    let body = serde_json::json!({
        "model": DEEPSEEK_MODEL,
        "messages": [
            { "role": "system", "content": "你是网站归类助手，只输出合法 JSON。" },
            { "role": "user", "content": prompt }
        ],
        "temperature": 0.1,
        "max_tokens": 2000
    });
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?;
    let response = client
        .post(DEEPSEEK_ENDPOINT)
        .bearer_auth(key)
        .json(&body)
        .send()
        .await?;
    let status = response.status();
    let text = response.text().await?;
    if !status.is_success() {
        let preview: String = text.chars().take(300).collect();
        bail!("DeepSeek API {status}: {preview}");
    }
    let payload: serde_json::Value =
        serde_json::from_str(&text).context("无法解析 DeepSeek 响应")?;
    let content = payload["choices"][0]["message"]["content"]
        .as_str()
        .context("DeepSeek 响应缺少内容")?;
    Ok(content.to_string())
}

fn parse_ai_mapping(content: &str, requested: &[String]) -> Result<HashMap<String, String>> {
    let start = content.find('{').context("AI 未返回 JSON 对象")?;
    let end = content.rfind('}').context("AI 未返回 JSON 对象")?;
    if end < start {
        bail!("AI 返回的 JSON 不完整");
    }
    let value: serde_json::Value = serde_json::from_str(&content[start..=end])
        .context("AI 返回的 JSON 无法解析")?;
    let object = value.as_object().context("AI 返回结果不是对象")?;
    let allowed: HashSet<&str> = requested.iter().map(String::as_str).collect();
    let mut mapping = HashMap::new();
    for (domain, label) in object {
        let domain = domain
            .trim()
            .trim_start_matches("www.")
            .to_lowercase();
        if !allowed.contains(domain.as_str()) {
            continue;
        }
        let Some(label) = label.as_str() else {
            continue;
        };
        let label: String = label
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(40)
            .collect();
        if !label.is_empty() {
            mapping.insert(domain, label);
        }
    }
    if mapping.is_empty() {
        bail!("AI 没有返回可用的站点归类");
    }
    Ok(mapping)
}

fn write_site_aliases(path: &Path, mapping: &HashMap<String, String>) -> Result<usize> {
    let mut conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    let transaction = conn.transaction()?;
    let updated = now_ms();
    {
        let mut statement = transaction.prepare(
            "INSERT INTO site_aliases (domain, label, updated_ms) VALUES (?1, ?2, ?3)
             ON CONFLICT(domain) DO UPDATE SET label = excluded.label,
                                              updated_ms = excluded.updated_ms",
        )?;
        for (domain, label) in mapping {
            statement.execute(params![domain, label, updated])?;
        }
    }
    transaction.commit()?;
    Ok(mapping.len())
}

pub async fn ai_group_sites(db_path: PathBuf) -> Result<usize> {
    let aliases = read_site_aliases(&db_path);
    let domains = collect_unknown_domains(&db_path, &aliases)?;
    if domains.is_empty() {
        return Ok(0);
    }
    let key = deepseek_key()?;
    let content = request_ai_grouping(&key, &domains).await?;
    let mapping = parse_ai_mapping(&content, &domains)?;
    write_site_aliases(&db_path, &mapping)
}

fn read_site_aliases(path: &Path) -> HashMap<String, String> {
    let conn = match Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(conn) => conn,
        Err(_) => return HashMap::new(),
    };
    let mut statement = match conn.prepare("SELECT domain, label FROM site_aliases") {
        Ok(statement) => statement,
        Err(_) => return HashMap::new(),
    };
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    });
    match rows {
        Ok(rows) => rows.filter_map(|row| row.ok()).collect(),
        Err(_) => HashMap::new(),
    }
}

fn url_host(url: &str) -> Option<String> {
    let trimmed = url.trim();
    let lower = trimmed.to_lowercase();
    if lower.starts_with("chrome://")
        || lower.starts_with("edge://")
        || lower.starts_with("about:")
        || lower.starts_with("file://")
    {
        return None;
    }
    let rest = trimmed
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(trimmed);
    let authority = rest.split(['/', '?', '#']).next()?.trim();
    let host = authority
        .rsplit('@')
        .next()?
        .split(':')
        .next()?
        .trim_end_matches('.')
        .to_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host).to_string();
    if host.is_empty()
        || host.chars().any(char::is_whitespace)
        || (!host.contains('.') && host != "localhost")
    {
        return None;
    }
    Some(host)
}

fn site_label(host: &str) -> Option<(&'static str, &'static str)> {
    const SITES: &[(&str, &str, &str)] = &[
        ("bilibili.com", "bilibili.com", "哔哩哔哩"),
        ("b23.tv", "bilibili.com", "哔哩哔哩"),
        ("chatgpt.com", "chatgpt.com", "ChatGPT"),
        ("chat.openai.com", "chatgpt.com", "ChatGPT"),
        ("docs.google.com", "docs.google.com", "Google 文档"),
        ("mail.google.com", "mail.google.com", "Gmail"),
        ("youtube.com", "youtube.com", "YouTube"),
        ("youtu.be", "youtube.com", "YouTube"),
        ("github.com", "github.com", "GitHub"),
        ("zhihu.com", "zhihu.com", "知乎"),
        ("weibo.com", "weibo.com", "微博"),
        ("douyin.com", "douyin.com", "抖音"),
        ("x.com", "x.com", "X"),
        ("twitter.com", "x.com", "X"),
        ("reddit.com", "reddit.com", "Reddit"),
        ("stackoverflow.com", "stackoverflow.com", "Stack Overflow"),
        ("google.com", "google.com", "Google"),
        ("bing.com", "bing.com", "Bing"),
        ("baidu.com", "baidu.com", "百度"),
        ("taobao.com", "taobao.com", "淘宝"),
        ("tmall.com", "tmall.com", "天猫"),
        ("jd.com", "jd.com", "京东"),
        ("netflix.com", "netflix.com", "Netflix"),
        ("spotify.com", "spotify.com", "Spotify"),
        ("notion.so", "notion.so", "Notion"),
        ("feishu.cn", "feishu.cn", "飞书"),
        ("qq.com", "qq.com", "腾讯"),
        ("163.com", "163.com", "网易"),
    ];
    for (domain, key, label) in SITES {
        if host == *domain
            || host
                .strip_suffix(domain)
                .map(|prefix| prefix.ends_with('.'))
                .unwrap_or(false)
        {
            return Some((key, label));
        }
    }
    if host == "localhost" || host == "127.0.0.1" {
        return Some(("localhost", "本地开发"));
    }
    None
}

fn app_label(app: &str, title: Option<&str>) -> String {
    let known = match app {
        "code.exe" | "code - insiders.exe" => "VS Code",
        "cursor.exe" => "Cursor",
        "trae.exe" => "Trae",
        "windsurf.exe" => "Windsurf",
        "qoder.exe" | "qoder cn.exe" => "Qoder",
        "chatgpt.exe" => "ChatGPT",
        "claude.exe" => "Claude",
        "notion.exe" => "Notion",
        "toccata-studio.exe" => "Toccata",
        "mumunxdevice.exe" => "MuMu 模拟器",
        "msedge.exe" => "Microsoft Edge",
        "chrome.exe" => "Google Chrome",
        "firefox.exe" => "Firefox",
        "brave.exe" => "Brave",
        "vivaldi.exe" => "Vivaldi",
        "opera.exe" => "Opera",
        "explorer.exe" => "文件资源管理器",
        "windowsterminal.exe" => "Windows Terminal",
        "powershell.exe" => "PowerShell",
        "cmd.exe" => "命令提示符",
        "idea64.exe" => "IntelliJ IDEA",
        "pycharm64.exe" => "PyCharm",
        "devenv.exe" => "Visual Studio",
        "wechat.exe" => "微信",
        "qq.exe" => "QQ",
        "dingtalk.exe" => "钉钉",
        "feishu.exe" | "lark.exe" => "飞书",
        "steam.exe" => "Steam",
        "discord.exe" => "Discord",
        "spotify.exe" => "Spotify",
        "winword.exe" => "Word",
        "excel.exe" => "Excel",
        "powerpnt.exe" => "PowerPoint",
        "notepad.exe" => "记事本",
        "mspaint.exe" => "画图",
        "calc.exe" => "计算器",
        "taskmgr.exe" => "任务管理器",
        "wps.exe" => "WPS",
        "mintty.exe" => "Mintty",
        "applicationframehost.exe" => "系统应用",
        "searchapp.exe" | "searchhost.exe" => "搜索",
        "lockapp.exe" => "锁屏",
        "startmenuexperiencehost.exe" => "开始菜单",
        "arknights.exe" => "明日方舟",
        "endfield.exe" => "明日方舟：终末地",
        _ => "",
    };
    if !known.is_empty() {
        return known.to_string();
    }
    if app.is_empty() {
        return title
            .map(|value| value.chars().take(40).collect())
            .unwrap_or_else(|| "未知应用".to_string());
    }
    let stem = app.strip_suffix(".exe").unwrap_or(app).replace(['-', '_', '.'], " ");
    let mut chars = stem.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => "未知应用".to_string(),
    }
}

#[derive(Clone)]
struct Span {
    start: i64,
    end: i64,
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
    let mut parent_spans: HashMap<String, Vec<Span>> = HashMap::new();
    let mut parent_keys: HashMap<String, i64> = HashMap::new();
    let mut parent_labels: HashMap<String, String> = HashMap::new();
    let mut child_spans: HashMap<(String, String), Vec<Span>> = HashMap::new();
    let mut child_keys: HashMap<(String, String), i64> = HashMap::new();
    let mut child_labels: HashMap<(String, String), String> = HashMap::new();
    let aliases = read_site_aliases(&store.db_path);

    for row in &rows {
        let category = classify_context(
            row.app.as_deref(),
            row.url.as_deref(),
            row.title.as_deref(),
            &aliases,
        );
        let parent_key = category.parent_key.clone();
        let parent_label = category.parent_label.clone();
        parent_labels
            .entry(parent_key.clone())
            .or_insert(parent_label);
        let is_child = category.key != parent_key;
        let child_key = (parent_key.clone(), category.key.clone());
        if is_child {
            child_labels
                .entry(child_key.clone())
                .or_insert(category.label.clone());
        }

        let span = if row.kind == 0 {
            Some(Span {
                start: row.ts_ms,
                end: row.ts_ms.saturating_add(ACTIVITY_WINDOW_MS),
            })
        } else {
            row.hold_ms
                .filter(|hold| *hold > 0)
                .map(|hold| Span {
                    start: row.ts_ms.saturating_sub(hold),
                    end: row.ts_ms,
                })
        };

        if let Some(span) = span {
            parent_spans
                .entry(parent_key.clone())
                .or_default()
                .push(span.clone());
            if is_child {
                child_spans
                    .entry(child_key.clone())
                    .or_default()
                    .push(span.clone());
            }
            spans.push(span);
        }

        if row.kind == 0 && row.ts_ms >= window_start && row.ts_ms < window_end {
            *key_counts.entry(row.keycode).or_insert(0) += 1;
            *parent_keys.entry(parent_key).or_insert(0) += 1;
            if is_child {
                *child_keys.entry(child_key).or_insert(0) += 1;
            }
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

    let mut apps: Vec<AppStat> = Vec::new();
    for (parent_key, spans) in parent_spans {
        let merged = merge_spans(spans);
        let active: i64 = merged
            .iter()
            .map(|span| overlap(span.start, span.end, window_start, window_end))
            .sum();
        let key_count = *parent_keys.get(&parent_key).unwrap_or(&0);
        if active == 0 && key_count == 0 {
            continue;
        }
        let label = parent_labels
            .get(&parent_key)
            .cloned()
            .unwrap_or_else(|| "未知应用".to_string());
        apps.push(AppStat {
            app: parent_key,
            label,
            active_ms: active,
            key_count,
            parent: None,
        });
    }

    let mut merged_children: HashMap<(String, String), AppStat> = HashMap::new();
    for ((parent_key, category_key), spans) in child_spans {
        let merged = merge_spans(spans);
        let active: i64 = merged
            .iter()
            .map(|span| overlap(span.start, span.end, window_start, window_end))
            .sum();
        let key_count = *child_keys
            .get(&(parent_key.clone(), category_key.clone()))
            .unwrap_or(&0);
        if active == 0 && key_count == 0 {
            continue;
        }
        let label = child_labels
            .get(&(parent_key.clone(), category_key.clone()))
            .cloned()
            .unwrap_or_else(|| "未知站点".to_string());
        let merge_key = (parent_key.clone(), label.clone());
        if let Some(existing) = merged_children.get_mut(&merge_key) {
            existing.active_ms += active;
            existing.key_count += key_count;
        } else {
            merged_children.insert(
                merge_key,
                AppStat {
                    app: category_key,
                    label,
                    active_ms: active,
                    key_count,
                    parent: Some(parent_key),
                },
            );
        }
    }
    apps.extend(merged_children.into_values());
    apps.sort_by(|left, right| {
        right
            .active_ms
            .cmp(&left.active_ms)
            .then_with(|| right.key_count.cmp(&left.key_count))
    });

    let key_count: i64 = key_counts.values().sum();
    let (mouse_left, mouse_right) =
        read_mouse_counts(&store.db_path, window_start, window_end).unwrap_or((0, 0));
    Snapshot {
        active_ms,
        key_count,
        mouse_left,
        mouse_right,
        key_counts,
        days: day_stats,
        apps,
    }
}

fn read_mouse_counts(path: &Path, start_ms: i64, end_ms: i64) -> Result<(i64, i64)> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut statement = conn.prepare(
        "SELECT button, COUNT(*) FROM mouse_events
         WHERE ts_ms >= ?1 AND ts_ms < ?2 GROUP BY button",
    )?;
    let mut rows = statement.query(params![start_ms, end_ms])?;
    let mut left = 0;
    let mut right = 0;
    while let Some(row) = rows.next()? {
        let button: i64 = row.get(0)?;
        let count: i64 = row.get(1)?;
        if button == 0 {
            left = count;
        } else if button == 1 {
            right = count;
        }
    }
    Ok((left, right))
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
        "SELECT ts_ms, keycode, kind, hold_ms, app, title, url
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
            title: row.get(5)?,
            url: row.get(6)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyStat {
    pub start_ms: i64,
    pub key_count: i64,
    pub active_ms: i64,
}

struct KeyInterval {
    ts_ms: i64,
    kind: i64,
    hold_ms: Option<i64>,
}

fn read_key_intervals(path: &Path, start_ms: i64, end_ms: i64) -> Result<Vec<KeyInterval>> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut statement = conn.prepare(
        "SELECT ts_ms, kind, hold_ms FROM key_events
         WHERE ts_ms >= ?1 AND ts_ms <= ?2
         ORDER BY ts_ms, rowid",
    )?;
    let rows = statement.query_map(params![start_ms, end_ms], |row| {
        Ok(KeyInterval {
            ts_ms: row.get(0)?,
            kind: row.get(1)?,
            hold_ms: row.get(2)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn daily_stats(
    store: &KeyboardStore,
    mut days: Vec<DayRange>,
    now_ms: i64,
) -> Vec<DailyStat> {
    days.retain(|day| day.end_ms > day.start_ms);
    days.sort_by_key(|day| day.start_ms);
    if days.is_empty() {
        return Vec::new();
    }
    let now_ms = if now_ms > 0 { now_ms } else { now_ms_now() };
    let window_start = days.first().map(|day| day.start_ms).unwrap_or(0);
    let window_end = days
        .iter()
        .map(|day| day.end_ms)
        .max()
        .unwrap_or(window_start)
        .min(now_ms);

    let rows = read_key_intervals(
        &store.db_path,
        window_start.saturating_sub(3_600_000),
        now_ms,
    )
    .unwrap_or_default();

    let mut spans: Vec<Span> = Vec::new();
    for row in &rows {
        if row.kind == 0 {
            spans.push(Span {
                start: row.ts_ms,
                end: row.ts_ms.saturating_add(ACTIVITY_WINDOW_MS),
            });
        } else if let Some(hold) = row.hold_ms.filter(|hold| *hold > 0) {
            spans.push(Span {
                start: row.ts_ms.saturating_sub(hold),
                end: row.ts_ms,
            });
        }
    }
    let merged = merge_spans(spans);

    days.iter()
        .map(|day| {
            let start = day.start_ms.max(window_start);
            let end = day.end_ms.min(window_end);
            let active_ms: i64 = merged
                .iter()
                .map(|span| overlap(span.start, span.end, start, end))
                .sum();
            let start_index = rows.partition_point(|row| row.ts_ms < start);
            let end_index = rows.partition_point(|row| row.ts_ms < end);
            let key_count = rows[start_index..end_index]
                .iter()
                .filter(|row| row.kind == 0)
                .count() as i64;
            DailyStat {
                start_ms: day.start_ms,
                key_count,
                active_ms,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(start: i64, end: i64) -> Span {
        Span { start, end }
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
    fn browser_urls_are_grouped_by_site() {
        let context = classify_context(
            Some("chrome.exe"),
            Some("https://www.bilibili.com/video/BV1xx411c7mD"),
            Some("某个视频 - Google Chrome"),
            &HashMap::new(),
        );
        assert_eq!(context.key, "site:bilibili.com");
        assert_eq!(context.label, "哔哩哔哩");
        assert_eq!(context.parent_key, "app:chrome.exe");
        assert_eq!(context.parent_label, "Google Chrome");

        let context = classify_context(
            Some("msedge.exe"),
            Some("https://chatgpt.com/c/123456"),
            Some("ChatGPT"),
            &HashMap::new(),
        );
        assert_eq!(context.key, "site:chatgpt.com");
        assert_eq!(context.label, "ChatGPT");
        assert_eq!(context.parent_key, "app:msedge.exe");

        let context = classify_context(
            Some("msedge.exe"),
            Some("https://b23.tv/abc"),
            Some("哔哩哔哩"),
            &HashMap::new(),
        );
        assert_eq!(context.key, "site:bilibili.com");
        assert_eq!(context.label, "哔哩哔哩");
    }

    #[test]
    fn ai_aliases_are_applied_to_unknown_sites() {
        let mut aliases = HashMap::new();
        aliases.insert("chat.deepseek.com".to_string(), "DeepSeek".to_string());
        aliases.insert("platform.deepseek.com".to_string(), "DeepSeek".to_string());
        let context = classify_context(
            Some("msedge.exe"),
            Some("https://chat.deepseek.com/a/chat"),
            None,
            &aliases,
        );
        assert_eq!(context.key, "web:chat.deepseek.com");
        assert_eq!(context.label, "DeepSeek");
        assert_eq!(context.parent_key, "app:msedge.exe");
    }

    #[test]
    fn unknown_browser_sites_use_hostname_and_apps_use_labels() {
        let context = classify_context(
            Some("chrome.exe"),
            Some("https://example.com/a/b?q=1"),
            None,
            &HashMap::new(),
        );
        assert_eq!(context.key, "web:example.com");
        assert_eq!(context.label, "example.com");
        assert_eq!(context.parent_key, "app:chrome.exe");

        let context =
            classify_context(Some("code.exe"), None, Some("project - VS Code"), &HashMap::new());
        assert_eq!(context.key, "app:code.exe");
        assert_eq!(context.label, "VS Code");
        assert_eq!(context.parent_key, "app:code.exe");

        let context =
            classify_context(Some("chrome.exe"), Some("chrome://newtab"), None, &HashMap::new());
        assert_eq!(context.key, "app:chrome.exe");
        assert_eq!(context.label, "Google Chrome");
        assert_eq!(context.parent_key, "app:chrome.exe");
    }

    #[test]
    fn empty_and_reversed_spans_are_ignored() {
        let merged = merge_spans(vec![span(5, 5), span(8, 2), span(1, 3)]);
        assert_eq!(merged.len(), 1);
        assert_eq!((merged[0].start, merged[0].end), (1, 3));
    }
}
