macro_rules! impls {
    ($(
        #[cfg($($meta:meta)*)]
        mod $mod:ident supports { $($feature:literal $setting:ident),* $(,)? };
    )*) => {
        cfg_if::cfg_if! {
            if #[cfg(any())] { }
            $(
                else if #[cfg(all($($meta)*, any($(feature = $feature),*)))]
                {
                    mod $mod;
                    use $mod as imp;
                    impls!(@struct { $($feature $setting),* });
                }
            )*
            else {
                mod fallback;
                use fallback as imp;
                impls!(@struct { });
            }
        }
    };
    (@type color_scheme) => { ColorScheme };
    (@type contrast) => { Contrast };
    (@type reduced_motion) => { ReducedMotion };
    (@type reduced_transparency) => { ReducedTransparency };
    (@type accent_color) => { AccentColor };
    (@type double_click_interval) => { DoubleClickInterval };
    (@struct { $($feature:literal $setting:ident),* }) => {
        #[derive(Debug, Default, Clone, Copy, PartialEq)]
        pub(crate) struct AvailablePreferences {
            $(
                #[cfg(feature = $feature)]
                $setting : impls!(@type $setting),
            )*
        }

        impl From<AvailablePreferences> for Preferences {
            fn from(_value: AvailablePreferences) -> Preferences {
                Preferences {
                    $(
                        #[cfg(feature = $feature)]
                        $setting: _value.$setting,
                    )*
                    ..Default::default()
                }
            }
        }
    }
}
