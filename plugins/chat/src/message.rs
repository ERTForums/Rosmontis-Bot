use anyhow::Error;
use serde::Deserialize;
use serde_json;
use serde_json::Value;

#[derive(Deserialize)]
pub struct OneBotMessage {
    message: Vec<OneBotMessageBody>,
}
#[derive(Deserialize)]
struct OneBotMessageBody {
    data: OneBotMessageData,
    #[serde(rename = "type")]
    msg_type: String,
}

#[derive(Deserialize)]
struct OneBotMessageData {
    qq: Option<String>,
    url: Option<String>,
}

impl OneBotMessage {
    /// 从 Value 解析 JSON
    pub fn from_json(json: &Value) -> Result<OneBotMessage, Error> {
        Ok(serde_json::from_value::<Self>(json.clone())?)
    }

    /// 判断自己是否被 At
    pub fn is_at(&self, self_id: i64) -> bool {
        if self
            .message
            .iter()
            .filter(|msg| msg.msg_type == "at")
            .filter_map(|msg| msg.data.qq.as_deref())
            .filter_map(|qq| qq.parse::<i64>().ok())
            .any(|id| id == self_id)
        {
            true
        } else {
            false
        }
    }

    /// 检查是否包含图片，返回 URL 列表
    pub fn find_image(&self) -> Vec<String> {
        let mut image_url = vec![];
        for i in &self.message {
            if i.msg_type == "image" {
                image_url.push(i.data.url.clone().expect("Failed to get url of image"))
            }
        }
        image_url
    }
}
