use super::types::{ChatMessage, ToolCall};
use futures::stream::{self, Stream, StreamExt};
use std::pin::Pin;

/// 解析器状态
#[derive(PartialEq, Debug, Clone)]
enum ParserState {
    /// 普通文本流
    StreamingText,
    /// 正在匹配 "<tool_use>" 开始标签
    MatchingStartTag { matched_chars: String },
    /// 在工具标签内，缓冲工具内容
    InsideTool,
    /// 正在匹配 "</tool_use>" 结束标签
    MatchingEndTag { matched_chars: String },
}

/// 流式工具解析器
pub struct StreamingToolParser {
    buffer: String,
    tool_content: String,
    state: ParserState,
}

impl StreamingToolParser {
    const START_TAG: &'static str = "<tool_use>";
    const END_TAG: &'static str = "</tool_use>";

    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            tool_content: String::new(),
            state: ParserState::StreamingText,
        }
    }

    /// 处理新的文本块
    pub fn process_chunk(&mut self, chunk: &str) -> Vec<ChatMessage> {
        let mut messages = Vec::new();

        for ch in chunk.chars() {
            if let Some(message) = self.process_char(ch) {
                messages.push(message);
            }
        }

        tracing::debug!(
            "Processed chunk '{}', state: {:?}, buffer: '{}', tool_content: '{}'",
            chunk,
            self.state,
            self.buffer,
            self.tool_content
        );

        messages
    }

    /// 逐字符处理
    fn process_char(&mut self, ch: char) -> Option<ChatMessage> {
        match &mut self.state {
            ParserState::StreamingText => {
                if ch == '<' {
                    let message = self.flush_buffer_as_text();
                    self.state = ParserState::MatchingStartTag {
                        matched_chars: "<".to_string(),
                    };
                    message
                } else {
                    self.buffer.push(ch);
                    None
                }
            }

            ParserState::MatchingStartTag { matched_chars } => {
                matched_chars.push(ch);

                if Self::START_TAG.starts_with(matched_chars.as_str()) {
                    if matched_chars == Self::START_TAG {
                        // 完全匹配，进入工具内部
                        self.state = ParserState::InsideTool;
                        self.tool_content.clear();
                    }
                    None
                } else {
                    // 匹配失败，回到文本状态
                    let failed_chars = matched_chars.clone();
                    self.state = ParserState::StreamingText;
                    self.buffer.push_str(&failed_chars);
                    None
                }
            }

            ParserState::InsideTool => {
                if ch == '<' {
                    self.state = ParserState::MatchingEndTag {
                        matched_chars: "<".to_string(),
                    };
                } else {
                    self.tool_content.push(ch);
                }
                None
            }

            ParserState::MatchingEndTag { matched_chars } => {
                matched_chars.push(ch);

                if Self::END_TAG.starts_with(matched_chars.as_str()) {
                    if matched_chars == Self::END_TAG {
                        // 完全匹配，工具结束
                        let tool_content = self.tool_content.clone();
                        self.tool_content.clear();
                        self.state = ParserState::StreamingText;

                        // 解析工具调用
                        let full_tool_xml = format!("<tool_use>{}</tool_use>", tool_content);
                        if let Some(tool_call) = self.parse_tool_call(&full_tool_xml) {
                            tracing::debug!("解析到工具调用: {:?}", tool_call);
                            return Some(ChatMessage::tool_call(tool_call));
                        } else {
                            return Some(ChatMessage::assistant_chunk(full_tool_xml));
                        }
                    }
                    None
                } else {
                    // 匹配失败，这不是结束标签
                    let failed_chars = matched_chars.clone();
                    self.state = ParserState::InsideTool;
                    self.tool_content.push_str(&failed_chars);
                    None
                }
            }
        }
    }

    /// 将缓冲区内容作为文本消息输出
    fn flush_buffer_as_text(&mut self) -> Option<ChatMessage> {
        if !self.buffer.is_empty() {
            let text = self.buffer.clone();
            self.buffer.clear();
            Some(ChatMessage::assistant_chunk(text))
        } else {
            None
        }
    }

    /// 流结束时处理剩余内容
    pub fn finish(&mut self) -> Vec<ChatMessage> {
        let mut messages = Vec::new();

        match &self.state {
            ParserState::StreamingText => {
                if let Some(msg) = self.flush_buffer_as_text() {
                    messages.push(msg);
                }
            }
            ParserState::MatchingStartTag { matched_chars } => {
                self.buffer.push_str(matched_chars);
                if let Some(msg) = self.flush_buffer_as_text() {
                    messages.push(msg);
                }
            }
            ParserState::InsideTool => {
                let incomplete_tool = format!("<tool_use>{}", self.tool_content);
                messages.push(ChatMessage::assistant_chunk(incomplete_tool));
            }
            ParserState::MatchingEndTag { matched_chars } => {
                let incomplete_tool = format!("<tool_use>{}{}", self.tool_content, matched_chars);
                messages.push(ChatMessage::assistant_chunk(incomplete_tool));
            }
        }

        // 重置状态
        self.buffer.clear();
        self.tool_content.clear();
        self.state = ParserState::StreamingText;

        tracing::debug!("Messages after finish: {:?}", messages);
        messages
    }

    /// 解析工具调用
    fn parse_tool_call(&self, xml: &str) -> Option<ToolCall> {
        if let Ok(tool_call) = serde_xml_rs::from_str::<ToolCall>(xml) {
            return Some(tool_call);
        }

        tracing::warn!("XML parsing failed for: '{}'. Trying regex.", xml);
        self.parse_with_regex(xml)
    }

    /// 正则表达式解析
    fn parse_with_regex(&self, text: &str) -> Option<ToolCall> {
        use regex::Regex;
        let re = Regex::new(
            r"(?s)<tool_use>\s*<name>\s*([^<]+?)\s*</name>\s*<arguments>\s*([^<]*?)\s*</arguments>\s*</tool_use>",
        )
        .ok()?;

        let caps = re.captures(text)?;
        let name = caps.get(1)?.as_str().trim().to_string();
        let arguments = caps.get(2)?.as_str().trim().to_string();

        Some(ToolCall { name, arguments })
    }
}

/// 创建流式工具解析器
pub fn create_streaming_tool_parser<S>(
    input_stream: S,
) -> Pin<Box<dyn Stream<Item = anyhow::Result<ChatMessage>> + Send>>
where
    S: Stream<Item = Result<String, anyhow::Error>> + Send + Unpin + 'static,
{
    let stream = async_stream::stream! {
        let mut parser = StreamingToolParser::new();
        let mut stream = input_stream;

        while let Some(result) = stream.next().await {
            match result {
                Ok(chunk) => {
                    let messages = parser.process_chunk(&chunk);
                    for message in messages {
                        yield Ok(message);
                    }
                }
                Err(e) => {
                    yield Err(e);
                    break;
                }
            }
        }

        // 处理流结束时的剩余内容
        let final_messages = parser.finish();
        for message in final_messages {
            yield Ok(message);
        }
    };

    Box::pin(stream)
}
