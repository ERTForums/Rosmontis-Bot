use crate::config::Config;
use crate::mcp_loader::MCPRegistry;
use crate::user_manager::{ChatRole, Message, MessageContent};
use anyhow::{Error, anyhow};
use kovi::log::error;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tiktoken_rs::o200k_base;

#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    pub choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
pub struct ChatChoice {
    pub message: Message,
}

pub struct OpenaiClient {
    api_url: String,
    bearer_token: String,
    model: String,
    system_prompt: String,
    temperature: Option<f32>,
    max_output_tokens: Option<u32>,
    http_client: Arc<reqwest::Client>,
    mcp_loader: Arc<Option<MCPRegistry>>,
    msg_limit: usize,
    token_limit: usize,
}

impl OpenaiClient {
    /// 构建 OpenAI 客户端
    pub async fn build(
        config: Config,
        http_client: Arc<reqwest::Client>,
        mcp_loader: Arc<Option<MCPRegistry>>,
    ) -> Self {
        OpenaiClient {
            api_url: config.api_url,
            bearer_token: config.bearer_token,
            model: config.model,
            system_prompt: config.system_prompt,
            temperature: config.temperature,
            max_output_tokens: config.max_output_tokens,
            http_client,
            mcp_loader,
            msg_limit: config.msg_limit.unwrap_or(0),
            token_limit: config.token_limit.unwrap_or(0),
        }
    }

    /// 发送 API 请求
    async fn request(&self, messages: &[Message]) -> Result<ChatResponse, Error> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages: messages.to_vec(),
            temperature: self.temperature,
            max_output_tokens: self.max_output_tokens,
        };

        let response_text = self
            .http_client
            .post(&self.api_url)
            .bearer_auth(&self.bearer_token)
            .json(&request)
            .send()
            .await?
            .text()
            .await?;

        let response: ChatResponse = match serde_json::from_str(&response_text) {
            Ok(v) => v,
            Err(e) => {
                error!(
                    "Failed to parse JSON from OpenAI response:\n{}",
                    response_text
                );
                return Err(Error::msg(e));
            }
        };

        Ok(response)
    }

    /// 使用 API 进行聊天
    pub async fn chat(&self, messages: &mut Vec<Message>) -> Result<MessageContent, Error> {
        // 插入系统提示词
        if messages
            .first()
            .map_or(true, |m| !matches!(m.role, ChatRole::System))
        {
            messages.insert(
                0,
                Message {
                    role: ChatRole::System,
                    content: MessageContent::Text(self.system_prompt.clone()),
                },
            );
        }

        // MCP (未实现)
        let _ = self.mcp_loader;

        // 发送请求
        let response = match self
            .request(&*history_preprocessing(
                messages,
                self.msg_limit,
                self.token_limit,
            ))
            .await
        {
            Ok(r) => r,
            Err(e) => {
                return Err(anyhow!("OpenAI request failed: {}", e));
            }
        };

        // 检查 choice
        let choice = match response.choices.first() {
            Some(c) => c,
            None => {
                return Err(Error::msg("No choice returned from OpenAI"));
            }
        };

        let content = choice.message.content.clone();

        // 把 "\\n" 转换成真实换行
        let content = match content {
            MessageContent::Text(v) => MessageContent::Text(v.replace("\\n", "\n")),
            MessageContent::Multi(v) => MessageContent::Multi(v),
        };

        // 插入历史记录
        messages.push(Message {
            role: ChatRole::Assistant,
            content: content.clone(),
        });

        Ok(content)
    }
}

/// 预处理历史记录
fn history_preprocessing(
    history: &[Message],
    msg_limit: usize,
    token_limit: usize,
) -> Vec<Message> {
    let bpe = o200k_base().unwrap();

    // 分离系统提示词
    let system_prompt = history.iter().filter(|x| x.role == ChatRole::System);
    let chat_prompt = history.iter().filter(|x| x.role != ChatRole::System);

    let mut token_count: usize = 0;
    let mut msg_count: usize = 0;
    let processed = chat_prompt.rev().take_while(|x| {
        token_count += match &x.content {
            MessageContent::Text(v) => bpe.encode_with_special_tokens(&*v).len(),
            _ => 0,
        };
        msg_count += 1;

        let msg_check = msg_limit == 0 || msg_count <= msg_limit;
        let token_check = token_limit == 0 || token_count <= token_limit;

        // 同时超出两个限制才会剪去
        msg_check || token_check
    });

    let mut history_rev: Vec<Message> = processed.chain(system_prompt.rev()).cloned().collect();

    history_rev.reverse();
    history_rev
}
