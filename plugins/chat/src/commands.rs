use crate::user_manager::User;
use kovi::log::info;

/// 基于规则处理命令，返回 true 不再 AI 回复
pub fn command<F>(msg: &str, user: &mut User, reply: F) -> bool
where
    F: Fn(&str),
{
    match msg.trim() {
        "help" => {
            reply("");
            true
        }
        "clear" => {
            user.history = vec![];
            info!("User {} cleared history", user.id);
            reply("历史记录已清理");
            true
        }
        _ => false,
    }
}
