use crate::backoffice::llm::types::MessageContent;

use super::types::{ChatMessage, ToolCall};
use futures::stream::{Stream, StreamExt};
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
                    // 匹配失败，立即输出失败的内容
                    let failed_chars = matched_chars.clone();
                    self.state = ParserState::StreamingText;

                    // 立即输出失败的内容，而不是添加到缓冲区
                    Some(ChatMessage::assistant().with_text_chunk(failed_chars))
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
                            return Some(ChatMessage::system().with_content(MessageContent::ToolCall(tool_call)));
                        } else {
                            return Some(ChatMessage::assistant().with_text_chunk(full_tool_xml));
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
            Some(ChatMessage::assistant().with_text_chunk(text))
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
                messages.push(ChatMessage::assistant().with_text_chunk(incomplete_tool));
            }
            ParserState::MatchingEndTag { matched_chars } => {
                let incomplete_tool = format!("<tool_use>{}{}", self.tool_content, matched_chars);
                messages.push(ChatMessage::assistant().with_text_chunk(incomplete_tool));
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

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;

    // 模拟 ChatMessage 和 ToolCall 的测试实现
    #[derive(Debug, Clone, PartialEq)]
    pub struct TestChatMessage {
        pub content: String,
        pub message_type: MessageType,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum MessageType {
        AssistantChunk,
        ToolCall,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct TestToolCall {
        pub name: String,
        pub arguments: String,
    }

    impl TestChatMessage {
        pub fn assistant_chunk(content: String) -> Self {
            Self {
                content,
                message_type: MessageType::AssistantChunk,
            }
        }

        pub fn tool_call(tool_call: TestToolCall) -> Self {
            Self {
                content: format!("{}:{}", tool_call.name, tool_call.arguments),
                message_type: MessageType::ToolCall,
            }
        }
    }

    // 测试用的简化解析器
    struct TestParser {
        buffer: String,
        tool_content: String,
        state: ParserState,
    }

    impl TestParser {
        const START_TAG: &'static str = "<tool_use>";
        const END_TAG: &'static str = "</tool_use>";

        pub fn new() -> Self {
            Self {
                buffer: String::new(),
                tool_content: String::new(),
                state: ParserState::StreamingText,
            }
        }

        pub fn process_chunk(&mut self, chunk: &str) -> Vec<TestChatMessage> {
            let mut messages = Vec::new();
            for ch in chunk.chars() {
                if let Some(message) = self.process_char(ch) {
                    messages.push(message);
                }
            }
            messages
        }

        fn process_char(&mut self, ch: char) -> Option<TestChatMessage> {
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
                            self.state = ParserState::InsideTool;
                            self.tool_content.clear();
                        }
                        None
                    } else {
                        // 匹配失败，立即输出失败的内容
                        let failed_chars = matched_chars.clone();
                        self.state = ParserState::StreamingText;

                        // 立即输出失败的内容，而不是添加到缓冲区
                        Some(TestChatMessage::assistant_chunk(failed_chars))
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
                            let tool_content = self.tool_content.clone();
                            self.tool_content.clear();
                            self.state = ParserState::StreamingText;

                            if let Some(tool_call) = self.parse_tool_call(&tool_content) {
                                return Some(TestChatMessage::tool_call(tool_call));
                            } else {
                                return Some(TestChatMessage::assistant_chunk(format!(
                                    "<tool_use>{}</tool_use>",
                                    tool_content
                                )));
                            }
                        }
                        None
                    } else {
                        let failed_chars = matched_chars.clone();
                        self.state = ParserState::InsideTool;
                        self.tool_content.push_str(&failed_chars);
                        None
                    }
                }
            }
        }

        fn flush_buffer_as_text(&mut self) -> Option<TestChatMessage> {
            if !self.buffer.is_empty() {
                let text = self.buffer.clone();
                self.buffer.clear();
                Some(TestChatMessage::assistant_chunk(text))
            } else {
                None
            }
        }

        pub fn finish(&mut self) -> Vec<TestChatMessage> {
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
                    messages.push(TestChatMessage::assistant_chunk(incomplete_tool));
                }
                ParserState::MatchingEndTag { matched_chars } => {
                    let incomplete_tool =
                        format!("<tool_use>{}{}", self.tool_content, matched_chars);
                    messages.push(TestChatMessage::assistant_chunk(incomplete_tool));
                }
            }

            self.buffer.clear();
            self.tool_content.clear();
            self.state = ParserState::StreamingText;

            messages
        }

        fn parse_tool_call(&self, content: &str) -> Option<TestToolCall> {
            use regex::Regex;
            let re = Regex::new(
                r"(?s)<name>\s*([^<]+?)\s*</name>\s*<arguments>\s*([^<]*?)\s*</arguments>",
            )
            .ok()?;

            let caps = re.captures(content)?;
            let name = caps.get(1)?.as_str().trim().to_string();
            let arguments = caps.get(2)?.as_str().trim().to_string();

            Some(TestToolCall { name, arguments })
        }
    }

    #[test]
    fn test_simple_text_streaming() {
        let mut parser = TestParser::new();

        // 测试简单文本流
        let messages = parser.process_chunk("Hello World");
        assert_eq!(messages.len(), 0); // 文本还在缓冲区中

        let final_messages = parser.finish();
        assert_eq!(final_messages.len(), 1);
        assert_eq!(final_messages[0].content, "Hello World");
        assert_eq!(final_messages[0].message_type, MessageType::AssistantChunk);
    }

    #[test]
    fn test_complete_tool_call_single_chunk() {
        let mut parser = TestParser::new();

        let tool_xml = "<tool_use><name>search</name><arguments>query</arguments></tool_use>";
        let messages = parser.process_chunk(tool_xml);

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].message_type, MessageType::ToolCall);
        assert_eq!(messages[0].content, "search:query");
    }

    #[test]
    fn test_tool_call_with_surrounding_text() {
        let mut parser = TestParser::new();

        let input = "Here is a tool call: <tool_use><name>search</name><arguments>rust</arguments></tool_use> and more text";
        let messages = parser.process_chunk(input);

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content, "Here is a tool call: ");
        assert_eq!(messages[0].message_type, MessageType::AssistantChunk);
        assert_eq!(messages[1].content, "search:rust");
        assert_eq!(messages[1].message_type, MessageType::ToolCall);

        let final_messages = parser.finish();
        assert_eq!(final_messages.len(), 1);
        assert_eq!(final_messages[0].content, " and more text");
    }

    #[test]
    fn test_streaming_tool_call_char_by_char() {
        let mut parser = TestParser::new();
        let mut all_messages = Vec::new();

        let tool_xml = "<tool_use><name>calculate</name><arguments>2+2</arguments></tool_use>";

        // 逐字符处理
        for ch in tool_xml.chars() {
            let messages = parser.process_chunk(&ch.to_string());
            all_messages.extend(messages);
        }

        assert_eq!(all_messages.len(), 1);
        assert_eq!(all_messages[0].message_type, MessageType::ToolCall);
        assert_eq!(all_messages[0].content, "calculate:2+2");
    }

    #[test]
    fn test_multiple_tool_calls() {
        let mut parser = TestParser::new();

        let input = "<tool_use><name>search</name><arguments>rust</arguments></tool_use><tool_use><name>calculate</name><arguments>1+1</arguments></tool_use>";
        let messages = parser.process_chunk(input);

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content, "search:rust");
        assert_eq!(messages[1].content, "calculate:1+1");
    }

    #[test]
    fn test_incomplete_tool_call() {
        let mut parser = TestParser::new();

        let incomplete = "<tool_use><name>search</name><arguments>incomplete";
        let messages = parser.process_chunk(incomplete);
        assert_eq!(messages.len(), 0);

        let final_messages = parser.finish();
        assert_eq!(final_messages.len(), 1);
        assert_eq!(
            final_messages[0].content,
            "<tool_use><name>search</name><arguments>incomplete"
        );
        assert_eq!(final_messages[0].message_type, MessageType::AssistantChunk);
    }

    #[test]
    fn test_false_start_tag() {
        let mut parser = TestParser::new();

        let input = "<tool_wrong>not a tool</tool_wrong>";
        let messages = parser.process_chunk(input);

        // 应该有2个立即输出的消息
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].content, "<tool_w"); // 第一次匹配失败
        assert_eq!(messages[1].content, "rong>not a tool"); // 遇到 < 时刷新 buffer
        assert_eq!(messages[2].content, "</"); // 第二次匹配失败

        let final_messages = parser.finish();
        assert_eq!(final_messages.len(), 1);
        assert_eq!(final_messages[0].content, "tool_wrong>"); // 剩余 buffer 内容
    }

    #[test]
    fn test_false_start_tag_with_space() {
        let mut parser = TestParser::new();

        let input = "<tool_wrong> not a tool</tool_wrong>";
        let messages = parser.process_chunk(input);

        // 应该有3个立即输出的消息
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].content, "<tool_w"); // 第一次匹配失败
        assert_eq!(messages[1].content, "rong> not a tool"); // 遇到 < 时刷新 buffer
        assert_eq!(messages[2].content, "</"); // 第二次匹配失败

        let final_messages = parser.finish();
        assert_eq!(final_messages.len(), 1);
        assert_eq!(final_messages[0].content, "tool_wrong>"); // 剩余 buffer 内容
    }

    #[test]
    fn test_immediate_text_output_after_false_match() {
        let mut parser = TestParser::new();

        let input = "<xyz>content";
        let messages = parser.process_chunk(input);

        // 遇到 'x' 时匹配失败，立即输出 "<x"
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "<x");

        let final_messages = parser.finish();
        assert_eq!(final_messages.len(), 1);
        assert_eq!(final_messages[0].content, "yz>content");
    }

    #[test]
    fn test_mixed_content_with_false_start() {
        let mut parser = TestParser::new();

        let input = "Hello <not_tool>test</not_tool> World";
        let messages = parser.process_chunk(input);

        // 应该有3个立即输出的消息
        assert_eq!(messages.len(), 4);
        assert_eq!(messages[0].content, "Hello "); // 遇到 < 时刷新 buffer
        assert_eq!(messages[1].content, "<n"); // 第一次匹配失败
        assert_eq!(messages[2].content, "ot_tool>test"); // 遇到 < 时刷新 buffer
        assert_eq!(messages[3].content, "</"); // 第二次匹配失败

        let final_messages = parser.finish();
        assert_eq!(final_messages.len(), 1);
        assert_eq!(final_messages[0].content, "not_tool> World");
    }

    #[test]
    fn test_streaming_across_chunks() {
        let mut parser = TestParser::new();
        let mut all_messages = Vec::new();

        // 分块输入
        let chunks = vec![
            "Hello ",
            "<tool_use><name>se",
            "arch</name><argumen",
            "ts>query</arguments></tool_use>",
            " world",
        ];

        for chunk in chunks {
            let messages = parser.process_chunk(chunk);
            all_messages.extend(messages);
        }

        let final_messages = parser.finish();
        all_messages.extend(final_messages);

        assert_eq!(all_messages.len(), 3);
        assert_eq!(all_messages[0].content, "Hello ");
        assert_eq!(all_messages[1].content, "search:query");
        assert_eq!(all_messages[2].content, " world");
    }

    #[test]
    fn test_empty_tool_arguments() {
        let mut parser = TestParser::new();

        let input = "<tool_use><name>ping</name><arguments></arguments></tool_use>";
        let messages = parser.process_chunk(input);

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "ping:");
    }

    #[test]
    fn test_whitespace_in_tool_tags() {
        let mut parser = TestParser::new();

        let input = "<tool_use><name>  search  </name><arguments>  query  </arguments></tool_use>";
        let messages = parser.process_chunk(input);

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "search:query");
    }

    #[tokio::test]
    async fn test_streaming_parser_function() {
        let chunks = vec![
            Ok("Hello ".to_string()),
            Ok("<tool_use><name>search</name><arguments>rust</arguments></tool_use>".to_string()),
            Ok(" world".to_string()),
        ];

        let input_stream = stream::iter(chunks);
        let mut parser_stream = create_streaming_tool_parser(input_stream);

        let mut results = Vec::new();
        while let Some(result) = parser_stream.next().await {
            results.push(result);
        }

        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.is_ok()));
    }

    #[tokio::test]
    async fn test_streaming_parser_with_error() {
        let chunks = vec![
            Ok("Hello ".to_string()),
            Err(anyhow::anyhow!("Network error")),
            Ok("world".to_string()),
        ];

        let input_stream = stream::iter(chunks);
        let mut parser_stream = create_streaming_tool_parser(input_stream);

        let mut results = Vec::new();
        while let Some(result) = parser_stream.next().await {
            let is_error = result.is_err();
            results.push(result);
            if is_error {
                break;
            }
        }

        assert_eq!(results.len(), 2); // Hello chunk + error
        assert!(results[0].is_ok());
        assert!(results[1].is_err());
    }

    #[test]
    fn test_state_reset_after_finish() {
        let mut parser = TestParser::new();

        // 处理不完整的工具调用
        parser.process_chunk("<tool_use><name>incomplete");
        let final_messages = parser.finish();
        assert_eq!(final_messages.len(), 1);

        // 验证状态已重置
        assert_eq!(parser.state, ParserState::StreamingText);
        assert!(parser.buffer.is_empty());
        assert!(parser.tool_content.is_empty());

        // 处理新的内容应该正常工作
        let messages = parser.process_chunk("New content");
        assert_eq!(messages.len(), 0);

        let final_messages = parser.finish();
        assert_eq!(final_messages.len(), 1);
        assert_eq!(final_messages[0].content, "New content");
    }

    #[test]
    fn test_malformed_xml_fallback() {
        let mut parser = TestParser::new();

        // 测试格式错误的XML
        let input = "<tool_use><name>search<arguments>no closing name tag</arguments></tool_use>";
        let messages = parser.process_chunk(input);

        // 应该作为普通文本处理
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].message_type, MessageType::AssistantChunk);
    }
}
