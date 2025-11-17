mod commands;
mod config;
mod function_register;
mod mcp_loader;
mod message;
mod openai_api;
mod user_manager;

use crate::commands::CommandRegistry;
use crate::config::Config;
use crate::function_register::{register_commands, register_mcp};
use crate::mcp_loader::MCPRegistry;
use crate::message::OneBotMessage;
use crate::openai_api::{ChatRole, Message as OpenaiMsg, OpenaiClient};
use crate::user_manager::UserManager;
use anyhow::Error;
use kovi::log::{error, info};
use kovi::{Message as KoviMsg, PluginBuilder as plugin, PluginBuilder};
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
    info!("OpenAI Client loaded");

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

    // 注册命令
    let mut commands = CommandRegistry::default();
    register_commands(&mut commands);
    let commands = Arc::new(commands);
    info!("Commands loaded");

    // 创建 MCP 加载器
    let mut mcp_loader = MCPRegistry::new();
    register_mcp(&mut mcp_loader);
    let mcp_loader = Arc::new(mcp_loader);
    info!("MCP functions loaded");

    // 回应消息
    plugin::on_msg({
        let user_manager_clone = Arc::clone(&user_manager);
        let client_clone = Arc::clone(&client);
        let mcp_loader_clone = Arc::clone(&mcp_loader);
        let commands_clone = Arc::clone(&commands);

        move |event| {
            {
                let user_manager = Arc::clone(&user_manager_clone);
                let client = Arc::clone(&client_clone);
                let mcp_loader = Arc::clone(&mcp_loader_clone);
                let commands = Arc::clone(&commands_clone);

                async move {
                    // 获取消息文本
                    let text = match event.borrow_text() {
                        Some(t) => t,
                        None => return Ok(()),
                    };

                    // 判断是否群聊被 At，私聊不需要 At
                    if event.is_group()
                        && !OneBotMessage::from_json(&event.original_json)
                            .expect("Failed to parse message")
                            .is_at(event.self_id)
                    {
                        return Ok(());
                    }

                    // 打开数据库
                    let mut user = user_manager.load_user(event.sender.user_id).await?;

                    // 处理指令
                    if commands.handle(text, &event, &mut user) {
                        // 保存用户数据
                        user_manager.save_user(&user).await?;
                        return Ok(());
                    }

                    // 构造消息列表
                    user.history.push(OpenaiMsg {
                        role: ChatRole::User,
                        content: text.to_string(),
                        name: None,
                    });

                    // AI 回复
                    match client.chat(&mut user.history, mcp_loader.as_ref()).await {
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
                    Ok::<(), Error>(())

                    // Event 结束
                }
            }
        }
    });
}
