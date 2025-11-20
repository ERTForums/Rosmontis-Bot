use crate::mcp_loader::MCPRegistry;
use anyhow::{anyhow, Error};
use kovi::log::error;
use reqwest::Proxy;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Message {
    pub role: ChatRole,
    pub content: MessageContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Multi(Vec<ContentPart>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPart {
    #[serde(rename = "type")]
    pub kind: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Function,
}

pub struct OpenaiClient {
    api_url: String,
    bearer_token: String,
    model: String,
    system_promote: String,
    temperature: Option<f32>,
    max_output_tokens: Option<u32>,
    http_client: Arc<reqwest::Client>,
}

impl OpenaiClient {
    /// 构建 OpenAI 客户端
    pub async fn build(
        api: String,
        token: String,
        model: String,
        promote: String,
        temperature: Option<f32>,
        max_output_tokens: Option<u32>,
        proxy: Option<String>,
    ) -> Self {
        let builder = reqwest::Client::builder()
            .pool_max_idle_per_host(20)
            .pool_idle_timeout(Duration::from_secs(90))
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(5))
            .tcp_nodelay(true);

        let client = Arc::new(match proxy {
            None => builder.build().expect("Failed to build reqwest client"),
            Some(v) => builder
                .proxy(Proxy::all(v).expect("Failed to connect to proxy"))
                .build()
                .expect("Failed to build reqwest client"),
        });

        OpenaiClient {
            api_url: api,
            bearer_token: token,
            model,
            system_promote: promote,
            temperature,
            max_output_tokens,
            http_client: client,
        }
    }

    /// 发送 API 请求
    async fn request(
        &self,
        messages: &Vec<Message>,
        _mcp_registry: &MCPRegistry,
    ) -> Result<ChatResponse, Error> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages: messages.clone(),
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
    pub async fn chat(
        &self,
        messages: &mut Vec<Message>,
        mcp_registry: &MCPRegistry,
    ) -> Result<MessageContent, Error> {
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

        let response = match self.request(messages, mcp_registry).await {
            Ok(r) => r,
            Err(e) => {
                return Err(anyhow!("OpenAI request failed: {}", e));
            }
        };

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

        messages.push(Message {
            role: ChatRole::Assistant,
            content: content.clone(),
        });

        Ok(content)
    }
}
