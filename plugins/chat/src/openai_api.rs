use crate::config::Config;
use crate::mcp_loader::MCPRegistry;
use crate::user_manager::{ChatRole, Message, MessageContent};
use anyhow::{Error, anyhow};
use kovi::log::error;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
    system_promote: String,
    temperature: Option<f32>,
    max_output_tokens: Option<u32>,
    http_client: Arc<reqwest::Client>,
    mcp_loader: Arc<Option<MCPRegistry>>,
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
            system_promote: config.system_promote,
            temperature: config.temperature,
            max_output_tokens: config.max_output_tokens,
            http_client,
            mcp_loader,
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
                    content: MessageContent::Text(self.system_promote.clone()),
                },
            );
        }

        // MCP (未实现)
        let _ = self.mcp_loader;

        // 发送请求
        let response = match self.request(messages).await {
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
        // let content = content.replace("\\n", "\n");
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
