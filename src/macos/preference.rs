#[cfg(feature = "accent-color")]
use crate::AccentColor;
use crate::AvailablePreferences;
#[cfg(feature = "color-scheme")]
use crate::ColorScheme;
#[cfg(feature = "contrast")]
use crate::Contrast;
#[cfg(feature = "reduced-motion")]
use crate::ReducedMotion;
#[cfg(feature = "reduced-transparency")]
use crate::ReducedTransparency;

#[derive(Debug, Clone, Copy)]
pub(crate) enum Preference {
    #[cfg(feature = "color-scheme")]
    ColorScheme(ColorScheme),
    #[cfg(feature = "_macos-accessibility")]
    Accessibility(AccessibilityPreferences),
    #[cfg(feature = "accent-color")]
    AccentColor(AccentColor),
}

#[cfg(feature = "_macos-accessibility")]
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct AccessibilityPreferences {
    #[cfg(feature = "contrast")]
    pub(crate) contrast: Contrast,
    #[cfg(feature = "reduced-motion")]
    pub(crate) reduced_motion: ReducedMotion,
    #[cfg(feature = "reduced-transparency")]
    pub(crate) reduced_transparency: ReducedTransparency,
}

impl Preference {
    pub(crate) fn apply(self, mut preferences: AvailablePreferences) -> AvailablePreferences {
        match self {
            #[cfg(feature = "color-scheme")]
            Preference::ColorScheme(v) => preferences.color_scheme = v,
            #[cfg(feature = "_macos-accessibility")]
            Preference::Accessibility(p) => {
                #[cfg(feature = "contrast")]
                {
                    preferences.contrast = p.contrast;
                }
                #[cfg(feature = "reduced-motion")]
                {
                    preferences.reduced_motion = p.reduced_motion;
                }
                #[cfg(feature = "reduced-transparency")]
                {
                    preferences.reduced_transparency = p.reduced_transparency;
                }
            }
            #[cfg(feature = "accent-color")]
            Preference::AccentColor(v) => preferences.accent_color = v,
        };
        preferences
    }
}
