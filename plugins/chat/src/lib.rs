mod commands;
mod config;
mod function_register;
mod mcp_loader;
mod message;
mod openai_api;
mod user_manager;

use crate::commands::{CommandRegistry, KoviMsg};
use crate::config::Config;
use crate::function_register::{register_commands, register_mcp};
use crate::mcp_loader::MCPRegistry;
use crate::message::OneBotMessage;
use crate::openai_api::{
    ChatRole, ContentPart, Message as OpenaiMsg, MessageContent, OpenaiClient,
};
use crate::user_manager::UserManager;
use anyhow::Error;
use kovi::log::{error, info};
use kovi::{Message, MsgEvent, PluginBuilder as plugin, PluginBuilder};
use std::path::PathBuf;
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

    plugin::on_notice({
        move |event| async move {
            println!("{}", event.original_json);
            event
        }
    });

    // 回应消息
    let data_path_arc = Arc::new(data_path);
    plugin::on_msg({
        // 第一次 clone
        let user_manager = Arc::clone(&user_manager);
        let client = Arc::clone(&client);
        let mcp_loader = Arc::clone(&mcp_loader);
        let commands = Arc::clone(&commands);
        let data_path = Arc::clone(&data_path_arc);

        // move
        move |event| {
            // 第二次 clone
            msg_handler(
                event,
                user_manager.clone(),
                client.clone(),
                mcp_loader.clone(),
                commands.clone(),
                data_path.clone(),
            )
        }
    });
}

async fn msg_handler(
    event: Arc<MsgEvent>,
    user_manager: Arc<UserManager>,
    client: Arc<OpenaiClient>,
    mcp_loader: Arc<MCPRegistry>,
    commands: Arc<CommandRegistry>,
    data_path: Arc<PathBuf>,
) -> Result<(), Error> {
    let origin_json =
        OneBotMessage::from_json(&event.original_json).expect("Failed to parse message");
    let images = origin_json.find_image();

    // 判断是否群聊被 At，私聊不需要 At
    if event.is_group() && !origin_json.is_at(event.self_id) {
        return Ok(());
    }

    // 打开数据库
    let mut user = user_manager.load_user(event.sender.user_id).await?;

    // 获取消息文本
    let text = match event.borrow_text() {
        Some(t) => t,
        None => {
            if images.is_empty() {
                return Ok(());
            } else {
                ""
            }
        }
    };

    // 处理指令
    if commands.handle(text, &event, &mut user, &*data_path) {
        // 保存用户数据
        user_manager.save_user(&user).await?;
        return Ok(());
    }

    // 构造消息列表
    if images.is_empty() {
        user.history.push(OpenaiMsg {
            role: ChatRole::User,
            content: MessageContent::Text(text.to_string()),
        })
    } else {
        let mut multi: Vec<ContentPart> = vec![ContentPart {
            kind: "text".to_string(),
            text: Some(text.to_string()),
            image_url: None,
        }];
        for i in images {
            multi.push(ContentPart {
                kind: "image_url".to_string(),
                text: None,
                image_url: Some(i),
            })
        }
        user.history.push(OpenaiMsg {
            role: ChatRole::User,
            content: MessageContent::Multi(multi),
        })
    }

    // AI 回复
    match client.chat(&mut user.history, mcp_loader.as_ref()).await {
        Ok(reply) => {
            info!("Reply {} : {:?}", user.id, reply);
            let reply = match reply {
                MessageContent::Text(v) => KoviMsg::from(v),
                MessageContent::Multi(v) => {
                    // 为什么会返回图片？？？
                    KoviMsg::from_value(serde_json::to_value(v)?)?
                }
            };
            if event.is_group() {
                let reply = Message::from(reply).add_reply(event.message_id);
                event.reply(reply)
            } else {
                event.reply(reply)
            };
        }
        Err(e) => {
            error!("An error occurred: {:?}", e);
        }
    }

    // 保存用户数据
    user_manager.save_user(&user).await?;

    Ok(())
}
