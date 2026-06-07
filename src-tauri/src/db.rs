use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::Mutex;

/// 数据库连接的全局持有者（Mutex 简化版；后续可换 r2d2 连接池）
pub struct Db {
    pub conn: Mutex<Connection>,
    pub path: PathBuf,
}

impl Db {
    /// 在指定数据目录初始化数据库，跑 migrations
    pub fn open(data_dir: &PathBuf) -> Result<Self> {
        std::fs::create_dir_all(data_dir)
            .with_context(|| format!("创建数据目录失败: {}", data_dir.display()))?;

        let db_path = data_dir.join("life-log.db");
        let conn = Connection::open(&db_path)
            .with_context(|| format!("打开数据库失败: {}", db_path.display()))?;

        // WAL 模式：并发读 + 单写，崩溃恢复好
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let db = Db {
            conn: Mutex::new(conn),
            path: db_path,
        };
        db.migrate()?;
        Ok(db)
    }

    /// 简易 migration：维护一张 schema_version 表，按序号跑
    fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().expect("db mutex poisoned");

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY,
                applied_at INTEGER NOT NULL
            );",
        )?;

        let current: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_version",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let migrations: &[(i64, &str)] = &[(1, MIGRATION_001_INIT)];

        for (version, sql) in migrations {
            if *version > current {
                tracing::info!("应用 migration v{}", version);
                conn.execute_batch(sql)
                    .with_context(|| format!("migration v{} 失败", version))?;
                let now = chrono::Utc::now().timestamp_millis();
                conn.execute(
                    "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
                    params![version, now],
                )?;
            }
        }

        Ok(())
    }

    /// 简单的 sanity check：表都建好了
    pub fn table_count(&self) -> Result<i64> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
            [],
            |r| r.get(0),
        )?;
        Ok(n)
    }
}

/// v1 初始建表 —— 对应 SPEC §4.2 全部 7 张表 + 索引
const MIGRATION_001_INIT: &str = r#"
-- 事件主表（万物皆 event）
CREATE TABLE events (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  ts              INTEGER NOT NULL,
  type            TEXT    NOT NULL,
  raw_text        TEXT,
  raw_voice_path  TEXT,
  structured      TEXT,
  tags            TEXT,
  media           TEXT,
  source          TEXT,
  llm_provider    TEXT,
  llm_model       TEXT,
  stt_provider    TEXT,
  created_at      INTEGER NOT NULL,
  updated_at      INTEGER NOT NULL,
  deleted_at      INTEGER
);
CREATE INDEX idx_events_ts        ON events(ts);
CREATE INDEX idx_events_type      ON events(type);
CREATE INDEX idx_events_deleted   ON events(deleted_at);
CREATE INDEX idx_events_type_ts   ON events(type, ts);

-- 标签字典
CREATE TABLE tags (
  name        TEXT PRIMARY KEY,
  kind        TEXT,
  use_count   INTEGER DEFAULT 0,
  is_primary  INTEGER DEFAULT 0,
  color       TEXT,
  created_at  INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL
);
CREATE INDEX idx_tags_kind ON tags(kind);

-- 打卡调度
CREATE TABLE checkin_schedule (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  scheduled_at  INTEGER NOT NULL,
  fired_at      INTEGER,
  responded_at  INTEGER,
  event_id      INTEGER,
  status        TEXT NOT NULL,
  FOREIGN KEY (event_id) REFERENCES events(id)
);
CREATE INDEX idx_schedule_status    ON checkin_schedule(status);
CREATE INDEX idx_schedule_scheduled ON checkin_schedule(scheduled_at);

-- K/V 配置
CREATE TABLE settings (
  key   TEXT PRIMARY KEY,
  value TEXT
);

-- AI 报告缓存
CREATE TABLE reports (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  kind          TEXT NOT NULL,
  range_start   INTEGER NOT NULL,
  range_end     INTEGER NOT NULL,
  content_md    TEXT NOT NULL,
  llm_provider  TEXT,
  llm_model     TEXT,
  created_at    INTEGER NOT NULL,
  pinned        INTEGER DEFAULT 0
);
CREATE INDEX idx_reports_range ON reports(range_start, range_end);

-- 任务表
CREATE TABLE tasks (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  title         TEXT    NOT NULL,
  description   TEXT,
  status        TEXT    NOT NULL,
  priority      INTEGER DEFAULT 3,
  tags          TEXT,
  project       TEXT,
  due_at        INTEGER,
  estimate_min  INTEGER,
  created_at    INTEGER NOT NULL,
  updated_at    INTEGER NOT NULL,
  started_at    INTEGER,
  done_at       INTEGER,
  cancelled_at  INTEGER,
  archived_at   INTEGER,
  deleted_at    INTEGER,
  source        TEXT,
  parent_id     INTEGER,
  FOREIGN KEY (parent_id) REFERENCES tasks(id)
);
CREATE INDEX idx_tasks_status   ON tasks(status);
CREATE INDEX idx_tasks_due      ON tasks(due_at);
CREATE INDEX idx_tasks_parent   ON tasks(parent_id);
CREATE INDEX idx_tasks_priority ON tasks(priority);

-- 任务 × 事件 关联
CREATE TABLE task_events (
  task_id     INTEGER NOT NULL,
  event_id    INTEGER NOT NULL,
  relation    TEXT    NOT NULL,
  created_at  INTEGER NOT NULL,
  PRIMARY KEY (task_id, event_id, relation),
  FOREIGN KEY (task_id)  REFERENCES tasks(id) ON DELETE CASCADE,
  FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE
);
CREATE INDEX idx_task_events_event    ON task_events(event_id);
CREATE INDEX idx_task_events_relation ON task_events(relation);
"#;
