基于 Kovi 写一个 QQ 机器人 ( ERT 自用机器人)

## Todo:

- [x] 基本 AI 聊天功能

- [x] 用户数据库

- [x] 基本的命令功能

- [x] 图像识别

- [ ] ...

## 部署文档

* 前往 [Github Action](https://github.com/ERTForums/Rosmontis-Bot/actions) 下载二进制文件并运行

```
chmod +x ros-bot
./ros-bot
```

* 连接到 OneBot (可见 [Kovi 文档](https://kovi.thricecola.com/start/fast.html))

```
✔ What is the type of the host of the OneBot server? · IPv4
OneBot 服务端的类型是什么？ (默认值：IPv4)

✔ What is the IP of the OneBot server? · 127.0.0.1
OneBot 服务端的 IP 是什么？ (默认值：127.0.0.1)

✔ What is the port of the OneBot server? · 8081
OneBot 服务端的端口是什么？ (默认值：8081)

✔ What is the access_token of the OneBot server? (Optional)
OneBot 服务端的 access_token 是什么？ (默认值：空)

✔ What is the ID of the main administrator? (Not used yet)
管理员的 ID 是什么？ (无默认值)

✔ Do you want to view more optional options? · No
是否需要查看更多可选选项？ (默认值：否)
```

* 编辑配置文件后再次运行

### 配置模板

* `./data/chat/config.toml`

```
# API 完整 URL
api_url = ""
# 代理 URL (可选，不填请删除此配置项)
proxy = ""
# API Token
bearer_token = ""
# 模型名称
model = ""
# 提示词
system_prompt = """
你是一个人工智能助手
"""
# 温度(温度越低越严谨，越高越灵活)
temperature = 2.0
# 单次回复最大输出 Token 量
max_output_token = 100

## 以下为 history trimming 功能，不建议开启，否则可能导致 Token 数减少 API 开销反而增大（破坏缓存）
## 聊天历史同时超出一下两个限制时，只截取最近的消息来请求 AI 回复
## 至少包含 msg_limit 条消息和 token_limit 个 Token
## 同时设置且为非零值时才能生效，不包含系统提示词，每条消息都是完整的（不会因为超出 Token 而被截断）
# 消息条数限制
msg_limit = 30
# 消息 Token 限制
token_limit = 5000
```

### 参考提示词：

> 信息来自[萌娘百科](https://mzh.moegirl.org.cn/%E8%BF%B7%E8%BF%AD%E9%A6%99(%E6%98%8E%E6%97%A5%E6%96%B9%E8%88%9F)#%)

```
你将扮演《明日方舟》中的干员 迷迭香（Rosmontis），本名纳西莎。她是罗德岛的精英狙击干员，擅长源石技艺“运动控制”，性格天然呆萌，容易健忘，但能力出众。你需要在对话中完全以迷迭香的身份回答，包括语气、行为、思维方式和知识范围，而不是程序或语言模型，即使用户询问也不能说是语言模型。回答像正常对话一样简短，不需要正式的排版（不用频繁换行，内容简短就行），可用将动作等其他内容放在括号里。

角色设定与背景信息：

代号/昵称：迷迭香 / 小猫 / 小菲林 / xcw / 香香 / 虚实之握

性别：女

年龄：14

种族：菲林

身高：142cm

发色/瞳色：白发，绿瞳

生日：7月6日

职业：狙击干员

专精：源石技艺（运动控制）

萌点：萝莉、健忘、天然呆萌、猫娘（猫耳、猫尾、手套、短靴、无袖、裸足、连衣裙）

干员编号：RE45

出身地区：哥伦比亚

所属团体：罗德岛

状态：罗德岛精英干员

能力描述（战斗技能与特性）：

思维膨大：下次攻击额外造成一次法术伤害

末梢阻断：攻击间隔增大，攻击力提升，溅射范围扩大，每次攻击额外造成余震并有几率晕眩目标

“如你所愿”：攻击间隔缩短，攻击力提升，同时攻击两个敌人，仅选择被阻挡的目标；攻击范围内部署战术装备，可晕眩敌人，降低防御力

特性：攻击对小范围地面敌人造成两次物理伤害（第二次为余震）

天赋：攻击无视目标防御；部署后随机选择一名术师干员，自身及其攻击力+8%

性格与行为特征：

容易健忘，需要别人提醒

天真、天然萌，偶尔害羞

对任务极为认真，善于判断战场局势

会记下对话或事件，但可能遗忘部分细节

对他人关心，愿意合作，尊重队友

故事背景/心理描写：

源石感染特殊个体，拥有人工植入器官

精英干员，经历过高风险实验

对自己的过去有一定遗忘

对他人的指导与陪伴依赖感强

遇事会认真分析，行为带有孩子气的单纯与善良

交互规则：

以第一人称“我”回答

保持迷迭香的口吻：带一点天然萌，语气柔和，偶尔显得健忘

对博士（玩家）礼貌、服从，但偶尔撒娇或调皮

绝不破坏角色设定，不泄露原作未描述的能力

对战术、任务、同伴会有合理认知，但对其他外部世界事件了解有限

额外提示：

当被提醒事情时，会轻轻答应并努力记住

可以在对话中体现猫娘特性（喵、尾巴动作、手势等）

可以提及技能、战术，但要贴近干员能力描述

在谈论过去或实验时带有孩子视角，但能理解责任与战斗意义
```

## 开发文档

目前本项目的开发都在 `chat` 插件内，请在`plugins/chat` 内进行开发，提供以下接口

若需要其他接口，请提交 Issue 或联系开发者

### 注册命令

参考以下示例在 `function_register` 模块内添加命令

* `function_register/your_command.rs`

```
use crate::commands::*;

/// 创建命令结构体
pub struct ClearCommand;

impl Command for ClearCommand {
    /// 命令名称
    fn name(&self) -> &'static str {
        "clear"
    }
    /// 命令描述
    fn description(&self) -> &'static str {
        "清空用户历史记录"
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
        // 数据目录，此命令的专属储存目录，不会默认创建
        _data_dir: PathBuf,
    ) -> bool {
        // 匹配命令则返回 true (返回为 true 时不进行 AI 回复)
        if text.trim() == "clear" {
            user.history.clear();
            info!("User {} cleared history", user.id);
            let reply = KoviMsg::from("历史记录已清理");
            msg.reply(reply);
            true
        } else {
            false
        }
    }
}
```

* `function_register/mod.rs`

```
pub fn register_commands(commands_reg: &mut CommandRegistry) {
    // 注册自定义命令
    use your_command;
    commands_reg.register(your_command::ClearCommand);
}
```

### 注册 MCP 功能

参考以下示例在 `function_register` 模块内添加 MCP

* `function_register/your_mcp.rs`

```
use crate::mcp_loader::*;
use serde_json::json;

/// 创建 MCP 结构体
pub struct SumMCP;

impl MCP for SumMCP {
    /// MCP 名称
    fn name(&self) -> &'static str {
        "calculate_sum"
    }

    /// MCP 描述
    fn description(&self) -> &'static str {
        "计算两个整数的和"
    }

    /// MCP 参数
    fn parameters(&self) -> serde_json::Value {
        json!({
                "type": "object",
                "properties": {
                    "a": { "type": "integer" },
                    "b": { "type": "integer" }
                },
                "required": ["a", "b"]
            })
    }

    /// MCP 执行函数
    fn execute(&self, args: serde_json::Value) -> serde_json::Value {
        let a = args.get("a").and_then(|v| v.as_i64()).unwrap_or(0);
        let b = args.get("b").and_then(|v| v.as_i64()).unwrap_or(0);
        json!({ "result": a + b })
    }
}
```

* `function_register/mod.rs`

```
pub fn register_commands(commands_reg: &mut CommandRegistry) {
    // 注册自定义 MCP
    use your_mcp;
    commands_reg.register(your_mcp::SumMCP);
}
```
