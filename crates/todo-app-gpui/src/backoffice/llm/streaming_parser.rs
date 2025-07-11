use nom::{
    bytes::streaming::{tag, take_until},
    character::streaming::multispace0,
    sequence::delimited,
    IResult,
};

#[derive(Debug, Clone)]
pub struct ToolUse {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug)]
pub enum ParseResult {
    Text(String),     // 普通文本，应该立即输出
    ToolUse(ToolUse), // 解析到的工具调用
}

#[derive(Debug, PartialEq)]
enum ParserState {
    Normal,            // 正常状态，查找标签或输出文本
    MightBeTag(usize), // 可能是标签开始，记录匹配的字符数
}

pub struct StreamingParser {
    buffer: String,
    state: ParserState,
}

impl StreamingParser {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            state: ParserState::Normal,
        }
    }

    /// 输入新的 token 块，返回可以立即输出的结果
    pub fn feed(&mut self, chunk: &str) -> Vec<ParseResult> {
        self.buffer.push_str(chunk);
        self.process_buffer()
    }

    /// 处理缓冲区内容，提取可以输出的结果
    fn process_buffer(&mut self) -> Vec<ParseResult> {
        let mut results = Vec::new();

        loop {
            match self.state {
                ParserState::Normal => {
                    if let Some(result) = self.process_normal_state() {
                        results.push(result);
                    } else {
                        break;
                    }
                }
                ParserState::MightBeTag(matched_len) => {
                    if let Some(result) = self.process_might_be_tag_state(matched_len) {
                        results.push(result);
                    } else {
                        break;
                    }
                }
            }
        }

        results
    }

    fn process_normal_state(&mut self) -> Option<ParseResult> {
        // 尝试解析完整的工具调用
        if let Ok((remaining, tool_use)) = parse_tool_use(&self.buffer) {
            self.buffer = remaining.to_string();
            return Some(ParseResult::ToolUse(tool_use));
        }

        // 查找可能的标签开始位置
        if let Some(pos) = self.find_tag_start() {
            if pos > 0 {
                // 输出标签前的文本
                let text = self.buffer[..pos].to_string();
                self.buffer = self.buffer[pos..].to_string();
                return Some(ParseResult::Text(text));
            } else {
                // 检查是否是部分标签
                let matched_len = self.get_partial_tag_match_length();
                if matched_len > 0 {
                    self.state = ParserState::MightBeTag(matched_len);
                    return None; // 等待更多数据
                }
            }
        }

        // 检查缓冲区末尾是否可能是标签开始
        let matched_len = self.get_partial_tag_match_length();
        if matched_len > 0 {
            self.state = ParserState::MightBeTag(matched_len);

            // 如果有部分匹配之前的文本，先输出
            let text_end = self.buffer.len() - matched_len;
            if text_end > 0 {
                let text = self.buffer[..text_end].to_string();
                self.buffer = self.buffer[text_end..].to_string();
                return Some(ParseResult::Text(text));
            }
            return None;
        }

        // 没有找到标签，输出所有内容作为文本
        if !self.buffer.is_empty() {
            let text = self.buffer.clone();
            self.buffer.clear();
            Some(ParseResult::Text(text))
        } else {
            None
        }
    }

    fn process_might_be_tag_state(&mut self, previous_matched_len: usize) -> Option<ParseResult> {
        let current_matched_len = self.get_partial_tag_match_length();

        if current_matched_len > previous_matched_len {
            // 匹配长度增加，继续等待
            self.state = ParserState::MightBeTag(current_matched_len);
            None
        } else if current_matched_len == 0 {
            // 不再匹配，输出之前的内容作为文本
            self.state = ParserState::Normal;
            if !self.buffer.is_empty() {
                let text = self.buffer[..1].to_string(); // 输出第一个字符
                self.buffer = self.buffer[1..].to_string();
                Some(ParseResult::Text(text))
            } else {
                None
            }
        } else {
            // 尝试解析完整标签
            if let Ok((remaining, tool_use)) = parse_tool_use(&self.buffer) {
                self.buffer = remaining.to_string();
                self.state = ParserState::Normal;
                Some(ParseResult::ToolUse(tool_use))
            } else {
                // 仍然是部分匹配，继续等待
                self.state = ParserState::MightBeTag(current_matched_len);
                None
            }
        }
    }

    fn find_tag_start(&self) -> Option<usize> {
        self.buffer.find("<tool_use>")
    }

    fn get_partial_tag_match_length(&self) -> usize {
        let target = "<tool_use>";
        let buffer_len = self.buffer.len();

        // 从最长可能的匹配开始检查
        for len in (1..=target.len().min(buffer_len)).rev() {
            let start_pos = buffer_len - len;
            if self.buffer[start_pos..] == target[..len] {
                return len;
            }
        }
        0
    }

    /// 获取当前缓冲区内容（用于调试）
    pub fn get_buffer(&self) -> &str {
        &self.buffer
    }

    /// 完成解析，输出剩余的缓冲区内容
    pub fn finish(mut self) -> Vec<ParseResult> {
        let mut results = Vec::new();

        if !self.buffer.is_empty() {
            results.push(ParseResult::Text(self.buffer));
        }

        results
    }
}

fn parse_tool_use(input: &str) -> IResult<&str, ToolUse> {
    let (input, _) = tag("<tool_use>")(input)?;
    let (input, _) = multispace0(input)?;
    let (input, name) = parse_name_tag(input)?;
    let (input, _) = multispace0(input)?;
    let (input, arguments) = parse_arguments_tag(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("</tool_use>")(input)?;

    Ok((input, ToolUse { name, arguments }))
}

fn parse_name_tag(input: &str) -> IResult<&str, String> {
    let (input, content) = delimited(tag("<name>"), take_until("</name>"), tag("</name>"))(input)?;
    Ok((input, content.to_string()))
}

fn parse_arguments_tag(input: &str) -> IResult<&str, String> {
    let (input, content) = delimited(
        tag("<arguments>"),
        take_until("</arguments>"),
        tag("</arguments>"),
    )(input)?;
    Ok((input, content.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complete_tool_use() {
        let mut parser = StreamingParser::new();
        let input =
            "<tool_use><name>search</name><arguments>{\"query\": \"rust\"}</arguments></tool_use>";
        let results = parser.feed(input);

        assert_eq!(results.len(), 1);
        if let ParseResult::ToolUse(tool) = &results[0] {
            assert_eq!(tool.name, "search");
            assert_eq!(tool.arguments, "{\"query\": \"rust\"}");
        } else {
            panic!("Expected ToolUse, got {:?}", results[0]);
        }
    }

    #[test]
    fn test_streaming_with_partial_tag() {
        let mut parser = StreamingParser::new();

        // 模拟流式输入
        let chunks = vec![
            "Hello world! ",
            "<tool",
            "_use><name>test",
            "</name><arguments>",
            "{\"param\": \"value\"}",
            "</arguments></tool_use>",
            " More text after.",
        ];

        let mut all_results = Vec::new();
        for chunk in chunks {
            let results = parser.feed(chunk);
            all_results.extend(results);
        }

        // 完成解析
        all_results.extend(parser.finish());

        assert_eq!(all_results.len(), 3);

        if let ParseResult::Text(ref text) = all_results[0] {
            assert_eq!(text, "Hello world! ");
        } else {
            panic!("Expected text, got {:?}", all_results[0]);
        }

        if let ParseResult::ToolUse(ref tool) = all_results[1] {
            assert_eq!(tool.name, "test");
            assert_eq!(tool.arguments, "{\"param\": \"value\"}");
        } else {
            panic!("Expected ToolUse, got {:?}", all_results[1]);
        }

        if let ParseResult::Text(ref text) = all_results[2] {
            assert_eq!(text, " More text after.");
        } else {
            panic!("Expected text, got {:?}", all_results[2]);
        }
    }

    #[test]
    fn test_multiple_tool_uses() {
        let mut parser = StreamingParser::new();
        let input = "<tool_use><name>tool1</name><arguments>args1</arguments></tool_use><tool_use><name>tool2</name><arguments>args2</arguments></tool_use>";
        let results = parser.feed(input);

        assert_eq!(results.len(), 2);

        for (i, result) in results.iter().enumerate() {
            if let ParseResult::ToolUse(tool) = result {
                assert_eq!(tool.name, format!("tool{}", i + 1));
                assert_eq!(tool.arguments, format!("args{}", i + 1));
            } else {
                panic!("Expected ToolUse at index {}, got {:?}", i, result);
            }
        }
    }

    #[test]
    fn test_false_positive_tag_start() {
        let mut parser = StreamingParser::new();

        // 测试看起来像标签开始但实际不是的情况
        let results1 = parser.feed("This is <tool but not a real tag");
        let results2 = parser.feed(" and more text");

        let mut all_results = results1;
        all_results.extend(results2);
        all_results.extend(parser.finish());

        assert_eq!(all_results.len(), 1);
        if let ParseResult::Text(ref text) = all_results[0] {
            assert_eq!(text, "This is <tool but not a real tag and more text");
        }
    }

    #[test]
    fn test_edge_case_partial_matches() {
        let mut parser = StreamingParser::new();

        // 测试各种部分匹配的情况
        let test_cases = vec![("Hello <", ""), ("t", ""), ("o", ""), ("ol_use>", "")];

        let mut all_results = Vec::new();
        for (chunk, _) in test_cases {
            let results = parser.feed(chunk);
            all_results.extend(results);
        }

        // 应该最终输出 "Hello <tool_use>"
        all_results.extend(parser.finish());

        assert_eq!(all_results.len(), 1);
        if let ParseResult::Text(ref text) = all_results[0] {
            assert_eq!(text, "Hello <tool_use>");
        }
    }
}

// 使用示例
pub fn example_usage() {
    let mut parser = StreamingParser::new();

    // 模拟从 LLM 接收到的流式数据
    let stream_chunks = vec![
        "我需要搜索一些信息。",
        "<tool_use>",
        "<name>search</name>",
        "<arguments>{\"query\": \"Rust编程\"}</arguments>",
        "</tool_use>",
        "搜索完成后，我会为你提供结果。",
    ];

    for chunk in stream_chunks {
        let results = parser.feed(chunk);
        for result in results {
            match result {
                ParseResult::Text(text) => {
                    println!("输出文本: {}", text);
                }
                ParseResult::ToolUse(tool) => {
                    println!("调用工具: {} with args: {}", tool.name, tool.arguments);
                }
            }
        }
    }

    // 处理剩余内容
    for result in parser.finish() {
        match result {
            ParseResult::Text(text) => {
                println!("输出文本: {}", text);
            }
            ParseResult::ToolUse(tool) => {
                println!("调用工具: {} with args: {}", tool.name, tool.arguments);
            }
        }
    }
}
