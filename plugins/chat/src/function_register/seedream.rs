use crate::commands::*;
use kovi::log::{error, info};
use serde::{Deserialize, Serialize};

const API_URL: &str = "https://ark.cn-beijing.volces.com/api/v3/images/generations";
const MODEL: &str = "doubao-seedream-4-0-250828";

/// 创建命令结构体
pub struct SeedreamCommand;

impl Command for SeedreamCommand {
    /// 命令名称
    fn name(&self) -> &'static str {
        "seedream"
    }
    /// 命令描述
    fn description(&self) -> &'static str {
        "使用 seedream 模型生成图像"
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
        if text.trim().starts_with("seedream ") {
            info!("User {} generated image", user.id);

            // 尝试从 token.txt 读取 bearer token
            let token_file = data_dir.join("token.txt");
            let token = match std::fs::read_to_string(&token_file) {
                Ok(v) => v,
                Err(_) => {
                    error!(
                        "Failed to get token, please write the token into the {} file",
                        token_file.display()
                    );
                    return true;
                }
            };

            // 解析消息中的图像
            let origin_msg = crate::message::OneBotMessage::from_json(&msg.original_json)
                .expect("Failed to parse message");
            let images = generate_img(
                token,
                text.trim().strip_prefix("seedream ").unwrap().to_string(),
                origin_msg.find_image(),
            );

            // 构造回复
            let mut reply = KoviMsg::new().add_reply(msg.message_id);
            for i in images {
                reply.push_image(&*i);
            }
            msg.reply(reply);

            true
        } else {
            false
        }
    }
}

#[derive(Serialize, Debug)]
struct Request {
    model: &'static str,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    image: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct Response {
    data: Vec<ResData>,
}

#[derive(Deserialize)]
struct ResData {
    url: String,
}

/// 调用 API 生成图像
fn generate_img(token: String, text: String, img: Vec<String>) -> Vec<String> {
    let client = reqwest::blocking::Client::new();
    let request = Request {
        model: MODEL,
        prompt: text,
        image: if img.is_empty() { None } else { Some(img) },
    };
    let response = client
        .post(API_URL)
        .bearer_auth(token)
        .json(&request)
        .send()
        .expect("Failed to request image generation API")
        .text()
        .unwrap();
    let response = match serde_json::from_str::<Response>(&*response) {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to parse image generation response: {e}");
            panic!("{}\n{}", serde_json::to_string(&request).unwrap(), response)
        }
    };
    response.data.into_iter().map(|x| x.url).collect()
}
