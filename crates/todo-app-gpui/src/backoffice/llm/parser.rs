use super::types::{ChatMessage, ToolCall};
use futures::stream::Stream;
use std::pin::Pin;

/// 流式工具解析器
pub struct StreamingToolParser {
    buffer: String,
    last_output_pos: usize,
    pending_messages: Vec<ChatMessage>, // 添加待发送消息队列
}

impl StreamingToolParser {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            last_output_pos: 0,
            pending_messages: Vec::new(),
        }
    }

    /// 处理新的文本块，返回可以立即输出的消息
    pub fn process_chunk(&mut self, text: &str) -> Vec<ChatMessage> {
        // 将新文本添加到缓冲区
        self.buffer.push_str(text);

        // 处理缓冲区中的所有内容
        self.process_buffer();

        // 返回所有待发送的消息
        std::mem::take(&mut self.pending_messages)
    }

    /// 流结束时处理剩余内容
    pub fn finish(&mut self) -> Vec<ChatMessage> {
        // 处理剩余缓冲区内容
        self.process_buffer();

        // 如果还有未处理的文本，作为最后一条消息
        if self.last_output_pos < self.buffer.len() {
            let remaining = self.buffer[self.last_output_pos..].to_string();
            if !remaining.trim().is_empty() {
                // 检查是否包含未完整的工具调用
                if let Some(tool_call) = self.extract_partial_tool_call(&remaining) {
                    self.pending_messages
                        .push(ChatMessage::tool_call(tool_call));
                } else {
                    self.pending_messages
                        .push(ChatMessage::assistant_chunk(remaining));
                }
            }
        }

        std::mem::take(&mut self.pending_messages)
    }

    /// 处理缓冲区内容，提取所有可用的消息
    fn process_buffer(&mut self) {
        loop {
            let initial_pos = self.last_output_pos;

            // 尝试提取下一个内容块
            if !self.try_extract_next_content() {
                break; // 没有更多内容可提取
            }

            // 如果位置没有变化，防止无限循环
            if self.last_output_pos == initial_pos {
                break;
            }
        }
    }

    /// 尝试提取下一个内容块（文本或工具调用）
    fn try_extract_next_content(&mut self) -> bool {
        let remaining_text = &self.buffer[self.last_output_pos..];

        // 查找下一个工具标签
        if let Some(tool_info) = self.find_next_complete_tool(remaining_text) {
            let (tool_start, tool_end, tool_xml) = tool_info;
            let abs_tool_start = self.last_output_pos + tool_start;
            let abs_tool_end = self.last_output_pos + tool_end;

            // 如果工具前有文本，先添加文本消息
            if tool_start > 0 {
                let text_before = remaining_text[..tool_start].to_string();
                if !text_before.trim().is_empty() {
                    self.pending_messages
                        .push(ChatMessage::assistant_chunk(text_before));
                }
            }

            // 解析并添加工具调用
            if let Some(tool_call) = self.extract_tool_call(&tool_xml) {
                self.pending_messages
                    .push(ChatMessage::tool_call(tool_call));
            } else {
                // 解析失败，当作文本处理
                self.pending_messages
                    .push(ChatMessage::assistant_chunk(tool_xml));
            }

            // 更新位置
            self.last_output_pos = abs_tool_end;
            return true;
        }

        // 没有完整工具调用，尝试输出安全文本
        self.try_output_safe_text()
    }

    /// 查找下一个完整的工具调用
    fn find_next_complete_tool(&self, text: &str) -> Option<(usize, usize, String)> {
        let tool_patterns = [
            ("<tool_use", "</tool_use>"),
            ("<tool", "</tool>"),
            ("<function_call", "</function_call>"),
        ];

        let mut earliest_tool = None;

        for (start_pattern, end_pattern) in &tool_patterns {
            if let Some(start_pos) = text.find(start_pattern) {
                // 从工具开始位置查找结束标签
                let search_from = start_pos;
                if let Some(end_pos) = text[search_from..].find(end_pattern) {
                    let abs_end_pos = search_from + end_pos + end_pattern.len();
                    let tool_xml = text[start_pos..abs_end_pos].to_string();

                    // 选择最早出现的工具
                    earliest_tool = Some(earliest_tool.map_or(
                        (start_pos, abs_end_pos, tool_xml.clone()),
                        |(min_start, min_end, min_xml)| {
                            if start_pos < min_start {
                                (start_pos, abs_end_pos, tool_xml)
                            } else {
                                (min_start, min_end, min_xml)
                            }
                        },
                    ));
                }
            }
        }

        earliest_tool
    }

    /// 输出安全的部分文本
    fn try_output_safe_text(&mut self) -> bool {
        let available_text = &self.buffer[self.last_output_pos..];

        // 检查是否有工具标签开始
        let has_tool_start = self.has_potential_tool_start(available_text);

        if has_tool_start {
            // 有潜在的工具标签，只输出到工具标签前的文本
            if let Some(tool_start_pos) = self.find_earliest_tool_start(available_text) {
                if tool_start_pos > 0 {
                    let safe_text = available_text[..tool_start_pos].to_string();
                    let len = safe_text.len();
                    if !safe_text.trim().is_empty() {
                        self.pending_messages
                            .push(ChatMessage::assistant_chunk(safe_text));
                        self.last_output_pos += len;
                        return true;
                    }
                }
            }
        } else {
            // 没有工具标签，可以安全输出大部分文本
            let chars: Vec<char> = available_text.chars().collect();
            if chars.len() > 20 {
                let safe_count = chars.len() - 10; // 保留最后10个字符
                let safe_text: String = chars[..safe_count].iter().collect();
                let len = safe_text.len();
                if !safe_text.trim().is_empty() {
                    self.pending_messages
                        .push(ChatMessage::assistant_chunk(safe_text));
                    self.last_output_pos += len;
                    return true;
                }
            }
        }

        false
    }

    /// 检查是否有潜在的工具标签开始
    fn has_potential_tool_start(&self, text: &str) -> bool {
        let patterns = ["<tool", "<function_call"];
        patterns.iter().any(|pattern| text.contains(pattern))
    }

    /// 查找最早的工具标签开始位置
    fn find_earliest_tool_start(&self, text: &str) -> Option<usize> {
        let patterns = ["<tool_use", "<tool", "<function_call"];

        patterns
            .iter()
            .filter_map(|pattern| text.find(pattern))
            .min()
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

    /// 提取部分工具调用（用于流结束时）
    fn extract_partial_tool_call(&self, xml: &str) -> Option<ToolCall> {
        // 对于不完整的XML，尝试更宽松的解析
        if xml.trim_start().starts_with("<tool") || xml.trim_start().starts_with("<function_call") {
            // 尝试提取已有的信息
            self.parse_with_regex(xml)
        } else {
            None
        }
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
    S: Stream<Item = Result<String, anyhow::Error>> + Send + Unpin + 'static,
{
    use futures::stream;
    use futures::StreamExt;

    let parser = StreamingToolParser::new();

    Box::pin(stream::unfold(
        (input_stream, parser, Vec::<ChatMessage>::new()),
        |(mut stream, mut parser, mut message_queue)| async move {
            // 如果队列中有消息，先发送队列中的消息
            if !message_queue.is_empty() {
                let message = message_queue.remove(0);
                return Some((Ok(message), (stream, parser, message_queue)));
            }

            match stream.next().await {
                Some(Ok(text)) => {
                    let mut messages = parser.process_chunk(&text);

                    if !messages.is_empty() {
                        // 取出第一个消息发送，其余放入队列
                        let first_message = messages.remove(0);
                        message_queue.extend(messages);
                        Some((Ok(first_message), (stream, parser, message_queue)))
                    } else {
                        // 没有消息，返回空的chunk以保持流活跃
                        Some((
                            Ok(ChatMessage::assistant_chunk("")),
                            (stream, parser, message_queue),
                        ))
                    }
                }
                Some(Err(e)) => Some((Err(e), (stream, parser, message_queue))),
                None => {
                    // 流结束，处理剩余内容
                    let final_messages = parser.finish();
                    if !final_messages.is_empty() {
                        let first_message = final_messages.into_iter().next().unwrap();
                        Some((Ok(first_message), (stream, parser, message_queue)))
                    } else {
                        None
                    }
                }
            }
        },
    ))
}
