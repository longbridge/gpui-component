use std::fmt::{self, Display, Formatter};

use gpui::{AbsoluteLength, Axis, Length, Pixels};
use serde::{Deserialize, Serialize};

/// A enum for defining the placement of the element.
///
/// See also: [`Side`] if you need to define the left, right side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
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
}
