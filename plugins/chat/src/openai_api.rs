use crate::mcp_loader::{FunctionCall, FunctionDef, MCPRegistry};
use anyhow::Error;
use kovi::log::error;
use reqwest::Proxy;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

/// 最大 MCP 循环次数
const MAX_MCP_LOOP: usize = 5;

#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    /// MCP functions
    pub functions: Vec<FunctionDef>,
    /// MCP function_call
    pub function_call: Vec<FunctionCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: Message,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Message {
    pub role: ChatRole,
    pub content: String,
    /// MCP Function 名称
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
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
        mcp_registry: &MCPRegistry,
    ) -> Result<ChatResponse, Error> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages: messages.clone(),
            functions: mcp_registry.functions(),
            function_call: mcp_registry.function_calls(),
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
    ) -> Result<String, Error> {
        // 插入系统提示词
        if messages
            .first()
            .map_or(true, |m| !matches!(m.role, ChatRole::System))
        {
            messages.insert(
                0,
                Message {
                    role: ChatRole::System,
                    content: self.system_promote.clone(),
                    name: None,
                },
            );
        }

        for _ in 0..MAX_MCP_LOOP {
            let response = self.request(messages, mcp_registry).await?;

            let choice = response
                .choices
                .first()
                .ok_or(Error::msg("No choice returned"))?;

            // 如果模型调用了 MCP
            if let ChatRole::Function = choice.message.role {
                let func_name = choice
                    .message
                    .name
                    .clone()
                    .ok_or(Error::msg("Function call missing name"))?;
                let args: Value = serde_json::from_str(&choice.message.content)?;
                let result = mcp_registry
                    .registry
                    .get(&func_name)
                    .ok_or(Error::msg(format!("MCP {} not found", func_name)))?
                    .execute(args);

                // 将 MCP 执行结果插入对话
                messages.push(Message {
                    role: ChatRole::Function,
                    content: serde_json::to_string(&result)?,
                    name: Some(func_name),
                });

                continue; // 再次请求模型
            }

            // 模型不再调用 MCP，直接返回 Assistant 消息
            let content = choice.message.content.clone();

            // 把 "\\n" 转换成真实换行
            let content = content.replace("\\n", "\n");

            messages.push(Message {
                role: ChatRole::Assistant,
                content: content.clone(),
                name: None,
            });

            return Ok(content);
        }

        Err(Error::msg("Max MCP loops reached"))
    }
}
