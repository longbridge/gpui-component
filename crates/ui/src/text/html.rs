extern crate markup5ever_rcdom as rcdom;

use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::{Handle, NodeData, RcDom};

use super::element;

pub(super) fn parse_html(source: &str) -> Result<element::Node, std::io::Error> {
    let mut bytes = source.as_bytes();
    // Ref
    // https://github.com/servo/html5ever/blob/main/rcdom/examples/print-rcdom.rs
    let dom = parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut bytes)?;

    Ok(dom.document.into())
}

impl From<Handle> for element::Node {
    fn from(node: Handle) -> Self {
        match &node.data {
            NodeData::Text { ref contents } => {
                let text = contents.borrow().to_string();
                element::Node::Paragraph(text.into())
            }
            NodeData::Element {
                ref name, attrs, ..
            } => {
                let name = name.local.to_string();
                todo!()
            }
            NodeData::Document => element::Node::Ignore,
            NodeData::Doctype { .. } => element::Node::Ignore,
            NodeData::Comment { .. } => element::Node::Ignore,
            NodeData::ProcessingInstruction { .. } => element::Node::Ignore,
            _ => {
                if cfg!(debug_assertions) {
                    eprintln!("[html] Unhandled node: {:?}", node);
                }

                element::Node::Unknown
            }
        }
    }
}
