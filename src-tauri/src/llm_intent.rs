use serde::{Deserialize, Serialize};

use crate::config::Config;

const SYSTEM_PROMPT: &str = r#"你是一个B站直播间的点歌助手。分析弹幕是否包含点歌意图，提取歌名和作者名。

分析规则：
1. 想听某首歌（"点歌xxx"、"我想听xxx"、"来一首xxx"、"放xxx"、"有没有xxx"等）→ is_song_request=true
2. 普通聊天、刷屏、表情等 → is_song_request=false
3. song_name：歌曲名称本身，去掉"点歌"等前缀。若只提到作者没提歌名则为null
4. author：若弹幕提到"xxx的歌"、"来首xxx的"等，填入作者名；若没提作者则为null
5. 不要提取难度星级或难度档位；即使弹幕包含"7星"、"ex"、"hard"、"红/蓝/紫谱"等，也只根据歌名和作者判断点歌意图

输出必须是JSON，不要其他文字：
{"is_song_request": true/false, "song_name": "歌名或null", "author": "作者名或null"}"#;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SongIntent {
    pub is_song_request: bool,
    pub song_name: Option<String>,
    pub author: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    response_format: ResponseFormat,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: &'static str,
    content: String,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    kind: &'static str,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: Option<String>,
}

pub async fn analyze(config: &Config, message: &str) -> Result<SongIntent, String> {
    if !config.llm_enabled {
        return Ok(SongIntent::default());
    }

    let base_url = config.llm_base_url.trim().trim_end_matches('/');
    let endpoint = format!("{base_url}/v1/chat/completions");
    let mut builder = reqwest::Client::builder();
    if !config.http_proxy.trim().is_empty() {
        builder = builder.proxy(
            reqwest::Proxy::all(config.http_proxy.trim())
                .map_err(|error| format!("配置 LLM 代理失败: {error}"))?,
        );
    }
    let client = builder
        .build()
        .map_err(|error| format!("创建 LLM HTTP 客户端失败: {error}"))?;
    let request = ChatRequest {
        model: config.llm_model.clone(),
        messages: vec![
            ChatMessage {
                role: "system",
                content: SYSTEM_PROMPT.to_string(),
            },
            ChatMessage {
                role: "user",
                content: message.to_string(),
            },
        ],
        response_format: ResponseFormat {
            kind: "json_object",
        },
        temperature: 0.1,
        max_tokens: config.llm_max_tokens,
    };

    let mut request_builder = client.post(endpoint).json(&request);
    if !config.llm_api_key.trim().is_empty() {
        request_builder = request_builder.bearer_auth(config.llm_api_key.trim());
    }

    let response = request_builder
        .send()
        .await
        .map_err(|error| format!("LLM 请求失败: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("LLM 请求失败: HTTP {}", response.status()));
    }
    let response: ChatResponse = response
        .json()
        .await
        .map_err(|error| format!("解析 LLM 响应失败: {error}"))?;
    let Some(content) = response
        .choices
        .first()
        .and_then(|choice| choice.message.content.as_deref())
    else {
        return Ok(SongIntent::default());
    };
    serde_json::from_str(content).map_err(|error| format!("解析 LLM JSON 失败: {error}"))
}
