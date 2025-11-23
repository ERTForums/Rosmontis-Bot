use anyhow::Error;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::path::PathBuf;

/// 消息
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Message {
    pub role: ChatRole,
    pub content: MessageContent,
}

/// 消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Multi(Vec<ContentPart>),
}

/// 非文本的消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPart {
    #[serde(rename = "type")]
    pub kind: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
}

/// 角色
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Function,
}

/// 用户数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// 用户 ID
    pub id: i64,
    /// 用户聊天历史
    pub history: Vec<Message>,
}

/// 用户管理器
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
