use super::event_listener::{EventListenerGuard, EventTargetExt};
use crate::{AccentColor, Srgba};
use std::ops::Deref;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{css, Comment, HtmlElement, TransitionEvent, Window};

type JsResult<T> = Result<T, JsValue>;

/// Detects the accent color by creating an element with `color: AccentColor`
// spellchecker:off
/// and reading out the computed value. Detection works by (mis-)using transitions
// spellchecker:on
/// and the `transitionstart` event.
pub(crate) struct AccentColorObserver {
    _element: OffscreenElement,
    _guard: EventListenerGuard,
}

impl AccentColorObserver {
    pub(crate) fn new(
        window: &Window,
        callback: impl FnMut(AccentColor) + 'static,
    ) -> Option<(Self, AccentColor)> {
        if !supports_accent_color() {
            return None;
        }
        let element = create_element(window)?;
        let guard = add_accent_color_listener(window.clone(), element.0.clone(), callback);
        let initial_value = get_accent_color_from_computed_style(window, &element.0);
        let observer = Self {
            _element: element,
            _guard: guard?,
        };
        Some((observer, initial_value))
    }
}

fn supports_accent_color() -> bool {
    css::supports("color: AccentColor").unwrap_or_default()
}

fn add_accent_color_listener(
    window: Window,
    element: HtmlElement,
    mut callback: impl FnMut(AccentColor) + 'static,
) -> Option<EventListenerGuard> {
    add_color_change_listener(&element.clone(), move || {
        callback(get_accent_color_from_computed_style(&window, &element))
    })
    .ok()
}

fn create_element(window: &Window) -> Option<OffscreenElement> {
    const COMMENT: &str = concat!(
        "this element is used by the '",
        env!("CARGO_PKG_NAME"),
        "' crate to detect changes to the system accent color"
    );
    let element = OffscreenElement::new(window, COMMENT)?;
    element.style().set_property("color", "AccentColor").ok()?;
    Some(element)
}

// This trick is adapted from @bramus' StyleObserver:
// <https://github.com/bramus/style-observer>.
//
// Note that the element has to be attached to the DOM
// and "visible" (not hidden via the `hidden` attribute or similar)
// for transitions to happen.
fn add_color_change_listener(
    element: &HtmlElement,
    mut f: impl FnMut() + 'static,
) -> JsResult<EventListenerGuard> {
    let style = element.style();
    style.set_property("transition", "color 0.001ms step-start")?;
    element.add_event_listener("transitionstart", move |event: TransitionEvent| {
        if event.property_name() == "color" {
            f();
        }
    })
}

fn get_accent_color_from_computed_style(window: &Window, element: &HtmlElement) -> AccentColor {
    AccentColor(get_color_from_computed_style(window, element))
}

fn get_color_from_computed_style(window: &Window, element: &HtmlElement) -> Option<Srgba> {
    let style = window.get_computed_style(element).ok().flatten()?;
    let value = style.get_property_value("color").ok()?;
    parse_css_color_value(&value)
}

/// An offscreeen and inert HTML element that's removed on drop.
#[derive(Clone)]
struct OffscreenElement(HtmlElement);

impl OffscreenElement {
    fn new(window: &Window, description: &str) -> Option<Self> {
        let document = window.document()?;
        let body = document.body()?;
        let element: HtmlElement = document.create_element("div").ok()?.unchecked_into();
        let comment: Comment = document.create_comment(description);
        _ = element.append_child(&comment);
        element.set_attribute("aria-hidden", "true").ok()?;
        _ = element.set_attribute("inert", "");
        set_offscreen(&element);
        body.append_child(&element).ok()?;
        Some(Self(element))
    }
}

impl Deref for OffscreenElement {
    type Target = HtmlElement;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn set_offscreen(element: &HtmlElement) {
    let style = element.style();
    _ = style.set_property("position", "fixed");
    _ = style.set_property("left", "-100%");
    _ = style.set_property("top", "-100%");
}

impl Drop for OffscreenElement {
    fn drop(&mut self) {
        self.0.remove();
    }
}

// Some excerpts from <https://www.w3.org/TR/css-color-4/#serializing-color-values>
// are sprinkled throughout this code for clarity.
fn parse_css_color_value(s: &str) -> Option<Srgba> {
    // > [..] For compatibility, the legacy form with comma separators is used; exactly one ASCII space follows each comma.
    // > This includes the comma (not slash) used to separate the blue component of rgba() from the alpha value.
    const SEPARATOR: &str = ", ";

    // > [..] Also, for compatibility, if the alpha is exactly 1, the rgb() form is used,
    // with an implicit alpha; otherwise, the rgba() form is used, with an explicit alpha value.
    if let Some(parts) = rgb(s) {
        let mut parts = parts.splitn(3, SEPARATOR);
        Some(Srgba::from_f64_array([
            component(&mut parts)? / 255.,
            component(&mut parts)? / 255.,
            component(&mut parts)? / 255.,
            1.,
        ]))
    } else if let Some(parts) = rgba(s) {
        let mut parts = parts.splitn(4, SEPARATOR);
        Some(Srgba::from_f64_array([
            component(&mut parts)? / 255.,
            component(&mut parts)? / 255.,
            component(&mut parts)? / 255.,
            component(&mut parts)?,
        ]))
    } else {
        None
    }
}

fn rgb(s: &str) -> Option<&str> {
    s.strip_prefix("rgb(").and_then(|s| s.strip_suffix(")"))
}

fn rgba(s: &str) -> Option<&str> {
    s.strip_prefix("rgba(").and_then(|s| s.strip_suffix(")"))
}

// > [..] authors of scripts which expect color values returned from getComputedStyle to have <integer> component values,
// > are advised to update them to also cope with <number>.
fn component<'a>(parts: &mut impl Iterator<Item = &'a str>) -> Option<f64> {
    let value = parts.next()?;
    value.parse().ok()
}

// TODO: unit tests for the parser
