use anyhow::Ok;
use gpui::*;
use gpui_component::input::{DocumentColorProvider, Rope};

use crate::rust_analyzer::RustAnalyzerLspProvider;

impl DocumentColorProvider for RustAnalyzerLspProvider {
    fn document_colors(
        &self,
        text: &Rope,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Task<gpui::Result<Vec<lsp_types::ColorInformation>>> {
        let nodes = color_lsp::parse(&text.to_string());
        let colors = nodes
            .into_iter()
            .map(|node| {
                let start = lsp_types::Position::new(node.position.line, node.position.character);
                let end = lsp_types::Position::new(
                    node.position.line,
                    node.position.character + node.matched.chars().count() as u32,
                );

                lsp_types::ColorInformation {
                    range: lsp_types::Range { start, end },
                    color: lsp_types::Color {
                        red: node.color.r,
                        green: node.color.g,
                        blue: node.color.b,
                        alpha: node.color.a,
                    },
                }
            })
            .collect::<Vec<_>>();

        Task::ready(Ok(colors))
    }
}
