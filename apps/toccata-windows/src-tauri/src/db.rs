use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub struct Db(pub Mutex<Connection>);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub parent_id: Option<String>,
    pub title: String,
    pub details: String,
    pub planned_at: Option<f64>,
    pub planned_has_time: bool,
    pub created_at: f64,
    pub updated_at: f64,
    pub completed_at: Option<f64>,
    pub is_pinned: bool,
    pub subtask_order: Option<i64>,
    pub reminder_enabled: bool,
    pub reminder_lead_minutes: Option<i64>,
    pub notified_at: Option<f64>,
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS tasks (
  id            TEXT PRIMARY KEY NOT NULL,
  parent_id     TEXT REFERENCES tasks(id) ON DELETE CASCADE,
  title         TEXT NOT NULL CHECK(length(trim(title)) > 0),
  details       TEXT NOT NULL DEFAULT '',
  planned_at    REAL,
  planned_has_time INTEGER NOT NULL DEFAULT 0,
  created_at    REAL NOT NULL,
  updated_at    REAL NOT NULL,
  completed_at  REAL,
  is_pinned     INTEGER NOT NULL DEFAULT 0,
  subtask_order INTEGER,
  reminder_enabled INTEGER NOT NULL DEFAULT 0,
  reminder_lead_minutes INTEGER,
  notified_at   REAL
);
CREATE INDEX IF NOT EXISTS task_parent ON tasks(parent_id);
CREATE INDEX IF NOT EXISTS task_completed ON tasks(completed_at);

CREATE TABLE IF NOT EXISTS task_progress (
  id         TEXT PRIMARY KEY NOT NULL,
  task_id    TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  text       TEXT NOT NULL CHECK(length(trim(text)) > 0),
  created_at REAL NOT NULL
);
CREATE INDEX IF NOT EXISTS progress_task ON task_progress(task_id, created_at);
"#;

pub fn open(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("无法创建数据目录 {}", parent.display()))?;
    }
    let conn = Connection::open(path)
        .with_context(|| format!("无法打开任务数据库 {}", path.display()))?;
    init(&conn)?;
    Ok(conn)
}

#[cfg(test)]
pub fn open_in_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    init(&conn)?;
    Ok(conn)
}

fn init(conn: &Connection) -> Result<()> {
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    conn.execute_batch(SCHEMA)?;
    migrate(conn)?;
    Ok(())
}

fn migrate(conn: &Connection) -> Result<()> {
    if !has_column(conn, "tasks", "is_pinned")? {
        conn.execute_batch("ALTER TABLE tasks ADD COLUMN is_pinned INTEGER NOT NULL DEFAULT 0;")?;
    }
    if !has_column(conn, "tasks", "subtask_order")? {
        conn.execute_batch("ALTER TABLE tasks ADD COLUMN subtask_order INTEGER;")?;
    }
    if !has_column(conn, "tasks", "planned_has_time")? {
        conn.execute_batch(
            "ALTER TABLE tasks ADD COLUMN planned_has_time INTEGER NOT NULL DEFAULT 0;",
        )?;
    }
    if !has_column(conn, "tasks", "reminder_enabled")? {
        conn.execute_batch(
            "ALTER TABLE tasks ADD COLUMN reminder_enabled INTEGER NOT NULL DEFAULT 0;",
        )?;
    }
    if !has_column(conn, "tasks", "reminder_lead_minutes")? {
        conn.execute_batch("ALTER TABLE tasks ADD COLUMN reminder_lead_minutes INTEGER;")?;
    }
    if !has_column(conn, "tasks", "notified_at")? {
        conn.execute_batch("ALTER TABLE tasks ADD COLUMN notified_at REAL;")?;
    }
    Ok(())
}

fn has_column(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn bool_int(value: bool) -> i64 {
    if value { 1 } else { 0 }
}

fn now_secs() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

fn row_to_task(row: &Row<'_>) -> rusqlite::Result<Task> {
    Ok(Task {
        id: row.get(0)?,
        parent_id: row.get(1)?,
        title: row.get(2)?,
        details: row.get(3)?,
        planned_at: row.get(4)?,
        planned_has_time: row.get::<_, i64>(5)? != 0,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
        completed_at: row.get(8)?,
        is_pinned: row.get::<_, i64>(9)? != 0,
        subtask_order: row.get(10)?,
        reminder_enabled: row.get::<_, i64>(11)? != 0,
        reminder_lead_minutes: row.get(12)?,
        notified_at: row.get(13)?,
    })
}

pub fn list(conn: &Connection) -> Result<Vec<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, parent_id, title, details, planned_at, planned_has_time, created_at,
                updated_at, completed_at, is_pinned, subtask_order, reminder_enabled,
                reminder_lead_minutes, notified_at
         FROM tasks ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], row_to_task)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn create(
    conn: &mut Connection,
    parent_id: Option<&str>,
    title: &str,
    details: &str,
    planned_at: Option<f64>,
    planned_has_time: bool,
    reminder_enabled: bool,
    reminder_lead_minutes: Option<i64>,
) -> Result<Task> {
    let title = title.trim();
    if title.is_empty() {
        bail!("任务名称不能为空");
    }

    let now = now_secs();
    let id = Uuid::new_v4().to_string();
    let tx = conn.transaction()?;

    if let Some(parent) = parent_id {
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM tasks WHERE id = ?1)",
            [parent],
            |row| row.get(0),
        )?;
        if !exists {
            bail!("父任务不存在");
        }
        if depth_of(&tx, parent)? >= 5 {
            bail!("子任务最多支持 5 层");
        }
        restore_ancestors(&tx, parent, now)?;
    }

    let subtask_order: i64 = tx.query_row(
        "SELECT COALESCE(MAX(subtask_order), -1) + 1 FROM tasks WHERE parent_id IS ?1",
        params![parent_id],
        |row| row.get(0),
    )?;

    tx.execute(
        "INSERT INTO tasks
           (id, parent_id, title, details, planned_at, planned_has_time, created_at,
            updated_at, completed_at, is_pinned, subtask_order, reminder_enabled,
            reminder_lead_minutes, notified_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, NULL, 0, ?8, ?9, ?10, NULL)",
        params![
            id,
            parent_id,
            title,
            details,
            planned_at,
            bool_int(planned_has_time),
            now,
            subtask_order,
            bool_int(reminder_enabled),
            reminder_lead_minutes
        ],
    )?;
    tx.commit()?;

    Ok(Task {
        id,
        parent_id: parent_id.map(str::to_string),
        title: title.to_string(),
        details: details.to_string(),
        planned_at,
        planned_has_time,
        created_at: now,
        updated_at: now,
        completed_at: None,
        is_pinned: false,
        subtask_order: Some(subtask_order),
        reminder_enabled,
        reminder_lead_minutes,
        notified_at: None,
    })
}

pub fn update(
    conn: &Connection,
    id: &str,
    title: &str,
    details: &str,
    planned_at: Option<f64>,
    planned_has_time: bool,
    reminder_enabled: bool,
    reminder_lead_minutes: Option<i64>,
) -> Result<()> {
    let title = title.trim();
    if title.is_empty() {
        bail!("任务名称不能为空");
    }

    let previous: Option<(Option<f64>, i64, i64, Option<i64>)> = conn
        .query_row(
            "SELECT planned_at, planned_has_time, reminder_enabled, reminder_lead_minutes
             FROM tasks WHERE id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?;
    let Some((old_planned, old_has_time, old_reminder, old_lead)) = previous else {
        bail!("任务不存在");
    };

    let schedule_changed = old_planned != planned_at
        || old_has_time != bool_int(planned_has_time)
        || old_reminder != bool_int(reminder_enabled)
        || old_lead != reminder_lead_minutes;

    let notified_at: Option<f64> = conn.query_row(
        "SELECT notified_at FROM tasks WHERE id = ?1",
        [id],
        |row| row.get(0),
    )?;
    let next_notified = if schedule_changed { None } else { notified_at };

    conn.execute(
        "UPDATE tasks SET title = ?2, details = ?3, planned_at = ?4, planned_has_time = ?5,
                reminder_enabled = ?6, reminder_lead_minutes = ?7, notified_at = ?8,
                updated_at = ?9
         WHERE id = ?1",
        params![
            id,
            title,
            details,
            planned_at,
            bool_int(planned_has_time),
            bool_int(reminder_enabled),
            reminder_lead_minutes,
            next_notified,
            now_secs()
        ],
    )?;
    Ok(())
}

pub fn mark_reminder_notified(conn: &Connection, id: &str, at: f64) -> Result<()> {
    let changed = conn.execute(
        "UPDATE tasks SET notified_at = ?2 WHERE id = ?1",
        params![id, at],
    )?;
    if changed == 0 {
        bail!("任务不存在");
    }
    Ok(())
}

pub fn set_completed(conn: &mut Connection, id: &str, completed: bool) -> Result<()> {
    let now = now_secs();
    let tx = conn.transaction()?;
    let exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM tasks WHERE id = ?1)",
        [id],
        |row| row.get(0),
    )?;
    if !exists {
        bail!("任务不存在");
    }

    if completed {
        tx.execute(
            "WITH RECURSIVE sub(id) AS (
               SELECT id FROM tasks WHERE id = ?1
               UNION ALL
               SELECT t.id FROM tasks t JOIN sub s ON t.parent_id = s.id
             )
             UPDATE tasks SET completed_at = ?2, updated_at = ?2
             WHERE id IN (SELECT id FROM sub) AND completed_at IS NULL",
            params![id, now],
        )?;
    } else {
        tx.execute(
            "WITH RECURSIVE anc(id, parent_id) AS (
               SELECT id, parent_id FROM tasks WHERE id = ?1
               UNION ALL
               SELECT t.id, t.parent_id FROM tasks t JOIN anc a ON t.id = a.parent_id
             )
             UPDATE tasks SET completed_at = NULL, updated_at = ?2
             WHERE id IN (SELECT id FROM anc) AND completed_at IS NOT NULL",
            params![id, now],
        )?;
    }
    tx.commit()?;
    Ok(())
}

pub fn set_pinned(conn: &Connection, id: &str, pinned: bool) -> Result<()> {
    let root: Option<String> = conn
        .query_row(
            "WITH RECURSIVE anc(id, parent_id) AS (
               SELECT id, parent_id FROM tasks WHERE id = ?1
               UNION ALL
               SELECT t.id, t.parent_id FROM tasks t JOIN anc a ON t.id = a.parent_id
             )
             SELECT id FROM anc WHERE parent_id IS NULL",
            [id],
            |row| row.get(0),
        )
        .optional()?;
    let root = root.context("任务不存在")?;
    conn.execute(
        "UPDATE tasks SET is_pinned = ?2, updated_at = ?3 WHERE id = ?1",
        params![root, if pinned { 1i64 } else { 0i64 }, now_secs()],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    let changed = conn.execute("DELETE FROM tasks WHERE id = ?1", [id])?;
    if changed == 0 {
        bail!("任务不存在");
    }
    Ok(())
}

fn depth_of(conn: &Connection, id: &str) -> Result<i64> {
    let depth: Option<i64> = conn
        .query_row(
            "WITH RECURSIVE anc(id, parent_id, depth) AS (
               SELECT id, parent_id, 0 FROM tasks WHERE id = ?1
               UNION ALL
               SELECT t.id, t.parent_id, anc.depth + 1
               FROM tasks t JOIN anc ON t.id = anc.parent_id
             )
             SELECT MAX(depth) FROM anc",
            [id],
            |row| row.get(0),
        )
        .optional()?;
    depth.context("父任务不存在")
}

fn restore_ancestors(conn: &Connection, id: &str, now: f64) -> Result<()> {
    conn.execute(
        "WITH RECURSIVE anc(id, parent_id) AS (
           SELECT id, parent_id FROM tasks WHERE id = ?1
           UNION ALL
           SELECT t.id, t.parent_id FROM tasks t JOIN anc a ON t.id = a.parent_id
         )
         UPDATE tasks SET completed_at = NULL, updated_at = ?2
         WHERE id IN (SELECT id FROM anc) AND completed_at IS NOT NULL",
        params![id, now],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_task(conn: &mut Connection, title: &str, parent: Option<&str>) -> Task {
        create(conn, parent, title, "", None, false, false, None).unwrap()
    }

    fn get(conn: &Connection, id: &str) -> Task {
        conn.query_row(
            "SELECT id, parent_id, title, details, planned_at, planned_has_time, created_at,
                    updated_at, completed_at, is_pinned, subtask_order, reminder_enabled,
                    reminder_lead_minutes, notified_at
             FROM tasks WHERE id = ?1",
            [id],
            row_to_task,
        )
        .unwrap()
    }

    #[test]
    fn reminder_notification_is_kept_until_the_schedule_changes() {
        let mut conn = open_in_memory().unwrap();
        let task = create(
            &mut conn,
            None,
            "remind me",
            "",
            Some(1_000.0),
            true,
            true,
            Some(10),
        )
        .unwrap();
        mark_reminder_notified(&conn, &task.id, 2_000.0).unwrap();

        update(
            &conn,
            &task.id,
            "renamed",
            "",
            Some(1_000.0),
            true,
            true,
            Some(10),
        )
        .unwrap();
        assert_eq!(get(&conn, &task.id).notified_at, Some(2_000.0));

        update(
            &conn,
            &task.id,
            "renamed",
            "",
            Some(3_000.0),
            true,
            true,
            Some(10),
        )
        .unwrap();
        assert!(get(&conn, &task.id).notified_at.is_none());
    }

    #[test]
    fn reminder_and_time_fields_round_trip() {
        let mut conn = open_in_memory().unwrap();
        let task = create(
            &mut conn,
            None,
            "with time",
            "",
            Some(1_700_000_000.0),
            true,
            true,
            Some(15),
        )
        .unwrap();

        let loaded = get(&conn, &task.id);
        assert!(loaded.planned_has_time);
        assert!(loaded.reminder_enabled);
        assert_eq!(loaded.reminder_lead_minutes, Some(15));
        assert_eq!(loaded.planned_at, Some(1_700_000_000.0));
    }

    #[test]
    fn completing_root_completes_incomplete_descendants_and_keeps_their_time() {
        let mut conn = open_in_memory().unwrap();
        let root = create_task(&mut conn, "root", None);
        let child = create_task(&mut conn, "child", Some(&root.id));
        let grandchild = create_task(&mut conn, "grandchild", Some(&child.id));

        set_completed(&mut conn, &child.id, true).unwrap();
        let child_done_at = get(&conn, &child.id).completed_at.unwrap();
        set_completed(&mut conn, &root.id, true).unwrap();

        assert!(get(&conn, &root.id).completed_at.is_some());
        assert_eq!(get(&conn, &child.id).completed_at, Some(child_done_at));
        assert!(get(&conn, &grandchild.id).completed_at.is_some());
    }

    #[test]
    fn restoring_child_restores_ancestors_but_keeps_completed_descendants() {
        let mut conn = open_in_memory().unwrap();
        let root = create_task(&mut conn, "root", None);
        let child = create_task(&mut conn, "child", Some(&root.id));
        let grandchild = create_task(&mut conn, "grandchild", Some(&child.id));
        set_completed(&mut conn, &root.id, true).unwrap();

        set_completed(&mut conn, &child.id, false).unwrap();

        assert!(get(&conn, &root.id).completed_at.is_none());
        assert!(get(&conn, &child.id).completed_at.is_none());
        assert!(get(&conn, &grandchild.id).completed_at.is_some());
    }

    #[test]
    fn new_subtask_restores_its_completed_ancestors() {
        let mut conn = open_in_memory().unwrap();
        let root = create_task(&mut conn, "root", None);
        set_completed(&mut conn, &root.id, true).unwrap();

        let child = create_task(&mut conn, "new child", Some(&root.id));

        assert!(get(&conn, &root.id).completed_at.is_none());
        assert!(get(&conn, &child.id).completed_at.is_none());
    }

    #[test]
    fn pinning_a_subtask_pins_the_root_only() {
        let mut conn = open_in_memory().unwrap();
        let root = create_task(&mut conn, "root", None);
        let child = create_task(&mut conn, "child", Some(&root.id));

        set_pinned(&conn, &child.id, true).unwrap();

        assert!(get(&conn, &root.id).is_pinned);
        assert!(!get(&conn, &child.id).is_pinned);
    }

    #[test]
    fn subtasks_stop_at_five_levels() {
        let mut conn = open_in_memory().unwrap();
        let root = create_task(&mut conn, "root", None);
        let mut parent = root.id.clone();
        for level in 1..=5 {
            let child = create_task(&mut conn, &format!("child {level}"), Some(&parent));
            parent = child.id;
        }
        assert!(create(&mut conn, Some(&parent), "too deep", "", None, false, false, None).is_err());
    }

    #[test]
    fn delete_cascades_to_descendants() {
        let mut conn = open_in_memory().unwrap();
        let root = create_task(&mut conn, "root", None);
        let child = create_task(&mut conn, "child", Some(&root.id));
        let grandchild = create_task(&mut conn, "grandchild", Some(&child.id));

        delete(&conn, &root.id).unwrap();

        assert!(list(&conn).unwrap().is_empty());
        let remaining: i64 = conn
            .query_row("SELECT COUNT(*) FROM tasks WHERE id = ?1", [&grandchild.id], |row| row.get(0))
            .unwrap();
        assert_eq!(remaining, 0);
    }

    #[test]
    fn manual_order_increments_per_parent() {
        let mut conn = open_in_memory().unwrap();
        let first = create_task(&mut conn, "first", None);
        let second = create_task(&mut conn, "second", None);
        let child = create_task(&mut conn, "child", Some(&first.id));

        assert_eq!(first.subtask_order, Some(0));
        assert_eq!(second.subtask_order, Some(1));
        assert_eq!(child.subtask_order, Some(0));
    }

    #[test]
    fn empty_titles_are_rejected() {
        let mut conn = open_in_memory().unwrap();
        assert!(create(&mut conn, None, "   ", "", None, false, false, None).is_err());
        let task = create_task(&mut conn, "ok", None);
        assert!(update(&conn, &task.id, "\t", "", None, false, false, None).is_err());
    }

    #[test]
    fn migrates_old_task_table_without_new_columns() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE tasks (
               id TEXT PRIMARY KEY NOT NULL,
               parent_id TEXT REFERENCES tasks(id) ON DELETE CASCADE,
               title TEXT NOT NULL,
               details TEXT NOT NULL DEFAULT '',
               planned_at REAL,
               created_at REAL NOT NULL,
               updated_at REAL NOT NULL,
               completed_at REAL,
               priority INTEGER
             );
             INSERT INTO tasks (id, title, created_at, updated_at)
             VALUES ('legacy', '旧任务', 1.0, 1.0);",
        )
        .unwrap();

        init(&conn).unwrap();

        let task = get(&conn, "legacy");
        assert_eq!(task.title, "旧任务");
        assert!(!task.is_pinned);
        assert_eq!(task.subtask_order, None);
    }
}
