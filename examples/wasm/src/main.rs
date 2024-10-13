use futures_util::StreamExt as _;
use log::Level;
use mundy::{Interest, Preferences};
use std::error::Error;
use wasm_bindgen::prelude::*;
use web_sys::{window, HtmlElement};

#[wasm_bindgen(main)]
async fn main() -> Result<(), Box<dyn Error>> {
    console_log::init_with_level(Level::Debug)?;

    let window = window().expect("should be called from main thread");
    let document = window.document().expect("document is missing");
    let body = document.body().expect("body is missing");
    let color_scheme: HtmlElement = get_output_element(&body, "#color-scheme");
    let contrast: HtmlElement = get_output_element(&body, "#contrast");
    let reduced_motion: HtmlElement = get_output_element(&body, "#reduced-motion");
    let reduced_transparency: HtmlElement = get_output_element(&body, "#reduced-transparency");
    let accent_color: HtmlElement = get_output_element(&body, "#accent-color");
    let accent_color_sample: HtmlElement = get_output_element(&body, "#accent-color-sample");

    let mut stream = Preferences::stream(Interest::All);
    while let Some(preferences) = stream.next().await {
        color_scheme.set_inner_text(&format!("{:?}", preferences.color_scheme));
        contrast.set_inner_text(&format!("{:?}", preferences.contrast));
        reduced_motion.set_inner_text(&format!("{:?}", preferences.reduced_motion));
        reduced_transparency.set_inner_text(&format!("{:?}", preferences.reduced_transparency));
        accent_color.set_inner_text(&format!("{:?}", preferences.accent_color));

        accent_color_sample.set_hidden(preferences.accent_color.0.is_none());
        if let Some(color) = preferences.accent_color.0 {
            let color = color.to_u8_array();
            let hex_color = format!(
                "#{:02X}{:02X}{:02X}{:02X}",
                color[0], color[1], color[2], color[3]
            );
            accent_color_sample
                .style()
                .set_property("color", &hex_color)
                .unwrap();
        }
    }

    Ok(())
}

fn get_output_element(parent: &HtmlElement, selector: &str) -> HtmlElement {
    parent
        .query_selector(selector)
        .unwrap()
        .unwrap()
        .unchecked_into()
}
