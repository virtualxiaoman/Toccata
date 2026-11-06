use crate::keyboard;
use anyhow::{bail, Context, Result};
use chrono::{Datelike, Days, Local, TimeZone};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub struct SchedulerStore {
    pub db_path: PathBuf,
}

static RUNNING: Mutex<Option<HashSet<String>>> = Mutex::new(None);

const TICK_INTERVAL: Duration = Duration::from_secs(15);
const SCRIPT_TIMEOUT: Duration = Duration::from_secs(300);
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone)]
struct ScriptRow {
    id: String,
    name: String,
    command: String,
    schedule_kind: String,
    interval_minutes: Option<i64>,
    time_of_day: Option<String>,
    weekday: Option<i64>,
    enabled: bool,
    last_run_ms: Option<i64>,
    next_run_ms: Option<i64>,
    last_status: Option<String>,
    last_output: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficialTask {
    enabled: bool,
    interval_minutes: i64,
    last_run_ms: Option<i64>,
    next_run_ms: Option<i64>,
    last_status: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledScript {
    id: String,
    name: String,
    command: String,
    schedule_kind: String,
    interval_minutes: Option<i64>,
    time_of_day: Option<String>,
    weekday: Option<i64>,
    enabled: bool,
    last_run_ms: Option<i64>,
    next_run_ms: Option<i64>,
    last_status: Option<String>,
    last_output: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulerState {
    official: OfficialTask,
    scripts: Vec<ScheduledScript>,
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

fn running() -> std::sync::MutexGuard<'static, Option<HashSet<String>>> {
    RUNNING.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn is_running(id: &str) -> bool {
    running()
        .as_ref()
        .map(|set| set.contains(id))
        .unwrap_or(false)
}

fn set_running(id: &str, active: bool) {
    let mut guard = running();
    let set = guard.get_or_insert_with(HashSet::new);
    if active {
        set.insert(id.to_string());
    } else {
        set.remove(id);
    }
}

pub fn start(db_path: PathBuf) {
    if let Err(error) = init_db(&db_path) {
        eprintln!("[toccata] 定时任务初始化失败：{error:#}");
    }
    thread::Builder::new()
        .name("toccata-scheduler".into())
        .spawn(move || loop {
            if let Err(error) = tick(&db_path) {
                eprintln!("[toccata] 定时任务检查失败：{error:#}");
            }
            thread::sleep(TICK_INTERVAL);
        })
        .ok();
}

fn init_db(path: &Path) -> Result<()> {
    let conn = Connection::open(path)
        .with_context(|| format!("无法打开定时任务数据库 {}", path.display()))?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         CREATE TABLE IF NOT EXISTS scheduler_config (
           key   TEXT PRIMARY KEY NOT NULL,
           value TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS scheduled_scripts (
           id               TEXT PRIMARY KEY NOT NULL,
           name             TEXT NOT NULL,
           command          TEXT NOT NULL,
           schedule_kind    TEXT NOT NULL,
           interval_minutes INTEGER,
           time_of_day      TEXT,
           weekday          INTEGER,
           enabled          INTEGER NOT NULL DEFAULT 1,
           last_run_ms      INTEGER,
           next_run_ms      INTEGER,
           last_status      TEXT,
           last_output      TEXT,
           created_ms       INTEGER NOT NULL,
           updated_ms       INTEGER NOT NULL
         );",
    )?;
    ensure_official_defaults(&conn)?;
    conn.execute(
        "UPDATE scheduled_scripts SET last_status = '已中断' WHERE last_status = '运行中'",
        [],
    )?;
    if get_config(&conn, "official_last_status").as_deref() == Some("运行中…") {
        set_config(&conn, "official_last_status", "已中断")?;
    }
    Ok(())
}

fn get_config(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row(
        "SELECT value FROM scheduler_config WHERE key = ?1",
        [key],
        |row| row.get(0),
    )
    .optional()
    .ok()
    .flatten()
}

fn set_config(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO scheduler_config (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

fn ensure_official_defaults(conn: &Connection) -> Result<()> {
    if get_config(conn, "official_enabled").is_none() {
        set_config(conn, "official_enabled", "1")?;
    }
    if get_config(conn, "official_interval_minutes").is_none() {
        set_config(conn, "official_interval_minutes", "180")?;
    }
    if get_config(conn, "official_next_run_ms").is_none() {
        let interval = get_config(conn, "official_interval_minutes")
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or(180);
        let next = now_ms() + interval.max(1) * 60_000;
        set_config(conn, "official_next_run_ms", &next.to_string())?;
    }
    if get_config(conn, "official_last_status").is_none() {
        set_config(conn, "official_last_status", "尚未运行")?;
    }
    Ok(())
}

fn official_task(conn: &Connection) -> OfficialTask {
    let enabled = get_config(conn, "official_enabled").as_deref() != Some("0");
    let interval_minutes = get_config(conn, "official_interval_minutes")
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(180)
        .max(1);
    let last_run_ms = get_config(conn, "official_last_run_ms").and_then(|v| v.parse().ok());
    let next_run_ms = get_config(conn, "official_next_run_ms").and_then(|v| v.parse().ok());
    let last_status = get_config(conn, "official_last_status");
    OfficialTask {
        enabled,
        interval_minutes,
        last_run_ms,
        next_run_ms: if enabled { next_run_ms } else { None },
        last_status,
    }
}

fn script_rows(conn: &Connection) -> Result<Vec<ScriptRow>> {
    let mut statement = conn.prepare(
        "SELECT id, name, command, schedule_kind, interval_minutes, time_of_day, weekday,
                enabled, last_run_ms, next_run_ms, last_status, last_output
         FROM scheduled_scripts ORDER BY created_ms",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(ScriptRow {
            id: row.get(0)?,
            name: row.get(1)?,
            command: row.get(2)?,
            schedule_kind: row.get(3)?,
            interval_minutes: row.get(4)?,
            time_of_day: row.get(5)?,
            weekday: row.get(6)?,
            enabled: row.get::<_, i64>(7)? != 0,
            last_run_ms: row.get(8)?,
            next_run_ms: row.get(9)?,
            last_status: row.get(10)?,
            last_output: row.get(11)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn to_public(row: ScriptRow) -> ScheduledScript {
    ScheduledScript {
        id: row.id,
        name: row.name,
        command: row.command,
        schedule_kind: row.schedule_kind,
        interval_minutes: row.interval_minutes,
        time_of_day: row.time_of_day,
        weekday: row.weekday,
        enabled: row.enabled,
        last_run_ms: row.last_run_ms,
        next_run_ms: if row.enabled { row.next_run_ms } else { None },
        last_status: row.last_status,
        last_output: row.last_output,
    }
}

pub fn state(store: &SchedulerStore) -> Result<SchedulerState> {
    let conn = Connection::open(&store.db_path)?;
    ensure_official_defaults(&conn)?;
    Ok(SchedulerState {
        official: official_task(&conn),
        scripts: script_rows(&conn)?.into_iter().map(to_public).collect(),
    })
}

fn parse_time(value: &str) -> Result<(u32, u32)> {
    let (hour, minute) = value
        .split_once(':')
        .context("时间格式应为 HH:MM")?;
    let hour: u32 = hour.parse().context("时间格式应为 HH:MM")?;
    let minute: u32 = minute.parse().context("时间格式应为 HH:MM")?;
    if hour > 23 || minute > 59 {
        bail!("时间格式应为 HH:MM");
    }
    Ok((hour, minute))
}

fn next_local_time(from_ms: i64, hour: u32, minute: u32, weekday: Option<i64>) -> Result<i64> {
    let local = Local
        .timestamp_millis_opt(from_ms)
        .single()
        .context("无法解析本地时间")?;
    let start = local.date_naive();
    let max_offset = if weekday.is_some() { 8 } else { 2 };
    for offset in 0..=max_offset {
        let date = start
            .checked_add_days(Days::new(offset))
            .context("日期超出范围")?;
        if let Some(target) = weekday {
            let current = date.weekday().number_from_monday() as i64;
            if current != target {
                continue;
            }
        }
        let naive = date
            .and_hms_opt(hour, minute, 0)
            .context("时间超出范围")?;
        if let Some(candidate) = Local.from_local_datetime(&naive).earliest() {
            let ms = candidate.timestamp_millis();
            if ms > from_ms {
                return Ok(ms);
            }
        }
    }
    bail!("无法计算下一次执行时间")
}

fn compute_next(
    kind: &str,
    interval_minutes: Option<i64>,
    time_of_day: Option<&str>,
    weekday: Option<i64>,
    from_ms: i64,
) -> Result<Option<i64>> {
    match kind {
        "interval" => {
            let minutes = interval_minutes.context("缺少间隔分钟数")?;
            if minutes < 1 || minutes > 10_080 {
                bail!("间隔需在 1 到 10080 分钟之间");
            }
            Ok(Some(from_ms + minutes * 60_000))
        }
        "daily" => {
            let (hour, minute) = parse_time(time_of_day.context("缺少执行时间")?)?;
            Ok(Some(next_local_time(from_ms, hour, minute, None)?))
        }
        "weekly" => {
            let (hour, minute) = parse_time(time_of_day.context("缺少执行时间")?)?;
            let target = weekday.context("缺少星期")?;
            if !(1..=7).contains(&target) {
                bail!("星期需在 1 到 7 之间");
            }
            Ok(Some(next_local_time(from_ms, hour, minute, Some(target))?))
        }
        "manual" => Ok(None),
        _ => bail!("未知的执行计划"),
    }
}

fn tick(path: &Path) -> Result<()> {
    let now = now_ms();
    let conn = Connection::open(path)?;
    ensure_official_defaults(&conn)?;

    let official_enabled = get_config(&conn, "official_enabled").as_deref() != Some("0");
    let official_next = get_config(&conn, "official_next_run_ms").and_then(|v| v.parse::<i64>().ok());
    if official_enabled
        && official_next.map(|next| next <= now).unwrap_or(false)
        && !is_running("official")
    {
        let interval = get_config(&conn, "official_interval_minutes")
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or(180)
            .max(1);
        set_config(&conn, "official_next_run_ms", &(now + interval * 60_000).to_string())?;
        set_config(&conn, "official_last_status", "运行中…")?;
        drop(conn);
        let path = path.to_path_buf();
        tauri::async_runtime::spawn(async move {
            run_official_task(path, interval).await;
        });
        return Ok(());
    }

    let due: Vec<ScriptRow> = script_rows(&conn)?
        .into_iter()
        .filter(|script| {
            script.enabled
                && script.schedule_kind != "manual"
                && script.next_run_ms.map(|next| next <= now).unwrap_or(false)
                && !is_running(&script.id)
        })
        .collect();

    for script in due {
        let next = compute_next(
            &script.schedule_kind,
            script.interval_minutes,
            script.time_of_day.as_deref(),
            script.weekday,
            now,
        )?;
        conn.execute(
            "UPDATE scheduled_scripts SET next_run_ms = ?2, last_status = '运行中…', updated_ms = ?3
             WHERE id = ?1",
            params![script.id, next, now],
        )?;
        spawn_script(path.to_path_buf(), script, false);
    }
    Ok(())
}

async fn run_official_task(path: PathBuf, interval_minutes: i64) {
    if is_running("official") {
        return;
    }
    set_running("official", true);
    let result = keyboard::ai_group_sites(path.clone()).await;
    let finished = now_ms();
    let next = finished + interval_minutes.max(1) * 60_000;
    if let Ok(conn) = Connection::open(&path) {
        let _ = set_config(&conn, "official_last_run_ms", &finished.to_string());
        let _ = set_config(&conn, "official_next_run_ms", &next.to_string());
        let status = match result {
            Ok(0) => "成功：没有需要汇总的新站点".to_string(),
            Ok(count) => format!("成功：汇总 {count} 个站点"),
            Err(error) => format!("失败：{error}"),
        };
        let _ = set_config(&conn, "official_last_status", &status);
    }
    set_running("official", false);
}

fn spawn_script(path: PathBuf, script: ScriptRow, manual: bool) {
    if is_running(&script.id) {
        return;
    }
    set_running(&script.id, true);
    thread::spawn(move || {
        if let Ok(conn) = Connection::open(&path) {
            let _ = conn.execute(
                "UPDATE scheduled_scripts SET last_status = '运行中…', updated_ms = ?2 WHERE id = ?1",
                params![script.id, now_ms()],
            );
        }
        let project_root = path
            .parent()
            .and_then(|data| data.parent())
            .map(Path::to_path_buf);
        let (success, status, output) = execute_script(&script.command, project_root.as_deref());
        let finished = now_ms();
        let next = if manual {
            script.next_run_ms
        } else {
            compute_next(
                &script.schedule_kind,
                script.interval_minutes,
                script.time_of_day.as_deref(),
                script.weekday,
                finished,
            )
            .ok()
            .flatten()
        };
        let _ = success;
        if let Ok(conn) = Connection::open(&path) {
            let _ = conn.execute(
                "UPDATE scheduled_scripts SET last_run_ms = ?2, last_status = ?3,
                        last_output = ?4, next_run_ms = ?5, updated_ms = ?2
                 WHERE id = ?1",
                params![script.id, finished, status, output, next],
            );
        }
        set_running(&script.id, false);
    });
}

fn decode_output(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_string(),
        Err(_) => encoding_rs::GBK.decode(bytes).0.into_owned(),
    }
}

fn execute_script(command: &str, cwd: Option<&Path>) -> (bool, String, String) {
    use std::os::windows::process::CommandExt;
    let mut process = Command::new("cmd.exe");
    process
        .arg("/C")
        .arg(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW);
    if let Some(directory) = cwd {
        process.current_dir(directory);
    }
    let mut child = match process.spawn() {
        Ok(child) => child,
        Err(error) => {
            return (false, format!("无法启动脚本：{error}"), String::new());
        }
    };
    let pid = child.id().to_string();
    let deadline = Instant::now() + SCRIPT_TIMEOUT;
    let exit_status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(error) => {
                return (false, format!("等待脚本失败：{error}"), String::new());
            }
        }
        if Instant::now() >= deadline {
            let _ = Command::new("taskkill")
                .args(["/PID", &pid, "/T", "/F"])
                .creation_flags(CREATE_NO_WINDOW)
                .status();
            let _ = child.wait();
            return (false, "执行超时（5 分钟）".to_string(), String::new());
        }
        thread::sleep(Duration::from_millis(200));
    };
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    if let Some(mut pipe) = child.stdout.take() {
        let _ = pipe.read_to_end(&mut stdout);
    }
    if let Some(mut pipe) = child.stderr.take() {
        let _ = pipe.read_to_end(&mut stderr);
    }
    let mut output = format!(
        "{}{}",
        decode_output(&stdout),
        decode_output(&stderr)
    );
    output = output.trim().to_string();
    if output.chars().count() > 4000 {
        output = output.chars().take(4000).collect::<String>() + "…";
    }
    let success = exit_status.success();
    let status = if success {
        "成功".to_string()
    } else {
        format!("失败（退出码 {}）", exit_status.code().unwrap_or(-1))
    };
    (success, status, output)
}

#[tauri::command]
pub fn scheduler_state(store: tauri::State<'_, SchedulerStore>) -> Result<SchedulerState, String> {
    state(&store).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn save_official_settings(
    store: tauri::State<'_, SchedulerStore>,
    enabled: bool,
    interval_minutes: i64,
) -> Result<(), String> {
    if !(1..=10_080).contains(&interval_minutes) {
        return Err("间隔需在 1 到 10080 分钟之间".to_string());
    }
    let conn = Connection::open(&store.db_path).map_err(|error| error.to_string())?;
    set_config(&conn, "official_enabled", if enabled { "1" } else { "0" })
        .map_err(|error| error.to_string())?;
    set_config(&conn, "official_interval_minutes", &interval_minutes.to_string())
        .map_err(|error| error.to_string())?;
    if enabled {
        let next = now_ms() + interval_minutes * 60_000;
        set_config(&conn, "official_next_run_ms", &next.to_string())
            .map_err(|error| error.to_string())?;
    } else {
        let _ = conn.execute(
            "DELETE FROM scheduler_config WHERE key = 'official_next_run_ms'",
            [],
        );
    }
    Ok(())
}

#[tauri::command]
pub async fn run_official_summary(store: tauri::State<'_, SchedulerStore>) -> Result<(), String> {
    let db_path = store.db_path.clone();
    if is_running("official") {
        return Err("官方任务正在运行".to_string());
    }
    let interval = {
        let conn = Connection::open(&db_path).map_err(|error| error.to_string())?;
        get_config(&conn, "official_interval_minutes")
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or(180)
            .max(1)
    };
    set_config(
        &Connection::open(&db_path).map_err(|error| error.to_string())?,
        "official_last_status",
        "运行中…",
    )
    .map_err(|error| error.to_string())?;
    run_official_task(db_path, interval).await;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn save_scheduled_script(
    store: tauri::State<'_, SchedulerStore>,
    id: Option<String>,
    name: String,
    command: String,
    schedule_kind: String,
    interval_minutes: Option<i64>,
    time_of_day: Option<String>,
    weekday: Option<i64>,
    enabled: bool,
) -> Result<String, String> {
    save_script(
        &store.db_path,
        id,
        name,
        command,
        schedule_kind,
        interval_minutes,
        time_of_day,
        weekday,
        enabled,
    )
    .map_err(|error| error.to_string())
}

#[allow(clippy::too_many_arguments)]
fn save_script(
    db_path: &Path,
    id: Option<String>,
    name: String,
    command: String,
    schedule_kind: String,
    interval_minutes: Option<i64>,
    time_of_day: Option<String>,
    weekday: Option<i64>,
    enabled: bool,
) -> Result<String> {
    let command = command.trim().to_string();
    if command.is_empty() {
        bail!("脚本内容不能为空");
    }
    if !["manual", "interval", "daily", "weekly"].contains(&schedule_kind.as_str()) {
        bail!("未知的执行计划");
    }
    if schedule_kind == "interval" {
        let minutes = interval_minutes.context("缺少间隔分钟数")?;
        if !(1..=10_080).contains(&minutes) {
            bail!("间隔需在 1 到 10080 分钟之间");
        }
    }
    if matches!(schedule_kind.as_str(), "daily" | "weekly") {
        parse_time(time_of_day.as_deref().unwrap_or(""))?;
    }
    if schedule_kind == "weekly" {
        let target = weekday.context("缺少星期")?;
        if !(1..=7).contains(&target) {
            bail!("星期需在 1 到 7 之间");
        }
    }
    let name = if name.trim().is_empty() {
        command.chars().take(30).collect::<String>()
    } else {
        name.trim().to_string()
    };
    let next = if enabled {
        compute_next(
            &schedule_kind,
            interval_minutes,
            time_of_day.as_deref(),
            weekday,
            now_ms(),
        )?
    } else {
        None
    };
    let now = now_ms();
    let conn = Connection::open(db_path)?;
    let script_id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM scheduled_scripts WHERE id = ?1)",
        [&script_id],
        |row| row.get(0),
    )?;
    if exists {
        conn.execute(
            "UPDATE scheduled_scripts SET name = ?2, command = ?3, schedule_kind = ?4,
                    interval_minutes = ?5, time_of_day = ?6, weekday = ?7, enabled = ?8,
                    next_run_ms = ?9, updated_ms = ?10
             WHERE id = ?1",
            params![
                script_id,
                name,
                command,
                schedule_kind,
                interval_minutes,
                time_of_day,
                weekday,
                if enabled { 1i64 } else { 0i64 },
                next,
                now
            ],
        )?;
    } else {
        conn.execute(
            "INSERT INTO scheduled_scripts
               (id, name, command, schedule_kind, interval_minutes, time_of_day, weekday,
                enabled, next_run_ms, last_status, created_ms, updated_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, '尚未运行', ?10, ?10)",
            params![
                script_id,
                name,
                command,
                schedule_kind,
                interval_minutes,
                time_of_day,
                weekday,
                if enabled { 1i64 } else { 0i64 },
                next,
                now
            ],
        )?;
    }
    Ok(script_id)
}

#[tauri::command]
pub fn delete_scheduled_script(
    store: tauri::State<'_, SchedulerStore>,
    id: String,
) -> Result<(), String> {
    let conn = Connection::open(&store.db_path).map_err(|error| error.to_string())?;
    conn.execute("DELETE FROM scheduled_scripts WHERE id = ?1", [&id])
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn set_scheduled_script_enabled(
    store: tauri::State<'_, SchedulerStore>,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    let conn = Connection::open(&store.db_path).map_err(|error| error.to_string())?;
    let script = script_rows(&conn)
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|script| script.id == id)
        .ok_or_else(|| "脚本不存在".to_string())?;
    let next = if enabled {
        compute_next(
            &script.schedule_kind,
            script.interval_minutes,
            script.time_of_day.as_deref(),
            script.weekday,
            now_ms(),
        )
        .map_err(|error| error.to_string())?
    } else {
        None
    };
    conn.execute(
        "UPDATE scheduled_scripts SET enabled = ?2, next_run_ms = ?3, updated_ms = ?4
         WHERE id = ?1",
        params![id, if enabled { 1i64 } else { 0i64 }, next, now_ms()],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn interval_and_manual_schedules_are_computed() {
        assert_eq!(
            compute_next("interval", Some(60), None, None, 1_000).unwrap(),
            Some(3_601_000)
        );
        assert_eq!(compute_next("manual", None, None, None, 1_000).unwrap(), None);
        assert!(compute_next("interval", Some(0), None, None, 1_000).is_err());
    }

    #[test]
    fn daily_and_weekly_schedules_land_on_requested_time() {
        let now = now_ms();
        let daily = compute_next("daily", None, Some("09:00"), None, now)
            .unwrap()
            .unwrap();
        assert!(daily > now);
        let daily_local = Local.timestamp_millis_opt(daily).single().unwrap();
        assert_eq!((daily_local.hour(), daily_local.minute()), (9, 0));

        let today = Local
            .timestamp_millis_opt(now)
            .single()
            .unwrap()
            .weekday()
            .number_from_monday() as i64;
        let weekly = compute_next("weekly", None, Some("08:30"), Some(today), now)
            .unwrap()
            .unwrap();
        assert!(weekly > now);
        let weekly_local = Local.timestamp_millis_opt(weekly).single().unwrap();
        assert_eq!(
            weekly_local.weekday().number_from_monday() as i64,
            today
        );
        assert_eq!((weekly_local.hour(), weekly_local.minute()), (8, 30));
    }

    #[test]
    fn invalid_time_and_weekday_are_rejected() {
        assert!(parse_time("25:00").is_err());
        assert!(parse_time("09:60").is_err());
        assert!(compute_next("weekly", None, Some("09:00"), Some(8), 0).is_err());
    }
}

#[tauri::command]
pub fn run_scheduled_script(
    store: tauri::State<'_, SchedulerStore>,
    id: String,
) -> Result<(), String> {
    let conn = Connection::open(&store.db_path).map_err(|error| error.to_string())?;
    let script = script_rows(&conn)
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|script| script.id == id)
        .ok_or_else(|| "脚本不存在".to_string())?;
    if is_running(&id) {
        return Err("脚本正在运行".to_string());
    }
    conn.execute(
        "UPDATE scheduled_scripts SET last_status = '运行中…', updated_ms = ?2 WHERE id = ?1",
        params![id, now_ms()],
    )
    .map_err(|error| error.to_string())?;
    spawn_script(store.db_path.clone(), script, true);
    Ok(())
}
