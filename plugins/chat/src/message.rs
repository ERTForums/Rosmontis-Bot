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
}
