use crate::user_manager::User;
use kovi::log::info;
use kovi::Message as KoviMsg;
use serde_json::Value;
use std::collections::HashMap;

/// 命令 Trait
pub trait Command: Send + Sync {
    /// 命令名称
    fn name(&self) -> &'static str;

    /// 命令描述
    fn description(&self) -> &'static str;

    /// 执行命令，如果命令匹配返回 true
    fn execute(
        &self,
        msg: &str,
        raw_msg: &Value,
        user: &mut User,
        registry: &CommandRegistry,
        reply: &mut dyn FnMut(KoviMsg),
    ) -> bool;
}

/// 命令注册器
pub struct CommandRegistry {
    commands: HashMap<String, Box<dyn Command>>,
}

impl CommandRegistry {
    /// 创建空注册器
    fn new() -> Self {
        CommandRegistry {
            commands: HashMap::new(),
        }
    }

    /// 注册命令
    pub fn register<C: Command + 'static>(&mut self, cmd: C) {
        self.commands.insert(cmd.name().to_string(), Box::new(cmd));
    }

    /// 处理消息，返回 true 表示命令已处理，不再 AI 回复
    pub fn handle(
        &self,
        msg: &str,
        raw_msg: &Value,
        user: &mut User,
        reply: &mut dyn FnMut(KoviMsg),
    ) -> bool {
        for cmd in self.commands.values() {
            if cmd.execute(msg, raw_msg, user, self, reply) {
                return true;
            }
        }
        false
    }

    /// 获取所有命令及描述
    pub fn list_commands(&self) -> Vec<(String, String)> {
        self.commands
            .values()
            .map(|c| (c.name().to_string(), c.description().to_string()))
            .collect()
    }
}

/// --------- 内置命令 ---------

/// help 命令，不持有注册器引用
pub struct HelpCommand;

impl Command for HelpCommand {
    fn name(&self) -> &'static str {
        "help"
    }

    fn description(&self) -> &'static str {
        "显示所有可用命令及说明"
    }

    fn execute(
        &self,
        msg: &str,
        _raw: &Value,
        _user: &mut User,
        registry: &CommandRegistry,
        reply: &mut dyn FnMut(KoviMsg),
    ) -> bool {
        if msg.trim() == "help" {
            let commands = registry.list_commands();
            let mut output = String::from("可用命令:\n");
            for (name, desc) in commands {
                output.push_str(&format!("{}: {}\n", name, desc));
            }
            let msg = KoviMsg::from(&output);
            reply(msg);
            true
        } else {
            false
        }
    }
}
/// clear 命令
pub struct ClearCommand;

impl Command for ClearCommand {
    fn name(&self) -> &'static str {
        "clear"
    }

    fn description(&self) -> &'static str {
        "清空用户历史记录"
    }

    fn execute(
        &self,
        msg: &str,
        _raw: &Value,
        user: &mut User,
        _registry: &CommandRegistry,
        reply: &mut dyn FnMut(KoviMsg),
    ) -> bool {
        if msg.trim() == "clear" {
            user.history.clear();
            info!("User {} cleared history", user.id);
            let msg = KoviMsg::from("历史记录已清理");
            reply(msg);
            true
        } else {
            false
        }
    }
}

/// 默认注册内置命令
impl Default for CommandRegistry {
    fn default() -> Self {
        let mut registry = CommandRegistry::new();
        registry.register(ClearCommand);
        registry.register(HelpCommand);
        registry
    }
}
