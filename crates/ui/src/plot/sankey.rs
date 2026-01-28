use std::collections::{HashMap, VecDeque};

use gpui::{App, Bounds, Hsla, Path, PathBuilder, Pixels, Point, Window, fill, px};

use crate::{PixelsExt, plot::origin_point};

/// A node in the Sankey diagram.
///
/// Nodes represent entities in the flow network. Each node has a computed
/// position and size based on the total flow through it.
///
/// # Fields
///
/// * `name` - Unique identifier for the node
/// * `index` - Numerical index in the node array
/// * `layer` - Horizontal position (0-based, left to right)
/// * `depth` - Distance from source nodes (computed via BFS)
/// * `value` - Total flow through the node
/// * `x0`, `x1` - Left and right edges (in pixels)
/// * `y0`, `y1` - Top and bottom edges (in pixels)
/// * `source_links` - Indices of outgoing links
/// * `target_links` - Indices of incoming links
#[derive(Clone, Debug)]
pub struct SankeyNode {
    /// The unique name/identifier of the node.
    pub name: String,

    /// The index of the node in the nodes array.
    pub index: usize,

    /// The horizontal layer (0-based, computed from topology).
    ///
    /// Nodes at layer 0 are typically sources, and higher layers
    /// represent downstream positions in the flow.
    pub layer: usize,

    /// The depth (distance from source nodes).
    ///
    /// Computed using breadth-first search. Source nodes have depth 0.
    pub depth: usize,

    /// The total value flowing through this node.
    ///
    /// This is the sum of all incoming or outgoing link values,
    /// whichever is greater.
    pub value: f32,

    /// The x-coordinate of the left edge.
    pub x0: f32,

    /// The x-coordinate of the right edge.
    pub x1: f32,

    /// The y-coordinate of the top edge.
    pub y0: f32,

    /// The y-coordinate of the bottom edge.
    pub y1: f32,

    /// Indices of outgoing links from this node.
    pub source_links: Vec<usize>,

    /// Indices of incoming links to this node.
    pub target_links: Vec<usize>,
}

/// A link (flow edge) in the Sankey diagram.
///
/// Links represent flows between nodes. The width of each link is
/// proportional to its value.
///
/// # Fields
///
/// * `source` - Index of the source node
/// * `target` - Index of the target node
/// * `value` - Flow value (determines link width)
/// * `y0`, `y1` - Vertical positions at source and target
/// * `width` - Computed visual width of the link at source
/// * `target_width` - Computed visual width of the link at target
#[derive(Clone, Debug)]
pub struct SankeyLink {
    /// The index of the source node.
    pub source: usize,

    /// The index of the target node.
    pub target: usize,

    /// The flow value (determines the link width).
    pub value: f32,

    /// The y-coordinate at the source node.
    ///
    /// This is the vertical position where the link starts at the source.
    pub y0: f32,

    /// The y-coordinate at the target node.
    ///
    /// This is the vertical position where the link ends at the target.
    pub y1: f32,

    /// The computed width of the link at the source node.
    ///
    /// This is calculated based on the link's value relative to
    /// the total flow through the source node.
    pub width: f32,

    /// The computed width of the link at the target node.
    ///
    /// This is calculated based on the link's value relative to
    /// the total flow through the target node.
    pub target_width: f32,
}

/// Input data for creating a Sankey link.
///
/// This is the input format for specifying flows between nodes.
/// Node names will be automatically extracted and nodes will be created.
#[derive(Clone, Debug)]
pub struct SankeyLinkData {
    /// The name of the source node.
    pub source: String,

    /// The name of the target node.
    pub target: String,

    /// The flow value (must be positive).
    ///
    /// This determines the width of the link in the visualization.
    pub value: f32,
}

/// A Sankey diagram builder and renderer.
///
/// Creates flow diagrams where link widths are proportional to flow quantities.
/// The layout algorithm automatically positions nodes and routes links to
/// minimize crossings.
#[allow(clippy::type_complexity)]
pub struct Sankey {
    /// The input link data.
    links: Vec<SankeyLinkData>,

    /// Computed nodes (populated during layout).
    nodes: Vec<SankeyNode>,

    /// Computed links (populated during layout).
    computed_links: Vec<SankeyLink>,

    /// Width of nodes in pixels.
    node_width: f32,

    /// Vertical padding between nodes in the same layer.
    node_padding: f32,

    /// Function to determine node fill color.
    node_fill: Box<dyn Fn(&SankeyNode) -> Hsla>,

    /// Function to determine link fill color.
    link_fill: Box<dyn Fn(&SankeyLink) -> Hsla>,

    /// Number of relaxation iterations for layout optimization.
    iterations: usize,
}

impl Default for Sankey {
    fn default() -> Self {
        Self {
            links: Vec::new(),
            nodes: Vec::new(),
            computed_links: Vec::new(),
            node_width: 20.,
            node_padding: 8.,
            node_fill: Box::new(|_| gpui::rgb(0x18a0fb).into()),
            link_fill: Box::new(|_| {
                let color: Hsla = gpui::rgb(0x18a0fb).into();
                gpui::hsla(color.h, color.s, color.l, 0.3)
            }),
            iterations: 6,
        }
    }
}

impl Sankey {
    /// Creates a new Sankey diagram with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the link data for the diagram.
    ///
    /// Links define the flows between nodes. Nodes will be automatically
    /// extracted from the source and target names.
    ///
    /// # Arguments
    ///
    /// * `links` - An iterator of [`SankeyLinkData`] items
    pub fn data<I>(mut self, links: I) -> Self
    where
        I: IntoIterator<Item = SankeyLinkData>,
    {
        self.links = links.into_iter().collect();
        self
    }

    /// Sets the width of nodes in pixels.
    ///
    /// This determines the horizontal thickness of each node rectangle.
    ///
    /// # Arguments
    ///
    /// * `width` - Node width in pixels (default: 20.0)
    pub fn node_width(mut self, width: f32) -> Self {
        self.node_width = width;
        self
    }

    /// Sets the vertical padding between nodes in the same layer.
    ///
    /// This controls the spacing between nodes that are positioned
    /// in the same horizontal layer.
    ///
    /// # Arguments
    ///
    /// * `padding` - Padding in pixels (default: 8.0)
    pub fn node_padding(mut self, padding: f32) -> Self {
        self.node_padding = padding;
        self
    }

    /// Sets the function to determine node fill colors.
    ///
    /// The function receives a [`SankeyNode`] reference and should return
    /// a color. This allows for dynamic coloring based on node properties
    /// like layer, depth, or value.
    ///
    /// # Arguments
    ///
    /// * `fill` - A function that takes a `&SankeyNode` and returns a color
    pub fn node_fill<F, C>(mut self, fill: F) -> Self
    where
        F: Fn(&SankeyNode) -> C + 'static,
        C: Into<Hsla>,
    {
        self.node_fill = Box::new(move |node| fill(node).into());
        self
    }

    /// Sets the function to determine link fill colors.
    ///
    /// The function receives a [`SankeyLink`] reference and should return
    /// a color. Typically links use semi-transparent colors to show overlaps.
    ///
    /// # Arguments
    ///
    /// * `fill` - A function that takes a `&SankeyLink` and returns a color
    pub fn link_fill<F, C>(mut self, fill: F) -> Self
    where
        F: Fn(&SankeyLink) -> C + 'static,
        C: Into<Hsla>,
    {
        self.link_fill = Box::new(move |link| fill(link).into());
        self
    }

    /// Sets the number of relaxation iterations for layout optimization.
    ///
    /// More iterations generally produce better layouts with fewer link
    /// crossings, but take more computation time. The default of 6 provides
    /// good results for most diagrams.
    ///
    /// # Arguments
    ///
    /// * `iterations` - Number of iterations (default: 6)
    pub fn iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations;
        self
    }

    /// Computes the layout for the given bounds.
    ///
    /// This is called automatically by [`paint`](Self::paint) and should not
    /// normally be called directly.
    ///
    /// The algorithm performs the following steps:
    /// 1. Extract nodes from link data
    /// 2. Compute node depths using BFS
    /// 3. Calculate node values and positions
    /// 4. Run relaxation iterations to minimize crossings
    /// 5. Compute final link positions and paths
    fn compute_layout(&mut self, bounds: &Bounds<Pixels>) {
        if self.links.is_empty() {
            return;
        }

        let width = bounds.size.width.as_f32();
        let height = bounds.size.height.as_f32();

        // Build node map
        let mut node_map: HashMap<String, usize> = HashMap::new();
        let mut node_names: Vec<String> = Vec::new();

        for link in &self.links {
            if !node_map.contains_key(&link.source) {
                let idx = node_names.len();
                node_map.insert(link.source.clone(), idx);
                node_names.push(link.source.clone());
            }
            if !node_map.contains_key(&link.target) {
                let idx = node_names.len();
                node_map.insert(link.target.clone(), idx);
                node_names.push(link.target.clone());
            }
        }

        let _n = node_names.len();

        // Initialize nodes
        self.nodes = node_names
            .iter()
            .enumerate()
            .map(|(i, name)| SankeyNode {
                name: name.clone(),
                index: i,
                layer: 0,
                depth: 0,
                value: 0.0,
                x0: 0.0,
                x1: 0.0,
                y0: 0.0,
                y1: 0.0,
                source_links: Vec::new(),
                target_links: Vec::new(),
            })
            .collect();

        // Build links and assign to nodes
        self.computed_links = Vec::new();
        for (link_idx, link) in self.links.iter().enumerate() {
            let source = node_map[&link.source];
            let target = node_map[&link.target];

            self.nodes[source].source_links.push(link_idx);
            self.nodes[target].target_links.push(link_idx);

            self.computed_links.push(SankeyLink {
                source,
                target,
                value: link.value,
                y0: 0.0,
                y1: 0.0,
                width: 0.0,
                target_width: 0.0,
            });
        }

        // Compute node depths (horizontal positioning)
        self.compute_node_depths();

        // Compute node layers
        let max_depth = self.nodes.iter().map(|n| n.depth).max().unwrap_or(0);
        for node in &mut self.nodes {
            node.layer = node.depth;
        }

        // Compute node values
        // For each node, value = max(sum of outgoing links, sum of incoming links)
        // For flow-conserving networks, these should be equal
        let mut source_values = vec![0.0; self.nodes.len()];
        let mut target_values = vec![0.0; self.nodes.len()];

        for link in &self.computed_links {
            source_values[link.source] += link.value;
            target_values[link.target] += link.value;
        }

        for i in 0..self.nodes.len() {
            self.nodes[i].value = source_values[i].max(target_values[i]);
        }

        // Compute node x positions
        let kx = if max_depth > 0 {
            (width - self.node_width) / max_depth as f32
        } else {
            0.0
        };

        for node in &mut self.nodes {
            node.x0 = node.layer as f32 * kx;
            node.x1 = node.x0 + self.node_width;
        }

        // Compute node y positions
        let mut layers: Vec<Vec<usize>> = vec![Vec::new(); max_depth + 1];
        for (i, node) in self.nodes.iter().enumerate() {
            layers[node.layer].push(i);
        }

        for layer in &mut layers {
            let total_value: f32 = layer.iter().map(|&i| self.nodes[i].value).sum();
            let ky =
                (height - (layer.len() as f32 - 1.0) * self.node_padding) / total_value.max(1.0);

            let mut y = 0.0;
            for &i in layer.iter() {
                let node = &mut self.nodes[i];
                node.y0 = y;
                node.y1 = y + node.value * ky;
                y = node.y1 + self.node_padding;
            }
        }

        // Relaxation iterations
        for _ in 0..self.iterations {
            self.relax_left_to_right();
            self.relax_right_to_left();
        }

        // Compute link positions
        self.compute_link_positions();
    }

    /// Computes node depths using breadth-first search.
    ///
    /// Source nodes (with no incoming links) are assigned depth 0,
    /// and subsequent layers are numbered incrementally.
    fn compute_node_depths(&mut self) {
        let n = self.nodes.len();
        let mut depths = vec![0usize; n];
        let mut visited = vec![false; n];
        let mut queue = VecDeque::new();

        // Find source nodes (nodes with no incoming links)
        for i in 0..n {
            if self.nodes[i].target_links.is_empty() {
                queue.push_back(i);
                visited[i] = true;
            }
        }

        while let Some(node_idx) = queue.pop_front() {
            let depth = depths[node_idx];

            // Update depths of target nodes
            for link in &self.computed_links {
                if link.source == node_idx {
                    let target = link.target;
                    if !visited[target] || depths[target] < depth + 1 {
                        depths[target] = depth + 1;
                        if !visited[target] {
                            visited[target] = true;
                            queue.push_back(target);
                        }
                    }
                }
            }
        }

        for (i, depth) in depths.iter().enumerate() {
            self.nodes[i].depth = *depth;
        }
    }

    /// Relaxes node positions from left to right.
    ///
    /// This adjusts target node positions to be closer to the weighted
    /// average of their source nodes, reducing link crossings.
    fn relax_left_to_right(&mut self) {
        for link in &self.computed_links {
            let source_idx = link.source;
            let target_idx = link.target;

            let source_center = (self.nodes[source_idx].y0 + self.nodes[source_idx].y1) / 2.0;
            let target_center = (self.nodes[target_idx].y0 + self.nodes[target_idx].y1) / 2.0;

            let dy = (source_center - target_center) * 0.2;

            let height = self.nodes[target_idx].y1 - self.nodes[target_idx].y0;
            self.nodes[target_idx].y0 += dy;
            self.nodes[target_idx].y1 = self.nodes[target_idx].y0 + height;
        }
    }

    /// Relaxes node positions from right to left.
    ///
    /// This adjusts source node positions to be closer to the weighted
    /// average of their target nodes, further reducing link crossings.
    fn relax_right_to_left(&mut self) {
        for link in &self.computed_links {
            let source_idx = link.source;
            let target_idx = link.target;

            let source_center = (self.nodes[source_idx].y0 + self.nodes[source_idx].y1) / 2.0;
            let target_center = (self.nodes[target_idx].y0 + self.nodes[target_idx].y1) / 2.0;

            let dy = (target_center - source_center) * 0.2;

            let height = self.nodes[source_idx].y1 - self.nodes[source_idx].y0;
            self.nodes[source_idx].y0 += dy;
            self.nodes[source_idx].y1 = self.nodes[source_idx].y0 + height;
        }
    }

    /// Computes the vertical positions and widths of all links.
    ///
    /// Links are stacked vertically at each node, with widths proportional
    /// to their values relative to the total flow through the node.
    fn compute_link_positions(&mut self) {
        // Track cumulative heights for source and target nodes
        let mut source_y = vec![0.0; self.nodes.len()];
        let mut target_y = vec![0.0; self.nodes.len()];

        for i in 0..self.nodes.len() {
            source_y[i] = self.nodes[i].y0;
            target_y[i] = self.nodes[i].y0;
        }

        for link in &mut self.computed_links {
            let source = link.source;
            let target = link.target;

            let source_height = self.nodes[source].y1 - self.nodes[source].y0;
            let target_height = self.nodes[target].y1 - self.nodes[target].y0;

            let source_value = self.nodes[source].value;
            let target_value = self.nodes[target].value;

            link.width = if source_value > 0.0 {
                link.value * source_height / source_value
            } else {
                0.0
            };

            link.y0 = source_y[source];
            source_y[source] += link.width;

            link.target_width = if target_value > 0.0 {
                link.value * target_height / target_value
            } else {
                0.0
            };

            link.y1 = target_y[target];
            target_y[target] += link.target_width;
        }
    }

    /// Creates a cubic Bezier path for rendering a link.
    ///
    /// The path is a filled shape with curved top and bottom edges,
    /// creating a smooth flow appearance between nodes.
    fn link_path(&self, link: &SankeyLink, origin: Point<Pixels>) -> Option<Path<Pixels>> {
        let source = &self.nodes[link.source];
        let target = &self.nodes[link.target];

        let x0 = source.x1;
        let x1 = target.x0;
        let xi = (x0 + x1) / 2.0;

        let mut builder = PathBuilder::fill();

        // Top edge
        let p0 = origin_point(px(x0), px(link.y0), origin);
        let p1 = origin_point(px(x1), px(link.y1), origin);
        builder.move_to(p0);
        builder.cubic_bezier_to(
            p1,
            origin_point(px(xi), px(link.y0), origin),
            origin_point(px(xi), px(link.y1), origin),
        );

        // Bottom edge (reverse)
        let p2 = origin_point(px(x1), px(link.y1 + link.target_width), origin);
        let p3 = origin_point(px(x0), px(link.y0 + link.width), origin);
        builder.line_to(p2);
        builder.cubic_bezier_to(
            p3,
            origin_point(px(xi), px(link.y1 + link.target_width), origin),
            origin_point(px(xi), px(link.y0 + link.width), origin),
        );

        builder.close();
        builder.build().ok()
    }

    /// Renders the Sankey diagram to the window.
    ///
    /// This computes the layout if needed and paints all links and nodes.
    /// Links are drawn first so they appear behind nodes.
    ///
    /// # Arguments
    ///
    /// * `bounds` - The bounding box for the diagram
    /// * `window` - The window to paint to
    /// * `_cx` - The application context (currently unused)
    pub fn paint(&mut self, bounds: &Bounds<Pixels>, window: &mut Window, _cx: &mut App) {
        self.compute_layout(bounds);

        let origin = bounds.origin;

        // Paint links first (so they appear behind nodes)
        for link in &self.computed_links {
            if let Some(path) = self.link_path(link, origin) {
                let color = (self.link_fill)(link);
                window.paint_path(path, color);
            }
        }

        // Paint nodes
        for node in &self.nodes {
            let p1 = origin_point(px(node.x0), px(node.y0), origin);
            let p2 = origin_point(px(node.x1), px(node.y1), origin);
            let bounds = Bounds::from_corners(p1, p2);
            let color = (self.node_fill)(node);
            window.paint_quad(fill(bounds, color));
        }
    }

    /// Returns a reference to the computed nodes.
    ///
    /// This is useful for accessing node information after layout,
    /// such as for rendering labels or tooltips.
    ///
    /// # Returns
    ///
    /// A slice of [`SankeyNode`] structs containing layout information.
    pub fn nodes(&self) -> &[SankeyNode] {
        &self.nodes
    }

    /// Returns a reference to the computed links.
    ///
    /// This is useful for accessing link information after layout,
    /// such as for implementing custom rendering or interactions.
    ///
    /// # Returns
    ///
    /// A slice of [`SankeyLink`] structs containing layout information.
    pub fn links(&self) -> &[SankeyLink] {
        &self.computed_links
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sankey_builder() {
        let links = vec![
            SankeyLinkData {
                source: "A".to_string(),
                target: "B".to_string(),
                value: 10.0,
            },
            SankeyLinkData {
                source: "B".to_string(),
                target: "C".to_string(),
                value: 10.0,
            },
        ];

        let sankey = Sankey::new()
            .data(links)
            .node_width(20.)
            .node_padding(8.)
            .iterations(6);

        assert_eq!(sankey.links.len(), 2);
        assert_eq!(sankey.node_width, 20.);
        assert_eq!(sankey.node_padding, 8.);
        assert_eq!(sankey.iterations, 6);
    }

    #[test]
    fn test_sankey_layout() {
        let links = vec![
            SankeyLinkData {
                source: "A".to_string(),
                target: "B".to_string(),
                value: 10.0,
            },
            SankeyLinkData {
                source: "A".to_string(),
                target: "C".to_string(),
                value: 5.0,
            },
            SankeyLinkData {
                source: "B".to_string(),
                target: "D".to_string(),
                value: 10.0,
            },
        ];

        let mut sankey = Sankey::new().data(links);

        let bounds = Bounds::new(gpui::point(px(0.), px(0.)), gpui::size(px(500.), px(300.)));
        sankey.compute_layout(&bounds);

        // Check that nodes were created
        assert_eq!(sankey.nodes.len(), 4);

        // Check that node A is at layer 0
        let node_a = sankey.nodes.iter().find(|n| n.name == "A").unwrap();
        assert_eq!(node_a.layer, 0);

        // Check that node B and C are at layer 1
        let node_b = sankey.nodes.iter().find(|n| n.name == "B").unwrap();
        assert_eq!(node_b.layer, 1);

        let node_c = sankey.nodes.iter().find(|n| n.name == "C").unwrap();
        assert_eq!(node_c.layer, 1);

        // Check that node D is at layer 2
        let node_d = sankey.nodes.iter().find(|n| n.name == "D").unwrap();
        assert_eq!(node_d.layer, 2);

        // Check that links were created
        assert_eq!(sankey.computed_links.len(), 3);
    }
}
