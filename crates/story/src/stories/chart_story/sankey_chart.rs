use gpui::{App, Bounds, Pixels, Window};
use gpui_component::{
    plot::{IntoPlot, Plot, Sankey, SankeyLinkData},
};

#[derive(IntoPlot)]
pub struct SankeyChart {
    sankey: Sankey,
}

impl SankeyChart {
    pub fn new() -> Self {
        let sankey = Sankey::new()
            .data(vec![
                // 2 sources -> converter
                SankeyLinkData {
                    source: "Source A".to_string(),
                    target: "Converter".to_string(),
                    value: 60.0,
                },
                SankeyLinkData {
                    source: "Source B".to_string(),
                    target: "Converter".to_string(),
                    value: 40.0,
                },
                // converter -> 3 outputs
                SankeyLinkData {
                    source: "Converter".to_string(),
                    target: "Output 1".to_string(),
                    value: 50.0,
                },
                SankeyLinkData {
                    source: "Converter".to_string(),
                    target: "Output 2".to_string(),
                    value: 30.0,
                },
                SankeyLinkData {
                    source: "Converter".to_string(),
                    target: "Output 3".to_string(),
                    value: 20.0,
                },
            ])
            .node_width(30.0)
            .node_padding(40.0)
            .iterations(8)
            .node_fill(|node| {
                use gpui::rgb;
                match node.name.as_str() {
                    "Source A" => rgb(0x6366f1),    // soft indigo
                    "Source B" => rgb(0xf97316),    // soft orange
                    "Converter" => rgb(0x3b82f6),   // soft blue
                    "Output 1" => rgb(0xa855f7),    // soft purple
                    "Output 2" => rgb(0xec4899),    // soft pink
                    "Output 3" => rgb(0x14b8a6),    // soft teal
                    _ => rgb(0x6b7280),
                }
            })
            .link_fill(|link| {
                use gpui::{rgb, hsla, Hsla};
                let base_color = match link.source {
                    0 => rgb(0x6366f1),   // Source A -> soft indigo
                    1 => rgb(0xf97316),   // Source B -> soft orange
                    2 => rgb(0x3b82f6),   // Converter -> soft blue
                    _ => rgb(0x6b7280),
                };
                let color: Hsla = base_color.into();
                hsla(color.h, color.s, color.l, 0.35)
            });

        Self { sankey }
    }
}

impl Plot for SankeyChart {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        self.sankey.paint(&bounds, window, cx);
    }
}
