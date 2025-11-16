use crate::openai_api::Message;
use anyhow::Error;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub history: Vec<Message>,
}

pub struct UserManager {
    pool: SqlitePool,
}

impl UserManager {
    // 打开或创建数据库
    pub async fn open(db_path: PathBuf) -> Result<Self, Error> {
        // 确保目录存在
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // 构造创建连接配置
        let options = SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

        // 建立连接池
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        // 创建表
        sqlx::query(
            r#"
        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY,
            history TEXT NOT NULL
        )
        "#,
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }

    /// 保存或更新用户
    pub async fn save_user(&self, user: &User) -> Result<(), Error> {
        let history_json = serde_json::to_string(&user.history)?;
        sqlx::query(
            r#"
            INSERT INTO users (id, history) VALUES (?, ?)
            ON CONFLICT(id) DO UPDATE SET history=excluded.history
            "#,
        )
        .bind(user.id)
        .bind(history_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// 加载用户，如果不存在返回默认用户
    pub async fn load_user(&self, id: i64) -> Result<User, Error> {
        if let Some(row) = sqlx::query("SELECT history FROM users WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
        {
            let history_json: String = row.try_get("history")?;
            let history: Vec<Message> = serde_json::from_str(&history_json)?;
            Ok(User { id, history })
        } else {
            Ok(User {
                id,
                history: Vec::new(),
            })
        }
    }
}
