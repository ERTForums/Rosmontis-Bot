use anyhow::Error;
use kovi::log::{error, info};
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
    pub max_tokens: Option<u32>,
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
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Function,
}

/// OpenAI 客户端
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

    async fn request(&self, messages: &mut Vec<Message>) -> Result<ChatResponse, Error> {
        // 添加系统提示词
        if let Some(first) = messages.first() {
            if let ChatRole::System = first.role {
            } else {
                messages.insert(
                    0,
                    Message {
                        role: ChatRole::System,
                        content: self.system_promote.clone(),
                    },
                )
            }
        } else {
            messages.push(Message {
                role: ChatRole::System,
                content: self.system_promote.clone(),
            });
        }

        // 构造请求体
        let request = ChatRequest {
            model: self.model.clone(),
            messages: messages.clone(),
            temperature: self.temperature,
            max_tokens: self.max_output_tokens,
        };

        // 发送请求
        let response = self
            .http_client
            .post(&self.api_url)
            .bearer_auth(&self.bearer_token)
            .json(&request)
            .send()
            .await?
            .text()
            .await?;

        // 如果反序列化失败直接输出
        let json = match serde_json::from_str::<ChatResponse>(response.as_str()) {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to parse json:");
                eprintln!("{:?}", response);
                return Err(Error::msg(e));
            }
        };

        // 返回
        Ok(json)
    }

    pub async fn chat(&self, messages: &mut Vec<Message>) -> Result<String, Error> {
        // 获取回复
        let response = self.request(messages).await?;

        // 获取日志消息
        let usage = response.usage.as_ref();
        let prompt = usage.map(|u| u.prompt_tokens).unwrap_or(0);
        let completion = usage.map(|u| u.completion_tokens).unwrap_or(0);
        let total = usage.map(|u| u.total_tokens).unwrap_or(0);

        // 只取第一个 choice（正常请求只有一个）
        let msg = response.choices.first();

        // 回复内容
        let content = msg
            .ok_or(Error::msg("The API returns an error: Choice is empty"))?
            .message
            .content
            .replace('\n', "\\n")
            .clone();

        // 结束原因
        let finish = msg
            .and_then(|c| c.finish_reason.as_deref())
            .unwrap_or("<none>");

        // 输出日志
        info!(
            "OpenAIResp id={} model={} finish={} prompt={} completion={} total={} content=\"{}\"",
            response.id, response.model, finish, prompt, completion, total, content
        );

        // 添加回复
        messages.push(Message {
            role: ChatRole::Assistant,
            content: content.clone(),
        });

        Ok(content.replace("\\n", "\n"))
    }
}
