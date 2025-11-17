use crate::commands::*;
use kovi::log::info;
use sysinfo::{Disks, System};

/// 创建命令结构体
pub struct StatusCommand;

impl Command for StatusCommand {
    /// 命令名称
    fn name(&self) -> &'static str {
        "status"
    }
    /// 命令描述
    fn description(&self) -> &'static str {
        "查询服务器状态"
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
    ) -> bool {
        // 匹配命令则返回 true (返回为 true 时不进行 AI 回复)
        if text.trim() == "status" {
            info!("User {} query server status", user.id);
            let reply = KoviMsg::from(server_status());
            msg.reply(reply);
            true
        } else {
            false
        }
    }
}

fn server_status() -> String {
    let mut sys = System::new_all();
    sys.refresh_all();

    // CPU
    let cpu_usage = sys.global_cpu_usage();

    // Memory
    let total_mem = sys.total_memory() / 1024 / 1024;
    let used_mem = sys.used_memory() / 1024 / 1024;

    // Uptime
    let uptime = System::uptime();
    let days = uptime / 86400;
    let hours = (uptime % 86400) / 3600;
    let minutes = (uptime % 3600) / 60;

    // Disk（取第一个磁盘）
    let disks = Disks::new_with_refreshed_list();
    let (disk_used, disk_total) = if let Some(d) = disks.first() {
        let total = d.total_space() / 1024 / 1024 / 1024;
        let avail = d.available_space() / 1024 / 1024 / 1024;
        (total - avail, total)
    } else {
        (0, 0)
    };

    // 打包成字符串
    format!(
        "🖥️ Server Status\n\
         ⏱️ Uptime: {}d {}h {}m\n\
         🔥 CPU Usage: {:.1}%\n\
         📦 Memory: {}MB / {}MB\n\
         💾 Disk: {}GB / {}GB\n\
         🌐 Processes: {}",
        days,
        hours,
        minutes,
        cpu_usage,
        used_mem,
        total_mem,
        disk_used,
        disk_total,
        sys.processes().len()
    )
}
