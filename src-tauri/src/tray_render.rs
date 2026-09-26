//! Native rendering of the status bar item.
//!
//! A plain `NSStatusItem` title is one line of proportional text, so the
//! width jumps as digits change and a second metric has to be squeezed onto
//! the same line (`43.5 w  ↓34K ↑19K`). Instead the label is drawn into a
//! template `NSImage`: tabular digits, fixed-width slots so the item does not
//! jiggle, and small two-line stacks for the extra metrics, the layout the
//! design prototype used. Being a template image, AppKit tints it for light
//! and dark menu bars and for the highlighted state by itself.

use block2::RcBlock;
use objc2::{rc::Retained, runtime::Bool};
use objc2_app_kit::{
    NSAutoresizingMaskOptions, NSCellImagePosition, NSColor, NSFont, NSFontAttributeName,
    NSFontWeightMedium, NSFontWeightSemibold, NSForegroundColorAttributeName, NSImage,
    NSImageScaling, NSStatusBarButton, NSStringDrawing,
};
use objc2_foundation::{NSDictionary, NSPoint, NSRect, NSSize, NSString};
use serde::{Deserialize, Serialize};
use specta::Type;

/// Menu bar content height in points. The item is 22-24pt tall depending on
/// the display; 18 leaves the usual inset.
const HEIGHT: f64 = 18.0;
const PRIMARY_SIZE: f64 = 13.0;
const STACK_SIZE: f64 = 9.0;
/// Gap between the power figure and a stack, and between stacks.
const GAP: f64 = 5.0;
/// Vertical gap between the two rows of a stack.
const STACK_GAP: f64 = 3.0;

/// What the status bar shows. Built on the power tick, drawn on the main
/// thread.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Type)]
#[serde(rename_all = "camelCase")]
pub struct TrayLabel {
    /// The power reading, e.g. `12.3w`.
    pub primary: String,
    /// Two-line stacks drawn after it, e.g. `[["↓1.2M", "↑86K"]]`.
    pub stacks: Vec<[String; 2]>,
}

impl TrayLabel {
    pub fn power(watts: f32) -> Self {
        Self {
            primary: format!("{watts:.1}w"),
            stacks: Vec::new(),
        }
    }

    /// Single-line fallback, used for the accessibility title.
    pub fn plain(&self) -> String {
        let mut out = self.primary.clone();
        for [a, b] in &self.stacks {
            out.push_str(&format!(" {a} {b}"));
        }
        out
    }
}

/// Width reserved for `text`, rounded up to the width of a template string of
/// the same shape, so `9.8w` and `12.3w` or `8%` and `100%` occupy one slot
/// and the item does not change size every tick.
fn slot_template(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_ascii_digit() { '0' } else { c })
        .collect::<String>()
}

fn attrs(size: f64, weight: f64) -> Retained<NSDictionary<NSString, objc2::runtime::AnyObject>> {
    unsafe {
        let font = NSFont::monospacedDigitSystemFontOfSize_weight(size, weight);
        // Any opaque colour works: a template image only uses alpha.
        let color = NSColor::blackColor();
        NSDictionary::from_vec(
            &[NSFontAttributeName, NSForegroundColorAttributeName],
            vec![
                Retained::cast::<objc2::runtime::AnyObject>(font),
                Retained::cast::<objc2::runtime::AnyObject>(color),
            ],
        )
    }
}

fn text_width(text: &str, attrs: &NSDictionary<NSString, objc2::runtime::AnyObject>) -> f64 {
    unsafe {
        NSString::from_str(text)
            .sizeWithAttributes(Some(attrs))
            .width
    }
}

/// Minimum widths per slot, which only ever grow while the app runs.
///
/// Without this a reading that drops from `100%` to `99%` would shrink the
/// item. Growing is allowed since clipping a value would be worse than a
/// one-off shift.
#[derive(Default)]
pub struct SlotWidths {
    primary: f64,
    stacks: Vec<f64>,
}

impl SlotWidths {
    fn fit(&mut self, index: Option<usize>, width: f64) -> f64 {
        let slot = match index {
            None => &mut self.primary,
            Some(i) => {
                if self.stacks.len() <= i {
                    self.stacks.resize(i + 1, 0.0);
                }
                &mut self.stacks[i]
            }
        };
        *slot = slot.max(width.ceil());
        *slot
    }

    /// Called when the layout changes shape (a stack added or removed), so
    /// widths from the old layout do not linger.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Draw `label` into `button` as a template image.
///
/// # Safety
/// Must run on the main thread; `button` must be the status item's button.
pub unsafe fn render(button: &NSStatusBarButton, label: &TrayLabel, slots: &mut SlotWidths) {
    if slots.stacks.len() != label.stacks.len() {
        slots.reset();
    }

    let primary_attrs = attrs(PRIMARY_SIZE, NSFontWeightMedium);
    let stack_attrs = attrs(STACK_SIZE, NSFontWeightSemibold);

    let primary_w = slots.fit(
        None,
        text_width(&slot_template(&label.primary), &primary_attrs)
            .max(text_width(&label.primary, &primary_attrs)),
    );
    let stack_ws: Vec<f64> = label
        .stacks
        .iter()
        .enumerate()
        .map(|(i, [a, b])| {
            let w = [a, b]
                .iter()
                .map(|s| {
                    text_width(&slot_template(s), &stack_attrs).max(text_width(s, &stack_attrs))
                })
                .fold(0.0, f64::max);
            slots.fit(Some(i), w)
        })
        .collect();

    let width = primary_w + stack_ws.iter().map(|w| w + GAP).sum::<f64>();
    let plain = label.plain();
    let label = label.clone();

    // The handler runs whenever AppKit needs the image at a given scale, so
    // it is resolution independent and redraws itself for Retina.
    let handler = RcBlock::new(move |_rect: NSRect| -> Bool {
        let primary_font =
            NSFont::monospacedDigitSystemFontOfSize_weight(PRIMARY_SIZE, NSFontWeightMedium);
        let stack_font =
            NSFont::monospacedDigitSystemFontOfSize_weight(STACK_SIZE, NSFontWeightSemibold);
        let primary_attrs = attrs(PRIMARY_SIZE, NSFontWeightMedium);
        let stack_attrs = attrs(STACK_SIZE, NSFontWeightSemibold);

        // `drawAtPoint` places the bottom of the line box, which sits one
        // descender below the baseline. Lines are centred on cap height,
        // which is what reads as the middle of a row of digits.
        let origin_for = |baseline: f64, font: &NSFont| baseline + font.descender();

        let primary_baseline = (HEIGHT - primary_font.capHeight()) / 2.0;
        // Right-aligned within its slot so the decimal point stays put.
        let pw = text_width(&label.primary, &primary_attrs);
        NSString::from_str(&label.primary).drawAtPoint_withAttributes(
            NSPoint::new(primary_w - pw, origin_for(primary_baseline, &primary_font)),
            Some(&primary_attrs),
        );

        // Two rows of cap height with a gap, centred as one block.
        let cap = stack_font.capHeight();
        let block = cap * 2.0 + STACK_GAP;
        let bottom_baseline = (HEIGHT - block) / 2.0;
        let top_baseline = bottom_baseline + cap + STACK_GAP;
        let mut x = primary_w + GAP;
        for (i, [top, bot]) in label.stacks.iter().enumerate() {
            let w = stack_ws[i];
            for (text, baseline) in [(top, top_baseline), (bot, bottom_baseline)] {
                let tw = text_width(text, &stack_attrs);
                NSString::from_str(text).drawAtPoint_withAttributes(
                    NSPoint::new(x + w - tw, origin_for(baseline, &stack_font)),
                    Some(&stack_attrs),
                );
            }
            x += w + GAP;
        }
        Bool::YES
    });

    let image =
        NSImage::imageWithSize_flipped_drawingHandler(NSSize::new(width, HEIGHT), false, &handler);
    image.setTemplate(true);

    button.setTitle(&NSString::from_str(""));
    button.setImage(Some(&image));
    button.setImagePosition(NSCellImagePosition::NSImageOnly);
    button.setImageScaling(NSImageScaling::NSImageScaleNone);
    // The image carries no text, so give VoiceOver the reading.
    let text = NSString::from_str(&plain);
    let _: () = objc2::msg_send![button, setAccessibilityLabel: &*text];

    // tray-icon lays a transparent click-target view over the button and only
    // resizes it from `set_title`. Keep it tracking the button, or clicks on
    // the part of a wider item beyond the old frame would be lost.
    let bounds = button.bounds();
    for view in button.subviews().iter() {
        view.setFrame(bounds);
        view.setAutoresizingMask(
            NSAutoresizingMaskOptions::NSViewWidthSizable
                | NSAutoresizingMaskOptions::NSViewHeightSizable,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_template_normalises_digits_only() {
        assert_eq!(slot_template("12.3w"), "00.0w");
        assert_eq!(slot_template("↓1.2M"), "↓0.0M");
        assert_eq!(slot_template("C100%"), "C000%");
    }

    #[test]
    fn plain_joins_stacks() {
        let label = TrayLabel {
            primary: "12.3w".into(),
            stacks: vec![["↓1.2M".into(), "↑86K".into()]],
        };
        assert_eq!(label.plain(), "12.3w ↓1.2M ↑86K");
    }
}

#[cfg(test)]
mod preview {
    use super::*;
    use objc2::msg_send_id;
    use objc2_foundation::NSData;

    /// Writes each layout to `$TRICKLE_TRAY_PREVIEW/*.tiff` for eyeballing.
    /// Run with: TRICKLE_TRAY_PREVIEW=/tmp/x cargo test -p trickle tray_preview -- --ignored
    #[test]
    #[ignore]
    fn tray_preview() {
        let Ok(dir) = std::env::var("TRICKLE_TRAY_PREVIEW") else {
            return;
        };
        let s = |a: &str, b: &str| [a.to_string(), b.to_string()];
        let cases = [
            ("power", vec![]),
            ("cpu", vec![s("CPU", "23%")]),
            ("network", vec![s("↓1.2M", "↑86K")]),
            ("compact", vec![s("C 23%", "G 8%"), s("M 61%", "↓1.2M")]),
        ];
        unsafe {
            let mtm = objc2_foundation::MainThreadMarker::new_unchecked();
            let button: Retained<NSStatusBarButton> =
                msg_send_id![mtm.alloc::<NSStatusBarButton>(), init];
            for (name, stacks) in cases {
                let label = TrayLabel {
                    primary: "12.3w".into(),
                    stacks,
                };
                render(&button, &label, &mut SlotWidths::default());
                let image = button.image().unwrap();
                let data: Retained<NSData> = image.TIFFRepresentation().unwrap();
                std::fs::write(format!("{dir}/{name}.tiff"), data.bytes()).unwrap();
                println!("{name}: {:?}", image.size());
            }
        }
    }
}
