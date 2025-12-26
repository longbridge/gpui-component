use std::fmt::{self, Debug, Display, Formatter};

use gpui::{AbsoluteLength, Axis, Length, Pixels};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A enum for defining the placement of the element.
///
/// See also: [`Side`] if you need to define the left, right side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Placement {
    #[serde(rename = "top")]
    Top,
    #[serde(rename = "bottom")]
    Bottom,
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
}

impl Display for Placement {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Placement::Top => write!(f, "Top"),
            Placement::Bottom => write!(f, "Bottom"),
            Placement::Left => write!(f, "Left"),
            Placement::Right => write!(f, "Right"),
        }
    }
}

impl Placement {
    #[inline]
    pub fn is_horizontal(&self) -> bool {
        match self {
            Placement::Left | Placement::Right => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_vertical(&self) -> bool {
        match self {
            Placement::Top | Placement::Bottom => true,
            _ => false,
        }
    }

    #[inline]
    pub fn axis(&self) -> Axis {
        match self {
            Placement::Top | Placement::Bottom => Axis::Vertical,
            Placement::Left | Placement::Right => Axis::Horizontal,
        }
    }
}

/// The anchor position of an element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub enum Anchor {
    #[default]
    #[serde(rename = "top-left")]
    TopLeft,
    #[serde(rename = "top-center")]
    TopCenter,
    #[serde(rename = "top-right")]
    TopRight,
    #[serde(rename = "bottom-left")]
    BottomLeft,
    #[serde(rename = "bottom-center")]
    BottomCenter,
    #[serde(rename = "bottom-right")]
    BottomRight,
}

impl Display for Anchor {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Anchor::TopLeft => write!(f, "TopLeft"),
            Anchor::TopCenter => write!(f, "TopCenter"),
            Anchor::TopRight => write!(f, "TopRight"),
            Anchor::BottomLeft => write!(f, "BottomLeft"),
            Anchor::BottomCenter => write!(f, "BottomCenter"),
            Anchor::BottomRight => write!(f, "BottomRight"),
        }
    }
}

impl Anchor {
    /// Returns true if the anchor is at the top.
    #[inline]
    pub fn is_top(&self) -> bool {
        matches!(self, Self::TopLeft | Self::TopCenter | Self::TopRight)
    }

    /// Returns true if the anchor is at the bottom.
    #[inline]
    pub fn is_bottom(&self) -> bool {
        matches!(
            self,
            Self::BottomLeft | Self::BottomCenter | Self::BottomRight
        )
    }

    /// Returns true if the anchor is at the left.
    #[inline]
    pub fn is_left(&self) -> bool {
        matches!(self, Self::TopLeft | Self::BottomLeft)
    }

    /// Returns true if the anchor is at the right.
    #[inline]
    pub fn is_right(&self) -> bool {
        matches!(self, Self::TopRight | Self::BottomRight)
    }

    /// Returns true if the anchor is at the center.
    #[inline]
    pub fn is_center(&self) -> bool {
        matches!(self, Self::TopCenter | Self::BottomCenter)
    }
}

/// A enum for defining the side of the element.
///
/// See also: [`Placement`] if you need to define the 4 edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
}

impl Side {
    /// Returns true if the side is left.
    #[inline]
    pub fn is_left(&self) -> bool {
        matches!(self, Self::Left)
    }

    /// Returns true if the side is right.
    #[inline]
    pub fn is_right(&self) -> bool {
        matches!(self, Self::Right)
    }
}

/// A trait to extend the [`Axis`] enum with utility methods.
pub trait AxisExt {
    fn is_horizontal(self) -> bool;
    fn is_vertical(self) -> bool;
}

impl AxisExt for Axis {
    #[inline]
    fn is_horizontal(self) -> bool {
        self == Axis::Horizontal
    }

    #[inline]
    fn is_vertical(self) -> bool {
        self == Axis::Vertical
    }
}

/// A trait for converting [`Pixels`] to `f32` and `f64`.
pub trait PixelsExt {
    fn as_f32(&self) -> f32;
    fn as_f64(self) -> f64;
}
impl PixelsExt for Pixels {
    fn as_f32(&self) -> f32 {
        f32::from(self)
    }

    fn as_f64(self) -> f64 {
        f64::from(self)
    }
}

/// A trait to extend the [`Length`] enum with utility methods.
pub trait LengthExt {
    /// Converts the [`Length`] to [`Pixels`] based on a given `base_size` and `rem_size`.
    ///
    /// If the [`Length`] is [`Length::Auto`], it returns `None`.
    fn to_pixels(&self, base_size: AbsoluteLength, rem_size: Pixels) -> Option<Pixels>;
}

impl LengthExt for Length {
    fn to_pixels(&self, base_size: AbsoluteLength, rem_size: Pixels) -> Option<Pixels> {
        match self {
            Length::Auto => None,
            Length::Definite(len) => Some(len.to_pixels(base_size, rem_size)),
        }
    }
}

/// A struct for defining the edges of an element.
///
/// A extend version of [`gpui::Edges`] to serialize/deserialize.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, Eq, PartialEq)]
#[repr(C)]
pub struct Edges<T: Clone + Debug + Default + PartialEq> {
    /// The size of the top edge.
    pub top: T,
    /// The size of the right edge.
    pub right: T,
    /// The size of the bottom edge.
    pub bottom: T,
    /// The size of the left edge.
    pub left: T,
}

impl<T> Edges<T>
where
    T: Clone + Debug + Default + PartialEq,
{
    /// Creates a new `Edges` instance with all edges set to the same value.
    pub fn all(value: T) -> Self {
        Self {
            top: value.clone(),
            right: value.clone(),
            bottom: value.clone(),
            left: value,
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::px;

    use super::Placement;
    #[test]
    fn test_placement() {
        assert!(Placement::Left.is_horizontal());
        assert!(Placement::Right.is_horizontal());
        assert!(!Placement::Top.is_horizontal());
        assert!(!Placement::Bottom.is_horizontal());

        assert!(Placement::Top.is_vertical());
        assert!(Placement::Bottom.is_vertical());
        assert!(!Placement::Left.is_vertical());
        assert!(!Placement::Right.is_vertical());

        assert_eq!(Placement::Top.axis(), gpui::Axis::Vertical);
        assert_eq!(Placement::Bottom.axis(), gpui::Axis::Vertical);
        assert_eq!(Placement::Left.axis(), gpui::Axis::Horizontal);
        assert_eq!(Placement::Right.axis(), gpui::Axis::Horizontal);

        assert_eq!(Placement::Top.to_string(), "Top");
        assert_eq!(Placement::Bottom.to_string(), "Bottom");
        assert_eq!(Placement::Left.to_string(), "Left");
        assert_eq!(Placement::Right.to_string(), "Right");

        assert_eq!(serde_json::to_string(&Placement::Top).unwrap(), r#""top""#);
        assert_eq!(
            serde_json::to_string(&Placement::Bottom).unwrap(),
            r#""bottom""#
        );
        assert_eq!(
            serde_json::to_string(&Placement::Left).unwrap(),
            r#""left""#
        );
        assert_eq!(
            serde_json::to_string(&Placement::Right).unwrap(),
            r#""right""#
        );

        assert_eq!(
            serde_json::from_str::<Placement>(r#""top""#).unwrap(),
            Placement::Top
        );
        assert_eq!(
            serde_json::from_str::<Placement>(r#""bottom""#).unwrap(),
            Placement::Bottom
        );
        assert_eq!(
            serde_json::from_str::<Placement>(r#""left""#).unwrap(),
            Placement::Left
        );
        assert_eq!(
            serde_json::from_str::<Placement>(r#""right""#).unwrap(),
            Placement::Right
        );
    }

    #[test]
    fn test_side() {
        use super::Side;
        let left = Side::Left;
        let right = Side::Right;

        assert!(left.is_left());
        assert!(!left.is_right());

        assert!(right.is_right());
        assert!(!right.is_left());

        // Test serialization
        assert_eq!(serde_json::to_string(&left).unwrap(), r#""left""#);
        assert_eq!(serde_json::to_string(&right).unwrap(), r#""right""#);
        assert_eq!(
            serde_json::from_str::<Side>(r#""left""#).unwrap(),
            Side::Left
        );
        assert_eq!(
            serde_json::from_str::<Side>(r#""right""#).unwrap(),
            Side::Right
        );
    }

    #[test]
    fn test_anchor() {
        use super::Anchor;

        assert_eq!(Anchor::default(), Anchor::TopLeft);

        assert_eq!(Anchor::TopLeft.to_string(), "TopLeft");
        assert_eq!(Anchor::TopCenter.to_string(), "TopCenter");
        assert_eq!(Anchor::TopRight.to_string(), "TopRight");
        assert_eq!(Anchor::BottomLeft.to_string(), "BottomLeft");
        assert_eq!(Anchor::BottomCenter.to_string(), "BottomCenter");
        assert_eq!(Anchor::BottomRight.to_string(), "BottomRight");

        // Test serialization
        assert_eq!(
            serde_json::to_string(&Anchor::TopLeft).unwrap(),
            r#""top-left""#
        );
        assert_eq!(
            serde_json::to_string(&Anchor::TopCenter).unwrap(),
            r#""top-center""#
        );
        assert_eq!(
            serde_json::to_string(&Anchor::TopRight).unwrap(),
            r#""top-right""#
        );
        assert_eq!(
            serde_json::to_string(&Anchor::BottomLeft).unwrap(),
            r#""bottom-left""#
        );
        assert_eq!(
            serde_json::to_string(&Anchor::BottomCenter).unwrap(),
            r#""bottom-center""#
        );
        assert_eq!(
            serde_json::to_string(&Anchor::BottomRight).unwrap(),
            r#""bottom-right""#
        );

        // Test deserialization
        assert_eq!(
            serde_json::from_str::<Anchor>(r#""top-left""#).unwrap(),
            Anchor::TopLeft
        );
        assert_eq!(
            serde_json::from_str::<Anchor>(r#""top-center""#).unwrap(),
            Anchor::TopCenter
        );
        assert_eq!(
            serde_json::from_str::<Anchor>(r#""top-right""#).unwrap(),
            Anchor::TopRight
        );
        assert_eq!(
            serde_json::from_str::<Anchor>(r#""bottom-left""#).unwrap(),
            Anchor::BottomLeft
        );
        assert_eq!(
            serde_json::from_str::<Anchor>(r#""bottom-center""#).unwrap(),
            Anchor::BottomCenter
        );
        assert_eq!(
            serde_json::from_str::<Anchor>(r#""bottom-right""#).unwrap(),
            Anchor::BottomRight
        );

        // Test is_top
        assert!(Anchor::TopLeft.is_top());
        assert!(Anchor::TopCenter.is_top());
        assert!(Anchor::TopRight.is_top());
        assert!(!Anchor::BottomLeft.is_top());
        assert!(!Anchor::BottomCenter.is_top());
        assert!(!Anchor::BottomRight.is_top());

        // Test is_bottom
        assert!(Anchor::BottomLeft.is_bottom());
        assert!(Anchor::BottomCenter.is_bottom());
        assert!(Anchor::BottomRight.is_bottom());
        assert!(!Anchor::TopLeft.is_bottom());
        assert!(!Anchor::TopCenter.is_bottom());
        assert!(!Anchor::TopRight.is_bottom());

        // Test is_left
        assert!(Anchor::TopLeft.is_left());
        assert!(Anchor::BottomLeft.is_left());
        assert!(!Anchor::TopCenter.is_left());
        assert!(!Anchor::BottomCenter.is_left());
        assert!(!Anchor::TopRight.is_left());
        assert!(!Anchor::BottomRight.is_left());

        // Test is_right
        assert!(Anchor::TopRight.is_right());
        assert!(Anchor::BottomRight.is_right());
        assert!(!Anchor::TopLeft.is_right());
        assert!(!Anchor::BottomLeft.is_right());
        assert!(!Anchor::TopCenter.is_right());
        assert!(!Anchor::BottomCenter.is_right());

        // Test is_center
        assert!(Anchor::TopCenter.is_center());
        assert!(Anchor::BottomCenter.is_center());
        assert!(!Anchor::TopLeft.is_center());
        assert!(!Anchor::TopRight.is_center());
        assert!(!Anchor::BottomLeft.is_center());
        assert!(!Anchor::BottomRight.is_center());
    }

    #[test]
    fn test_edges_pixels() {
        use super::Edges;
        use gpui::Pixels;

        let edge_value = px(10.0);
        let edges = Edges::all(edge_value);

        assert_eq!(edges.top, edge_value);
        assert_eq!(edges.right, edge_value);
        assert_eq!(edges.bottom, edge_value);
        assert_eq!(edges.left, edge_value);

        let custom_edges = Edges {
            top: px(5.0),
            right: px(10.0),
            bottom: px(15.0),
            left: px(20.0),
        };

        assert_eq!(custom_edges.top, px(5.0));
        assert_eq!(custom_edges.right, px(10.0));
        assert_eq!(custom_edges.bottom, px(15.0));
        assert_eq!(custom_edges.left, px(20.0));

        // Test serialization
        let serialized = serde_json::to_string(&custom_edges).unwrap();
        assert_eq!(
            serialized,
            r#"{"top":5.0,"right":10.0,"bottom":15.0,"left":20.0}"#
        );

        // Test deserialization
        let deserialized: Edges<Pixels> = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, custom_edges);
    }
}
