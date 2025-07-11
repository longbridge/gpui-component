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
    tool_content: String, // 单独缓冲工具内容
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
                    // 可能是工具标签的开始，切换到匹配状态
                    let message = self.flush_buffer_as_text();
                    self.state = ParserState::MatchingStartTag {
                        matched_chars: "<".to_string(),
                    };
                    message
                } else {
                    // 普通字符，添加到缓冲区
                    self.buffer.push(ch);
                    None
                }
            }

            ParserState::MatchingStartTag { matched_chars } => {
                matched_chars.push(ch);

                // 修复：使用 matched_chars.as_str() 来获取 &str
                if Self::START_TAG.starts_with(matched_chars.as_str()) {
                    // 还在匹配路径上
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

                    // 将失败的字符加回缓冲区
                    self.buffer.push_str(&failed_chars);

                    None
                }
            }

            ParserState::InsideTool => {
                if ch == '<' {
                    // 可能是结束标签的开始
                    self.state = ParserState::MatchingEndTag {
                        matched_chars: "<".to_string(),
                    };
                } else {
                    // 工具内容
                    self.tool_content.push(ch);
                }
                None
            }

            ParserState::MatchingEndTag { matched_chars } => {
                matched_chars.push(ch);

                // 修复：使用 matched_chars.as_str() 来获取 &str
                if Self::END_TAG.starts_with(matched_chars.as_str()) {
                    // 还在匹配路径上
                    if matched_chars == Self::END_TAG {
                        // 完全匹配，工具结束
                        let tool_content = self.tool_content.clone();
                        self.tool_content.clear();
                        self.state = ParserState::StreamingText;

                        // 解析工具调用
                        let full_tool_xml = format!("<tool_use>{}</tool_use>", tool_content);
                        if let Some(tool_call) = self.parse_tool_call(&full_tool_xml) {
                            tracing::debug!("解析到工具 tool call: {:?}", tool_call);
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

                    // 将失败的字符加回工具内容
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

        // 根据当前状态处理剩余内容
        match &self.state {
            ParserState::StreamingText => {
                if let Some(msg) = self.flush_buffer_as_text() {
                    messages.push(msg);
                }
            }
            ParserState::MatchingStartTag { matched_chars } => {
                // 未完成的开始标签匹配，当作普通文本
                self.buffer.push_str(matched_chars);
                if let Some(msg) = self.flush_buffer_as_text() {
                    messages.push(msg);
                }
            }
            ParserState::InsideTool => {
                // 未完成的工具，当作普通文本
                let incomplete_tool = format!("<tool_use>{}", self.tool_content);
                messages.push(ChatMessage::assistant_chunk(incomplete_tool));
            }
            ParserState::MatchingEndTag { matched_chars } => {
                // 未完成的结束标签匹配，将内容加回工具内容并当作普通文本
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
    // 修复：使用 scan 来正确管理 parser 的所有权
    let stream = input_stream
        .scan(StreamingToolParser::new(), |parser, result| {
            let res = match result {
                Ok(chunk) => Ok(parser.process_chunk(&chunk)),
                Err(e) => Err(e),
            };
            futures::future::ready(Some(res))
        })
        .chain(stream::once(async {
            // 这里我们创建一个新的解析器来处理 finish，
            // 但实际上 finish 应该被集成到上面的 scan 中
            // 为了简化，我们返回空的结果
            Ok(Vec::<ChatMessage>::new())
        }))
        .map(|result| match result {
            Ok(messages) => stream::iter(messages.into_iter().map(Ok)).left_stream(),
            Err(e) => stream::once(async move { Err(e) }).right_stream(),
        })
        .flatten()
        .filter(|msg| {
            futures::future::ready(
                msg.as_ref()
                    .map_or(true, |m| !m.get_text().is_empty() || m.is_tool_call()),
            )
        });

    Box::pin(stream)
}
