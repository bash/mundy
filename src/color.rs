/// A color in the sRGB color space. Each component is in the range `[0, 1]`.
///
/// ## Examples
/// ```
/// # use mundy::Srgba;
/// # let color = Srgba { red: 1., green: 0., blue: 0., alpha: 1. };
/// // Convert each channels to u8
/// let (r, g, b, a) = color.to_u8_array().into();
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Srgba {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

impl Srgba {
    pub fn from_f64_array(color: [f64; 4]) -> Self {
        Self {
            red: color[0],
            green: color[1],
            blue: color[2],
            alpha: color[3],
        }
    }

    pub fn to_f64_array(self) -> [f64; 4] {
        [self.red, self.green, self.blue, self.alpha]
    }

    pub fn to_u8_array(self) -> [u8; 4] {
        // Code shamelessly stolen from bevy_color.
        self.to_f64_array()
            .map(|v| (v.clamp(0., 1.) * 255.).round() as u8)
    }

    pub fn from_u8_array(color: [u8; 4]) -> Self {
        Self::from_f64_array(color.map(|c| c as f64 / 255.))
    }
}

#[cfg(feature = "epaint")]
impl From<Srgba> for epaint::Color32 {
    fn from(value: Srgba) -> Self {
        let array = value.to_u8_array();
        epaint::Color32::from_rgba_premultiplied(array[0], array[1], array[2], array[3])
    }
}

#[cfg(feature = "bevy_color")]
impl From<Srgba> for bevy_color::Srgba {
    fn from(value: Srgba) -> Self {
        use bevy_color::ColorToComponents as _;
        bevy_color::Srgba::from_f32_array(value.to_f64_array().map(|c| c as f32))
    }
}

#[cfg(feature = "bevy_color")]
impl From<Srgba> for bevy_color::Color {
    fn from(value: Srgba) -> Self {
        bevy_color::Srgba::from(value).into()
    }
}
