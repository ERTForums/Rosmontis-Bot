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
use crate::openai_api::OpenaiClient;
use crate::user_manager::UserManager;
use crate::user_manager::{ChatRole, ContentPart, Message as OpenaiMsg, MessageContent};
use anyhow::Error;
use kovi::log::{error, info};
use kovi::{Message, MsgEvent, NoticeEvent, PluginBuilder as plugin, PluginBuilder, RuntimeBot};
use reqwest::Proxy;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

#[kovi::plugin]
async fn main() {
    // 获得储存空间
    let bot = PluginBuilder::get_runtime_bot();
    let data_path = bot.get_data_path();

    // 读取配置文件
    let config = Config::from_file(data_path.join("config.toml"));

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

    // 构建 HTTP 客户端
    let builder = reqwest::Client::builder()
        .pool_max_idle_per_host(20)
        .pool_idle_timeout(Duration::from_secs(270))
        .timeout(Duration::from_secs(90))
        .connect_timeout(Duration::from_secs(30))
        .tcp_nodelay(true);
    let http_client = Arc::new(match &config.proxy {
        None => builder.build().expect("Failed to build reqwest client"),
        Some(v) => builder
            .proxy(Proxy::all(v).expect("Failed to connect to proxy"))
            .build()
            .expect("Failed to build reqwest client"),
    });

    // 注册命令
    let mut commands = CommandRegistry::default();
    register_commands(&mut commands);
    let commands = Arc::new(commands);
    info!("Commands loaded");

    // 创建 MCP 加载器
    let mut mcp_loader = MCPRegistry::new();
    register_mcp(&mut mcp_loader);
    let mcp_loader = Arc::new(Some(mcp_loader));
    info!("MCP functions loaded");

    // 创建 OpenAI 客户端
    let client = Arc::new(OpenaiClient::build(config, http_client, mcp_loader).await);
    info!("OpenAI Client loaded");

    // 回应戳一戳
    plugin::on_notice({
        let user_manager = Arc::clone(&user_manager);
        let client = Arc::clone(&client);
        let bot = Arc::clone(&bot);
        move |event| notice_handler(event, user_manager.clone(), client.clone(), bot.clone())
    });

    // 回应消息
    let data_path_arc = Arc::new(data_path);
    plugin::on_msg({
        // 第一次 clone
        let user_manager = Arc::clone(&user_manager);
        let client = Arc::clone(&client);
        let commands = Arc::clone(&commands);
        let data_path = Arc::clone(&data_path_arc);

        // move
        move |event| {
            // 第二次 clone
            msg_handler(
                event,
                user_manager.clone(),
                client.clone(),
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

    // 打开数据库
    let mut user = user_manager.load_user(event.sender.user_id).await?;

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
            // 下载并编码
            /*
            // 下载图片
            let byte_img = reqwest::Client::new()
                .get(i)
                .send()
                .await?
                .bytes()
                .await?
                .to_vec();
            // 识别图片类型
            let mime_type = if byte_img.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
                "image/png"
            } else if byte_img.starts_with(&[0xFF, 0xD8, 0xFF]) {
                "image/jpeg"
            } else if byte_img.starts_with(b"GIF8") {
                "image/gif"
            } else if byte_img.starts_with(b"RIFF") && byte_img[8..12].eq(b"WEBP") {
                "image/webp"
            } else {
                "application/octet-stream" // 实在认不出来
            };
            // Base64
            let base64_img = base64::engine::general_purpose::STANDARD.encode(byte_img);
            // 插入图片
            multi.push(ContentPart {
                kind: "image_url".to_string(),
                text: None,
                image_url: Some(format!("data:{};base64,{}", mime_type, base64_img)),
            })
            */
            // 直接使用 URL
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

    match client.chat(&mut user.history).await {
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

async fn notice_handler(
    event: Arc<NoticeEvent>,
    user_manager: Arc<UserManager>,
    client: Arc<OpenaiClient>,
    bot: Arc<RuntimeBot>,
) -> Result<(), Error> {
    #[derive(Deserialize)]
    struct Notice {
        group_id: Option<i64>,
        user_id: i64,
        sub_type: String,
        target_id: i64,
    }
    let notice = serde_json::from_value::<Notice>(event.original_json.clone())?;
    if notice.sub_type != "poke" || notice.target_id != event.self_id {
        return Ok(());
    }

    info!("User {} send a poke", notice.user_id);

    // 打开数据库
    let mut user = user_manager.load_user(notice.user_id).await?;

    // 构造消息列表
    user.history.push(OpenaiMsg {
        role: ChatRole::User,
        // 暂时仅支持默认文本
        content: MessageContent::Text("(戳一戳)".to_string()),
    });

    // 获取 AI 回复
    let reply = client.chat(&mut user.history).await?;
    // 仅处理文本回复
    let reply = KoviMsg::from(if let MessageContent::Text(v) = reply {
        v
    } else {
        return Err(Error::msg("Reply contain Multi"));
    });

    info!("Reply {} : {:?}", user.id, reply);
    if let Some(group) = notice.group_id {
        bot.send_group_msg(group, reply)
    } else {
        bot.send_private_msg(notice.user_id, reply)
    }

    // 保存用户数据
    user_manager.save_user(&user).await?;

    Ok(())
}
