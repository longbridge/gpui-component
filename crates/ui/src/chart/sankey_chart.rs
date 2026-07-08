use std::rc::Rc;

use gpui::{
    App, Bounds, Corners, Hsla, Pixels, SharedString, TextAlign, Window, fill, linear_color_stop,
    linear_gradient, point, px,
};
use gpui_component_macros::IntoPlot;

use crate::{
    ActiveTheme,
    plot::{
        Plot,
        label::{PlotLabel, TEXT_GAP, TEXT_SIZE, Text, measure_text_width},
        origin_point,
        shape::{Sankey, SankeyAlign, SankeyLink, sankey_link_path},
    },
};

const DEFAULT_NODE_WIDTH: f32 = 10.;
const DEFAULT_NODE_PADDING: f32 = 16.;
const DEFAULT_LINK_OPACITY: f32 = 0.3;
const DEFAULT_MIN_LINK_WIDTH: f32 = 1.;
const DEFAULT_LABEL_GAP: f32 = 6.;
/// Cap the label margins so very long labels never collapse the flow area.
const MAX_LABEL_MARGIN_RATIO: f32 = 0.6;

/// A styled line of a sankey node label.
#[derive(Clone)]
pub struct SankeyLabel {
    text: SharedString,
    color: Option<Hsla>,
    font_size: Option<f32>,
}

impl SankeyLabel {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            color: None,
            font_size: None,
        }
    }

    /// Set the text color. Defaults to the theme foreground.
    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Set the font size. Defaults to 10.
    pub fn font_size(mut self, font_size: f32) -> Self {
        self.font_size = Some(font_size);
        self
    }

    fn line_height(&self) -> f32 {
        self.font_size.unwrap_or(TEXT_SIZE) + TEXT_GAP
    }
}

fn block_height(lines: &[SankeyLabel]) -> f32 {
    lines.iter().map(|line| line.line_height()).sum()
}

/// A Sankey diagram, layout modeled after [d3-sankey](https://github.com/d3/d3-sankey).
///
/// Links reference nodes by their index in the node list; map string ids to
/// indices before constructing.
#[derive(IntoPlot)]
pub struct SankeyChart<T: 'static> {
    nodes: Vec<T>,
    links: Vec<SankeyLink>,
    node_width: f32,
    node_padding: f32,
    align: SankeyAlign,
    iterations: usize,
    node_corner_radius: Option<Pixels>,
    node_color: Option<Rc<dyn Fn(&T) -> Hsla>>,
    node_label: Option<Rc<dyn Fn(&T) -> SharedString>>,
    value_label: Option<Rc<dyn Fn(&T, f64) -> SharedString>>,
    labels: Option<Rc<dyn Fn(&T, f64) -> Vec<SankeyLabel>>>,
    link_opacity: f32,
    min_link_width: f32,
    label_gap: f32,
}

impl<T> SankeyChart<T> {
    pub fn new<I, L>(nodes: I, links: L) -> Self
    where
        I: IntoIterator<Item = T>,
        L: IntoIterator<Item = SankeyLink>,
    {
        Self {
            nodes: nodes.into_iter().collect(),
            links: links.into_iter().collect(),
            node_width: DEFAULT_NODE_WIDTH,
            node_padding: DEFAULT_NODE_PADDING,
            align: SankeyAlign::default(),
            iterations: 6,
            node_corner_radius: None,
            node_color: None,
            node_label: None,
            value_label: None,
            labels: None,
            link_opacity: DEFAULT_LINK_OPACITY,
            min_link_width: DEFAULT_MIN_LINK_WIDTH,
            label_gap: DEFAULT_LABEL_GAP,
        }
    }

    /// Set the node rectangle width. Defaults to 10.
    pub fn node_width(mut self, node_width: f32) -> Self {
        self.node_width = node_width;
        self
    }

    /// Set the vertical gap between nodes in a column. Defaults to 16.
    pub fn node_padding(mut self, node_padding: f32) -> Self {
        self.node_padding = node_padding;
        self
    }

    /// Set the node alignment. Defaults to [`SankeyAlign::Justify`].
    pub fn node_align(mut self, align: SankeyAlign) -> Self {
        self.align = align;
        self
    }

    /// Set the number of relaxation passes. Defaults to 6.
    pub fn iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations;
        self
    }

    /// Set the corner radius of the node rectangles. Defaults to 0.
    pub fn node_corner_radius(mut self, radius: impl Into<Pixels>) -> Self {
        self.node_corner_radius = Some(radius.into());
        self
    }

    /// Set the color of each node.
    ///
    /// Defaults to cycling the theme chart palette by node index.
    pub fn node_color<H>(mut self, color: impl Fn(&T) -> H + 'static) -> Self
    where
        H: Into<Hsla> + 'static,
    {
        self.node_color = Some(Rc::new(move |t| color(t).into()));
        self
    }

    /// Set the name label of each node, drawn in muted foreground.
    pub fn node_label(mut self, label: impl Fn(&T) -> SharedString + 'static) -> Self {
        self.node_label = Some(Rc::new(label));
        self
    }

    /// Set the value label of each node, drawn above the name label.
    ///
    /// The closure receives the datum and the node's computed throughput
    /// (max of incoming and outgoing flow).
    pub fn value_label(mut self, label: impl Fn(&T, f64) -> SharedString + 'static) -> Self {
        self.value_label = Some(Rc::new(label));
        self
    }

    /// Set fully custom node labels, one [`SankeyLabel`] per line, top to
    /// bottom. Takes precedence over `node_label`/`value_label` when set.
    ///
    /// The closure receives the datum and the node's computed throughput
    /// (max of incoming and outgoing flow).
    pub fn labels(mut self, labels: impl Fn(&T, f64) -> Vec<SankeyLabel> + 'static) -> Self {
        self.labels = Some(Rc::new(labels));
        self
    }

    /// Set the opacity of the link ribbons. Defaults to 0.3.
    pub fn link_opacity(mut self, opacity: f32) -> Self {
        self.link_opacity = opacity;
        self
    }

    /// Set the minimum ribbon thickness, so tiny flows stay visible. Defaults to 1.
    pub fn min_link_width(mut self, width: f32) -> Self {
        self.min_link_width = width;
        self
    }

    /// Set the gap between a node and its labels. Defaults to 6.
    pub fn label_gap(mut self, gap: f32) -> Self {
        self.label_gap = gap;
        self
    }

    fn sankey(&self) -> Sankey {
        Sankey::new()
            .node_width(self.node_width)
            .node_padding(self.node_padding)
            .node_align(self.align)
            .iterations(self.iterations)
    }
}

impl<T> Plot for SankeyChart<T> {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let width = bounds.size.width.as_f32();
        let height = bounds.size.height.as_f32();
        if self.nodes.is_empty() || self.links.is_empty() || width <= 0. || height <= 0. {
            return;
        }

        // First pass: only the topology (layer, value) is needed to measure
        // the label margins.
        let Ok(topology) = self.sankey().topology(self.nodes.len(), &self.links) else {
            return;
        };
        let layer_count = topology.layer_count();

        // Collect each node's label lines: the custom `labels` closure wins,
        // otherwise synthesize the value/name lines with the default styles.
        let node_labels: Vec<Vec<SankeyLabel>> = topology
            .nodes
            .iter()
            .map(|node| {
                let datum = &self.nodes[node.index];
                if let Some(labels) = &self.labels {
                    labels(datum, node.value)
                } else {
                    let mut lines = Vec::new();
                    if let Some(value_label) = &self.value_label {
                        lines.push(SankeyLabel::new(value_label(datum, node.value)));
                    }
                    if let Some(node_label) = &self.node_label {
                        lines.push(
                            SankeyLabel::new(node_label(datum)).color(cx.theme().muted_foreground),
                        );
                    }
                    lines
                }
            })
            .collect();
        let has_labels = node_labels.iter().any(|lines| !lines.is_empty());

        // Reserve margins so the labels beside the first/last columns and
        // above the middle columns are not clipped.
        let mut left = 0f32;
        let mut right = 0f32;
        if has_labels {
            for node in &topology.nodes {
                if node.layer != 0 && node.layer + 1 != layer_count {
                    continue;
                }
                let mut label_width = 0f32;
                for line in &node_labels[node.index] {
                    label_width = label_width.max(measure_text_width(
                        &line.text,
                        px(line.font_size.unwrap_or(TEXT_SIZE)),
                        window,
                    ));
                }
                if node.layer == 0 {
                    left = left.max(label_width + self.label_gap);
                } else {
                    right = right.max(label_width + self.label_gap);
                }
            }

            let max_margin = width * MAX_LABEL_MARGIN_RATIO;
            if left + right > max_margin {
                let k = max_margin / (left + right);
                left *= k;
                right *= k;
            }
        }
        // Above-node labels are only emitted for middle columns, so reserve
        // the top band for the tallest such label block. Cap the vertical
        // margins like the horizontal ones so a short chart doesn't collapse
        // the flow.
        let mut top = 0f32;
        if has_labels && layer_count > 2 {
            for node in &topology.nodes {
                if node.layer == 0 || node.layer + 1 == layer_count {
                    continue;
                }
                let block = block_height(&node_labels[node.index]);
                if block > 0. {
                    top = top.max(block + TEXT_GAP);
                }
            }
        }
        let mut bottom = if has_labels { TEXT_GAP } else { 0. };
        let max_vertical = height * MAX_LABEL_MARGIN_RATIO;
        if top + bottom > max_vertical {
            let k = max_vertical / (top + bottom);
            top *= k;
            bottom *= k;
        }

        // Second pass: complete the placement on the final extent, reusing
        // the first pass's topology.
        let graph = self
            .sankey()
            .extent(
                left,
                top,
                (width - right).max(left + 1.),
                (height - bottom).max(top + 1.),
            )
            .layout_from(topology);

        let palette = [
            cx.theme().chart_1,
            cx.theme().chart_2,
            cx.theme().chart_3,
            cx.theme().chart_4,
            cx.theme().chart_5,
        ];
        let colors: Vec<Hsla> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(index, datum)| match &self.node_color {
                Some(color) => color(datum),
                None => palette[index % palette.len()],
            })
            .collect();

        // Links first, under the nodes.
        for link in &graph.links {
            if link.value <= 0. {
                continue;
            }
            let source = &graph.nodes[link.source];
            let target = &graph.nodes[link.target];
            let Some(path) =
                sankey_link_path(source, target, link, self.min_link_width, bounds.origin)
            else {
                continue;
            };
            window.paint_path(
                path,
                linear_gradient(
                    90.,
                    linear_color_stop(colors[link.source].opacity(self.link_opacity), 0.),
                    linear_color_stop(colors[link.target].opacity(self.link_opacity), 1.),
                ),
            );
        }

        let corner_radii = Corners::all(self.node_corner_radius.unwrap_or_default());
        for node in &graph.nodes {
            let node_bounds = Bounds::from_corners(
                origin_point(px(node.x0), px(node.y0), bounds.origin),
                // Keep tiny nodes visible with a minimum 1px height.
                origin_point(px(node.x1), px(node.y1.max(node.y0 + 1.)), bounds.origin),
            );
            window.paint_quad(fill(node_bounds, colors[node.index]).corner_radii(corner_radii));
        }

        let mut texts = Vec::new();
        for node in &graph.nodes {
            let lines = &node_labels[node.index];
            if lines.is_empty() {
                continue;
            }

            let is_first = node.layer == 0;
            let is_last = node.layer + 1 == layer_count;
            let (x, align) = if is_first {
                (node.x0 - self.label_gap, TextAlign::Right)
            } else if is_last {
                (node.x1 + self.label_gap, TextAlign::Left)
            } else {
                ((node.x0 + node.x1) / 2., TextAlign::Center)
            };

            let block = block_height(lines);
            let mut y = if is_first || is_last {
                // Block vertically centered beside the node, clamped into
                // the plot area so labels of nodes near the top or bottom
                // edge are not clipped.
                ((node.y0 + node.y1) / 2. - block / 2.)
                    .min(height - block)
                    .max(0.)
            } else {
                // Block above the node.
                node.y0 - block - TEXT_GAP
            };

            for line in lines {
                texts.push(
                    Text::new(
                        line.text.clone(),
                        point(px(x), px(y)),
                        line.color.unwrap_or(cx.theme().foreground),
                    )
                    .font_size(px(line.font_size.unwrap_or(TEXT_SIZE)))
                    .align(align),
                );
                y += line.line_height();
            }
        }
        PlotLabel::new(texts).paint(&bounds, window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sankey_chart_builder() {
        let chart = SankeyChart::new(vec!["a", "b"], vec![SankeyLink::new(0, 1, 5.)]);
        assert_eq!(chart.nodes.len(), 2);
        assert_eq!(chart.links.len(), 1);
        assert_eq!(chart.node_width, DEFAULT_NODE_WIDTH);
        assert_eq!(chart.node_padding, DEFAULT_NODE_PADDING);
        assert_eq!(chart.align, SankeyAlign::Justify);
        assert_eq!(chart.iterations, 6);
        assert_eq!(chart.node_corner_radius, None);
        assert_eq!(chart.link_opacity, DEFAULT_LINK_OPACITY);
        assert_eq!(chart.min_link_width, DEFAULT_MIN_LINK_WIDTH);
        assert_eq!(chart.label_gap, DEFAULT_LABEL_GAP);
        assert!(chart.node_color.is_none());
        assert!(chart.node_label.is_none());
        assert!(chart.value_label.is_none());
        assert!(chart.labels.is_none());

        let chart = chart
            .node_width(8.)
            .node_padding(20.)
            .node_align(SankeyAlign::Left)
            .iterations(10)
            .node_corner_radius(px(2.))
            .node_color(|_| gpui::red())
            .node_label(|d| SharedString::from(d.to_string()))
            .value_label(|_, value| SharedString::from(format!("{}", value)))
            .labels(|d, value| {
                vec![
                    SankeyLabel::new(format!("{}", value)),
                    SankeyLabel::new(d.to_string()),
                ]
            })
            .link_opacity(0.5)
            .min_link_width(2.)
            .label_gap(10.);
        assert_eq!(chart.node_width, 8.);
        assert_eq!(chart.node_padding, 20.);
        assert_eq!(chart.align, SankeyAlign::Left);
        assert_eq!(chart.iterations, 10);
        assert_eq!(chart.node_corner_radius, Some(px(2.)));
        assert_eq!(chart.link_opacity, 0.5);
        assert_eq!(chart.min_link_width, 2.);
        assert_eq!(chart.label_gap, 10.);
        assert!(chart.node_color.is_some());
        assert!(chart.node_label.is_some());
        assert!(chart.value_label.is_some());
        assert!(chart.labels.is_some());
    }

    #[test]
    fn test_sankey_label_builder() {
        let label = SankeyLabel::new("a");
        assert_eq!(label.text, "a");
        assert_eq!(label.color, None);
        assert_eq!(label.font_size, None);
        assert_eq!(label.line_height(), TEXT_SIZE + TEXT_GAP);

        let label = SankeyLabel::new("b").color(gpui::red()).font_size(14.);
        assert_eq!(label.color, Some(gpui::red()));
        assert_eq!(label.font_size, Some(14.));
        assert_eq!(label.line_height(), 14. + TEXT_GAP);

        assert_eq!(
            block_height(&[SankeyLabel::new("a"), SankeyLabel::new("b").font_size(14.)]),
            TEXT_SIZE + TEXT_GAP + 14. + TEXT_GAP
        );
        assert_eq!(block_height(&[]), 0.);
    }
}
