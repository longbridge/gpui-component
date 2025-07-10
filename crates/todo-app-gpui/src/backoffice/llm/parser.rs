use super::types::{ChatMessage, ToolCall};
use futures::stream::Stream;
use std::pin::Pin;

/// 流式工具解析器
pub struct StreamingToolParser {
    buffer: String,
    state: ParserState,
    last_output_pos: usize,
}

#[derive(Debug, Clone)]
enum ParserState {
    Normal,
    InToolTag { start_pos: usize, tag_name: String },
}

impl StreamingToolParser {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            state: ParserState::Normal,
            last_output_pos: 0,
        }
    }

    /// 处理新的文本块，返回可以立即输出的消息
    pub fn process_chunk(&mut self, text: &str) -> Vec<ChatMessage> {
        let mut messages = Vec::new();

        // 将新文本添加到缓冲区
        self.buffer.push_str(text);

        // 处理缓冲区中的内容
        while let Some(message) = self.try_extract_message() {
            messages.push(message);
        }

        messages
    }

    /// 流结束时处理剩余内容
    pub fn finish(&mut self) -> Option<ChatMessage> {
        if self.last_output_pos < self.buffer.len() {
            let remaining = self.buffer[self.last_output_pos..].to_string();
            self.last_output_pos = self.buffer.len();

            if !remaining.trim().is_empty() {
                // 检查是否包含工具调用
                if let Some(tool_call) = self.extract_tool_call(&remaining) {
                    return Some(ChatMessage::tool_call(tool_call));
                } else {
                    return Some(ChatMessage::assistant_chunk(remaining));
                }
            }
        }
        None
    }

    /// 尝试从缓冲区提取消息
    fn try_extract_message(&mut self) -> Option<ChatMessage> {
        match &self.state {
            ParserState::Normal => {
                // 查找工具标签开始
                if let Some((tool_start, tag_name)) = self.find_tool_start() {
                    // 输出工具标签前的文本
                    if tool_start > self.last_output_pos {
                        let text = self.buffer[self.last_output_pos..tool_start].to_string();
                        self.last_output_pos = tool_start;
                        self.state = ParserState::InToolTag {
                            start_pos: tool_start,
                            tag_name,
                        };

                        if !text.trim().is_empty() {
                            return Some(ChatMessage::assistant_chunk(text));
                        }
                    } else {
                        self.state = ParserState::InToolTag {
                            start_pos: tool_start,
                            tag_name,
                        };
                    }
                }

                // 如果没有工具标签，输出部分安全文本
                self.try_output_safe_text()
            }

            ParserState::InToolTag {
                start_pos,
                tag_name,
            } => {
                // 查找工具标签结束
                if let Some(tool_end) = self.find_tool_end(*start_pos, tag_name) {
                    let tool_xml = &self.buffer[*start_pos..tool_end];
                    self.last_output_pos = tool_end;
                    self.state = ParserState::Normal;

                    // 尝试解析工具调用
                    if let Some(tool_call) = self.extract_tool_call(tool_xml) {
                        return Some(ChatMessage::tool_call(tool_call));
                    } else {
                        // 解析失败，当作普通文本
                        return Some(ChatMessage::assistant_chunk(tool_xml.to_string()));
                    }
                }
                None
            }
        }
    }

    /// 查找工具标签开始位置
    fn find_tool_start(&self) -> Option<(usize, String)> {
        let search_text = &self.buffer[self.last_output_pos..];
        let patterns = [
            ("<tool_use", "tool_use"),
            ("<tool", "tool"),
            ("<function_call", "function_call"),
        ];

        let mut earliest = None;
        for (pattern, tag_name) in &patterns {
            if let Some(pos) = search_text.find(pattern) {
                let abs_pos = self.last_output_pos + pos;
                earliest = Some(earliest.map_or(
                    (abs_pos, tag_name.to_string()),
                    |(min_pos, min_tag)| {
                        if abs_pos < min_pos {
                            (abs_pos, tag_name.to_string())
                        } else {
                            (min_pos, min_tag)
                        }
                    },
                ));
            }
        }

        earliest
    }

    /// 查找工具标签结束位置
    fn find_tool_end(&self, start_pos: usize, tag_name: &str) -> Option<usize> {
        let search_text = &self.buffer[start_pos..];
        let end_pattern = format!("</{}>", tag_name);

        if let Some(end_pos) = search_text.find(&end_pattern) {
            Some(start_pos + end_pos + end_pattern.len())
        } else {
            None
        }
    }

    /// 输出安全的部分文本
    fn try_output_safe_text(&mut self) -> Option<ChatMessage> {
        let available_text = &self.buffer[self.last_output_pos..];
        let chars: Vec<char> = available_text.chars().collect();

        // 需要足够的字符才输出
        if chars.len() > 20 {
            // 保留最后10个字符，防止截断工具标签
            let safe_count = chars.len() - 10;
            let safe_text: String = chars[..safe_count].iter().collect();

            if !safe_text.trim().is_empty() {
                self.last_output_pos += safe_text.len();
                return Some(ChatMessage::assistant_chunk(safe_text));
            }
        }

        None
    }

    /// 提取工具调用
    fn extract_tool_call(&self, xml: &str) -> Option<ToolCall> {
        // 清理XML
        let cleaned_xml = xml
            .lines()
            .filter(|line| {
                let line = line.trim();
                !line.is_empty() && !line.contains("DEBUG") && !line.starts_with("20")
            })
            .collect::<Vec<_>>()
            .join("\n");

        // 尝试XML反序列化
        if let Ok(tool_call) = serde_xml_rs::from_str::<ToolCall>(&cleaned_xml) {
            return Some(tool_call);
        }

        // 尝试正则表达式解析
        self.parse_with_regex(&cleaned_xml)
    }

    /// 使用正则表达式解析工具调用
    fn parse_with_regex(&self, xml: &str) -> Option<ToolCall> {
        use regex::Regex;

        // 提取工具名称
        let name_regex =
            Regex::new(r#"<(?:tool|tool_use|function_call)\s+name=['"]([^'"]+)['"]"#).ok()?;
        let name = name_regex.captures(xml)?.get(1)?.as_str().to_string();

        // 提取参数
        let args_regex =
            Regex::new(r"<(?:parameters|args|arguments)>(.*?)</(?:parameters|args|arguments)>")
                .ok()?;
        let arguments = if let Some(caps) = args_regex.captures(xml) {
            caps.get(1)?.as_str().to_string()
        } else {
            "{}".to_string()
        };

        Some(ToolCall { name, arguments })
    }
}

/// 创建流式工具解析器包装器
pub fn create_streaming_tool_parser<S>(
    input_stream: S,
) -> Pin<Box<dyn Stream<Item = anyhow::Result<ChatMessage>> + Send>>
where
    S: Stream<Item = Result<String, anyhow::Error>> + Send + Unpin + 'static, // 添加 Unpin 约束
{
    use futures::stream;
    use futures::StreamExt;

    let parser = StreamingToolParser::new();

    Box::pin(stream::unfold(
        (input_stream, parser),
        |(mut stream, mut parser)| async move {
            match stream.next().await {
                Some(Ok(text)) => {
                    let messages = parser.process_chunk(&text);
                    if !messages.is_empty() {
                        Some((Ok(messages.into_iter().next().unwrap()), (stream, parser)))
                    } else {
                        // 没有消息可输出，继续处理下一个chunk
                        Some((Ok(ChatMessage::assistant_chunk("")), (stream, parser)))
                    }
                }
                Some(Err(e)) => Some((Err(e), (stream, parser))),
                None => {
                    // 流结束，处理剩余内容
                    if let Some(final_message) = parser.finish() {
                        Some((Ok(final_message), (stream, parser)))
                    } else {
                        None
                    }
                }
            }
        },
    ))
}
