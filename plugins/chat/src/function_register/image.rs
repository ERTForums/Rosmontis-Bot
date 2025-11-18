use crate::commands::*;
use anyhow::{Error, anyhow};
use kovi::log::{error, info};
use rand::rng;
use rand::seq::IndexedRandom;
use std::fs;

/// 创建命令结构体
pub struct ImageCommand;

impl Command for ImageCommand {
    /// 命令名称
    fn name(&self) -> &'static str {
        "image"
    }
    /// 命令描述
    fn description(&self) -> &'static str {
        "随机香图"
    }
    /// 执行命令
    fn execute(
        &self,
        // 文本信息
        text: &str,
        // 原始的 MsgEvent
        msg: &Arc<MsgEvent>,
        // 用户信息，目前包含 ID 和与 AI 的聊天记录
        user: &mut User,
        // 命令注册器，用于查看或调用其他命令
        _registry: &CommandRegistry,
        data_dir: PathBuf,
    ) -> bool {
        // 匹配命令则返回 true (返回为 true 时不进行 AI 回复)
        if text.trim() == "image" {
            user.history.clear();
            info!("User {} cleared history", user.id);

            // 判断目录是否存在
            if !data_dir.is_dir() {
                error!("There is no image library at {}", data_dir.display());
                let _ = fs::create_dir_all(data_dir);
                return true;
            }

            // 抽取图片
            let image = match random_file_in_dir(data_dir) {
                Ok(v) => v,
                Err(e) => {
                    error!("Failed to get image: {}", e);
                    return true;
                }
            };

            let reply = KoviMsg::new()
                .add_reply(msg.message_id)
                .add_image(format!("file://{}", image.display()).as_str());
            msg.reply(reply);

            true
        } else {
            false
        }
    }
}

fn random_file_in_dir(dir: PathBuf) -> Result<PathBuf, Error> {
    let mut files = Vec::new();

    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            files.push(path);
        }
    }

    let mut rng = rng();
    files
        .choose(&mut rng)
        .cloned()
        .ok_or_else(|| anyhow!("no files found in directory: {:?}", dir))
}
