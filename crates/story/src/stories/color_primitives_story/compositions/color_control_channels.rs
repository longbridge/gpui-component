#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColorControlChannels {
    pub hue: bool,
    pub saturation: bool,
    pub lightness: bool,
    pub value: bool,
    pub a: bool,
    pub b: bool,
    pub alpha: bool,
}

impl ColorControlChannels {
    pub fn none() -> Self {
        Self {
            hue: false,
            saturation: false,
            lightness: false,
            value: false,
            a: false,
            b: false,
            alpha: false,
        }
    }

    #[allow(dead_code)]
    pub fn hue_alpha() -> Self {
        Self {
            hue: true,
            alpha: true,
            ..Self::none()
        }
    }

    #[allow(dead_code)]
    pub fn hsl() -> Self {
        Self {
            hue: true,
            saturation: true,
            lightness: true,
            ..Self::none()
        }
    }

    pub fn hsv() -> Self {
        Self {
            hue: true,
            saturation: true,
            value: true,
            ..Self::none()
        }
    }

    #[allow(dead_code)]
    pub fn lab() -> Self {
        Self {
            lightness: true,
            a: true,
            b: true,
            ..Self::none()
        }
    }

    pub fn photoshop() -> Self {
        Self::hsv()
    }

    pub fn with_alpha(mut self, enabled: bool) -> Self {
        self.alpha = enabled;
        self
    }
}

impl Default for ColorControlChannels {
    fn default() -> Self {
        Self::none()
    }
}
