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
use kovi::PluginBuilder as plugin;
use kovi::log::{error, info};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;

#[kovi::plugin]
async fn main() {
    let config = Config::from_file();
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
    let user_manager = match UserManager::open(config.user_repository).await {
        Ok(v) => {
            info!("User manager loaded");
            Arc::new(Mutex::new(v))
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
            let user_manager = Arc::clone(&user_manager_clone);
            let client = Arc::clone(&client_clone);

            async move {
                // 判断是否群聊被 At，私聊不需要 At
                if event.is_group()
                    && !OneBotMessage::from_json(&event.original_json)
                        .expect("Failed to parse message")
                        .is_at(event.self_id)
                {
                    return;
                }

                // 获取消息文本
                let text = match event.borrow_text() {
                    Some(t) => t,
                    None => return,
                };

                // 锁住 UserManager
                let mut manager = user_manager.lock().await;

                // 获取用户的可变引用
                let user = manager.auto(event.sender.user_id);

                // 处理指令
                if command(text, user, |x| event.reply(x)) {
                    return;
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

                // Event 结束
            }
        }
    });

    // 保存用户数据
    kovi::spawn(async move {
        loop {
            // 保存用户管理器
            if let Err(e) = user_manager.lock().await.save().await {
                error!("Failed to save user manager: {}", e);
            }
            info!("User data saved");
            sleep(Duration::from_secs(10)).await
        }
    });
}
