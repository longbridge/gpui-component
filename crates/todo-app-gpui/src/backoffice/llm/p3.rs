/// 创建流式工具解析器
pub fn create_streaming_tool_parser<S>(
    input_stream: S,
) -> Pin<Box<dyn Stream<Item = anyhow::Result<ChatMessage>> + Send>>
where
    S: Stream<Item = Result<String, anyhow::Error>> + Send + Unpin + 'static,
{
    let stream = input_stream
        .scan(
            (StreamingToolParser::new(), false), // (解析器, 是否已结束)
            |state, result| {
                let (parser, is_finished) = state;

                if *is_finished {
                    return futures::future::ready(None);
                }

                match result {
                    Ok(chunk) => {
                        if chunk.is_empty() {
                            // 空 chunk 表示流结束
                            *is_finished = true;
                            let final_messages = parser.finish();
                            futures::future::ready(Some(Ok(final_messages)))
                        } else {
                            let messages = parser.process_chunk(&chunk);
                            futures::future::ready(Some(Ok(messages)))
                        }
                    }
                    Err(e) => {
                        *is_finished = true;
                        futures::future::ready(Some(Err(e)))
                    }
                }
            },
        )
        .chain(stream::once(async { Ok(Vec::<ChatMessage>::new()) })) // 添加空块触发结束
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
