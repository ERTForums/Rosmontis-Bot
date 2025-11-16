use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// MCP trait
pub trait MCP {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn parameters(&self) -> serde_json::Value;
    fn execute(&self, args: serde_json::Value) -> serde_json::Value;
}

/// MCP 注册器
#[derive(Default)]
pub struct MCPRegistry {
    pub(crate) registry: HashMap<String, Box<dyn MCP + Send + Sync>>,
}

impl MCPRegistry {
    pub fn new() -> Self {
        Self {
            registry: HashMap::new(),
        }
    }

    /// 注册 MCP
    pub fn register<M: MCP + Send + Sync + 'static>(&mut self, mcp: M) {
        self.registry.insert(mcp.name().to_string(), Box::new(mcp));
    }

    /// 获取所有 MCP 的 functions 列表
    pub fn functions(&self) -> Vec<FunctionDef> {
        self.registry
            .values()
            .map(|m| FunctionDef {
                name: m.name().to_string(),
                description: m.description().to_string(),
                parameters: m.parameters(),
            })
            .collect()
    }

    /// 获取 function_call 列表，返回所有注册的 MCP
    pub fn function_calls(&self) -> Vec<FunctionCall> {
        self.registry
            .keys()
            .map(|n| FunctionCall { name: n.clone() })
            .collect()
    }
}

/// 序列化的函数定义
#[derive(Serialize, Deserialize, Debug)]
pub struct FunctionDef {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// 序列化的 function_call
#[derive(Serialize, Deserialize, Debug)]
pub struct FunctionCall {
    pub name: String,
}

#[cfg(test)]
mod tests {
    use crate::mcp_loader::*;
    use serde_json::json;

    struct SumMCP;

    impl MCP for SumMCP {
        fn name(&self) -> &'static str {
            "calculate_sum"
        }

        fn description(&self) -> &'static str {
            "计算两个整数的和"
        }

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

        fn execute(&self, args: serde_json::Value) -> serde_json::Value {
            let a = args.get("a").and_then(|v| v.as_i64()).unwrap_or(0);
            let b = args.get("b").and_then(|v| v.as_i64()).unwrap_or(0);
            json!({ "result": a + b })
        }
    }

    #[test]
    fn test_mcp_registry() {
        let mut registry = MCPRegistry::new();
        registry.register(SumMCP);

        let funcs = registry.functions();
        println!(
            "Functions: {}",
            serde_json::to_string_pretty(&funcs).unwrap()
        );

        let calls_all = registry.function_calls();
        println!(
            "Function Calls (all): {}",
            serde_json::to_string_pretty(&calls_all).unwrap()
        );

        let calls_some = registry.function_calls();
        println!(
            "Function Calls (selected): {}",
            serde_json::to_string_pretty(&calls_some).unwrap()
        );
    }
}
