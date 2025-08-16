use log::info;
use tokio;
use zai_rs::client::http::*;
use zai_rs::model::base::*;
use zai_rs::model::chat::data::ChatCompletion;
#[tokio::main]
async fn main() {
    env_logger::init();

    let model = GLM4_5_flash {};

    // 模拟添加一个 function call 工具：get_weather(city: string)
    let weather_func = Function::new(
        "get_weather",
        "Get current weather for a city",
        serde_json::json!({
            "type": "object",
            "properties": {
                "city": {"type": "string"}
            },
            "required": ["city"],
            "additionalProperties": false
        }),
    );
    let tools = Tools::Function {
        function: weather_func,
    };

    // 读取 API Key，并保留以便后续继续对话
    let key = get_key();

    // 会话的第一条用户消息
    let user_text = "你是谁，帮为查找深圳今天的天气";

    let mut client = ChatCompletion::new(model, TextMessage::user(user_text), key.clone())
        .with_thinking(ThinkingType::Disabled)
        .with_temperature(0.7)
        .with_top_p(0.9)
        .with_max_tokens(512)
        .with_tools(vec![tools.clone()]);
    let resp = client.post().await.unwrap();
    let v: serde_json::Value = resp.json().await.unwrap();
    info!("{}", serde_json::to_string_pretty(&v).unwrap());

    // 1) 解析第一条 tool_call（更简洁）
    if let Some((id, name, arguments)) = parse_first_tool_call(&v) {
        info!("提取到的 tool_call -> name: {name}, arguments: {arguments}");

        // 2) 执行本地工具并返回模拟结果
        let result = handle_tool_call(&name, &arguments)
            .unwrap_or_else(|| serde_json::json!({"ok": false, "error": "no_result"}));
        info!(
            "模拟函数返回结果: {}",
            serde_json::to_string_pretty(&result).unwrap()
        );

        // 3) 回传工具结果并继续一轮对话（复用同一个 client）
        let tool_msg = TextMessage::tool_with_id(serde_json::to_string(&result).unwrap(), id);
        client = client
            .add_messages(tool_msg)
            .with_tools(vec![tools.clone()])
            .with_max_tokens(512);

        let resp2 = client.post().await.unwrap();
        let v2: serde_json::Value = resp2.json().await.unwrap();
        info!(
            "继续对话返回: {}",
            serde_json::to_string_pretty(&v2).unwrap()
        );
    } else {
        info!("未发现 tool_calls");
    }
}

fn get_key() -> String {
    if let Ok(key) = std::env::var("ZHIPU_API_KEY") {
        key
    } else {
        // 从输入中读取
        let mut key = String::new();
        std::io::stdin().read_line(&mut key).unwrap();
        key.trim().to_string()
    }
}

/// 从响应中解析第一条 tool_call: 返回 (id, name, arguments)
fn parse_first_tool_call(v: &serde_json::Value) -> Option<(String, String, String)> {
    let tool_calls = v.pointer("/choices/0/message/tool_calls")?.as_array()?;
    let tc0 = tool_calls.get(0)?;
    let id = tc0.get("id")?.as_str()?.to_string();
    let func = tc0.get("function")?;
    let name = func.get("name")?.as_str()?.to_string();
    let arguments = func.get("arguments")?.as_str()?.to_string();
    Some((id, name, arguments))
}

/// 处理工具调用：解析参数并返回模拟结果
fn handle_tool_call(name: &str, arguments: &str) -> Option<serde_json::Value> {
    match name {
        "get_weather" => {
            // arguments 通常是一个 JSON 字符串，如："{\"city\": \"深圳\"}"
            let parsed: serde_json::Value = match serde_json::from_str(arguments) {
                Ok(v) => v,
                Err(err) => {
                    log::warn!("解析 arguments 失败: {} | 原始: {}", err, arguments);
                    return Some(serde_json::json!({
                        "ok": false,
                        "error": "invalid_arguments",
                        "raw": arguments,
                    }));
                }
            };
            let city = parsed
                .get("city")
                .and_then(|v| v.as_str())
                .unwrap_or("未知城市");

            // 返回一个模拟的天气结果
            Some(serde_json::json!({
                "ok": true,
                "name": name,
                "request": { "city": city },
                "result": {
                    "city": city,
                    "condition": "晴",
                    "temperature_c": 28,
                    "humidity": 0.65,
                    "tips": format!("{} 现在户外紫外线较强，注意防晒。", city),
                },
                "source": "mock",
            }))
        }
        _ => {
            // 未知的工具名
            Some(serde_json::json!({
                "ok": false,
                "error": "unknown_tool",
                "name": name,
                "raw_arguments": arguments,
            }))
        }
    }
}
