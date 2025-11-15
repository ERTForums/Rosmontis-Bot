use crate::openai_api::Message;
use anyhow::Error;
use kovi::utils::{load_json_data, save_json_data};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub(crate) id: i64,
    pub(crate) history: Vec<Message>,
}

/// 用户管理器
#[derive(Debug, Serialize, Deserialize)]
pub struct UserManager {
    pub(crate) repository: PathBuf,
    pub(crate) user_list: Vec<User>,
}

impl UserManager {
    /// 打开用户仓库
    pub async fn open(path: PathBuf) -> Result<Self, Error> {
        let user_list = load_json_data(vec![], &path).map_err(|e| Error::msg(e.to_string()))?;

        Ok(UserManager {
            repository: path,
            user_list,
        })
    }

    /// 保存用户仓库
    pub async fn save(&self) -> Result<(), Error> {
        save_json_data::<Vec<User>, &PathBuf>(&self.user_list, &self.repository)
            .map_err(|e| Error::msg(e.to_string()))
    }

    /// 自动创建用户并返回可变引用
    pub fn auto(&mut self, id: i64) -> &mut User {
        if let Some(pos) = self.user_list.iter().position(|u| u.id == id) {
            &mut self.user_list[pos]
        } else {
            self.user_list.push(User {
                id,
                history: vec![],
            });
            self.user_list.last_mut().unwrap()
        }
    }
}
