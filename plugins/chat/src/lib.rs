mod commands;
mod config;
mod message;
mod openai_api;
mod user_manager;

use crate::commands::command;
use crate::config::Config;
use crate::message::OneBotMessage;
use crate::openai_api::{ChatRole, Message, OpenaiClient};
use crate::user_manager::UserManager;
use anyhow::Error;
use kovi::log::{error, info};
use kovi::{PluginBuilder as plugin, PluginBuilder};
use std::sync::Arc;

#[kovi::plugin]
async fn main() {
    // 获得储存空间
    let bot = PluginBuilder::get_runtime_bot();
    let data_path = bot.get_data_path();

    // 读取配置文件
    let config = Config::from_file(data_path.join("config.toml"));

    // 创建 OpenAI 客户端
    let client = Arc::new(
        OpenaiClient::build(
            config.api_url,
            config.bearer_token,
            config.model,
            config.system_promote,
            config.temperature,
            config.max_output_tokens,
            config.proxy,
        )
        .await,
    );

    // 打开用户管理器
    let user_manager = match UserManager::open(data_path.join("users.db")).await {
        Ok(v) => {
            info!("User manager loaded");
            Arc::new(v)
        }
        Err(e) => {
            error!("Failed to open user manager: {}", e);
            return;
        }
    };

    // 回应消息
    plugin::on_msg({
        let user_manager_clone = Arc::clone(&user_manager);
        let client_clone = Arc::clone(&client);

        move |event| {
            {
                let user_manager = Arc::clone(&user_manager_clone);
                let client = Arc::clone(&client_clone);

                async move {
                    // 判断是否群聊被 At，私聊不需要 At
                    if event.is_group()
                        && !OneBotMessage::from_json(&event.original_json)
                            .expect("Failed to parse message")
                            .is_at(event.self_id)
                    {
                        return Ok(());
                    }

                    // 获取消息文本
                    let text = match event.borrow_text() {
                        Some(t) => t,
                        None => return Ok(()),
                    };

                    // 打开数据库
                    let mut user = user_manager.load_user(event.sender.user_id).await?;

                    // 处理指令
                    if command(text, &mut user, |x| event.reply(x)) {
                        return Ok(());
                    }

                    // 构造消息列表
                    user.history.push(Message {
                        role: ChatRole::User,
                        content: text.to_string(),
                    });

                    // AI 回复
                    match client.chat(&mut user.history).await {
                        Ok(reply) => {
                            info!("Reply {} : {}", user.id, reply);
                            event.reply(reply);
                        }
                        Err(e) => {
                            error!("An error occurred: {:?}", e);
                        }
                    }

                    // 保存用户数据
                    user_manager.save_user(&user).await?;

                    // Event 结束
                    Ok::<(), Error>(())
                }
            }
        }
    });
}
