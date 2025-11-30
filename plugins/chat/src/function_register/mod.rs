mod image;
mod seedream;
mod status;

pub use crate::commands::CommandRegistry;
pub use crate::mcp_loader::MCPRegistry;

/// 在此注册命令
pub fn register_commands(commands_reg: &mut CommandRegistry) {
    // 注册自定义命令
    // commands_reg.register(/* cmd */);

    // 服务器状态命令
    use status::StatusCommand;
    commands_reg.register(StatusCommand);
    // 随机香图命令
    use image::ImageCommand;
    commands_reg.register(ImageCommand);
    // 豆包生成图像命令
    use seedream::SeedreamCommand;
    commands_reg.register(SeedreamCommand)
}

/// 在此注册 MCP
pub fn register_mcp(mcp_loader: &mut MCPRegistry) {
    // 注册自定义 MCP
    // mcp_loader.register(/* mcp */);
}
