use std::fmt;

/// A color in the sRGB color space. Each component is in the range `[0, 1]`.
///
/// ## Examples
/// ```
/// # use mundy::Srgba;
/// # let color = Srgba { red: 1., green: 0., blue: 0., alpha: 1. };
/// // Convert each channels to u8
/// let (r, g, b, a) = color.to_u8_array().into();
/// ```
#[derive(Clone, Copy, PartialEq)]
pub struct Srgba {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

impl fmt::Debug for Srgba {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Srgba")
            .field("_", &Hex(self.to_u8_array()))
            .field("red", &self.red)
            .field("green", &self.green)
            .field("blue", &self.blue)
            .field("alpha", &self.alpha)
            .finish()
    }
}

struct Hex([u8; 4]);

impl fmt::Debug for Hex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "#{r:02x}{g:02x}{b:02x}{a:02x}",
            r = self.0[0],
            g = self.0[1],
            b = self.0[2],
            a = self.0[3]
        )
    }
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
