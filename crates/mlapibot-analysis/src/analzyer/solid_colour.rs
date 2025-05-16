use hex_color::HexColor;
use image::{GenericImage, GenericImageView};
use mlapibot_common::{DetectedItem, Detection};

fn default_tolerance() -> u8 {
    1
}

#[derive(Debug, serde::Deserialize)]
pub struct SolidColourAnalyzer {
    colours: Vec<HexColor>,
    #[serde(default = "default_tolerance")]
    tolerance: u8,
    threshold: f32,
}

fn saturating_add(clr: &HexColor, v: i8) -> HexColor {
    HexColor::rgba(
        clr.r.saturating_add_signed(v),
        clr.g.saturating_add_signed(v),
        clr.b.saturating_add_signed(v),
        clr.a.saturating_add_signed(v),
    )
}

impl SolidColourAnalyzer {
    pub fn analyze(
        &self,
        context: &crate::context::Context,
    ) -> crate::error::Result<Option<Detection>> {
        let colours: Vec<_> = self
            .colours
            .iter()
            .map(|colour| {
                let min_hex = saturating_add(colour, -1);
                let max_hex = saturating_add(colour, 1);

                (min_hex, max_hex)
            })
            .collect();

        let mut detected_on = Vec::new();

        for (idx, img) in context.images.iter().enumerate() {
            let img = img.image();
            let mut matching_pixels = 0;
            let total_pixels = img.width() * img.height();

            for y in 0..img.height() {
                for x in 0..img.width() {
                    let pixel = img.get_pixel(x, y);

                    let [r, g, b, a] = pixel.0;

                    let matches = colours.iter().any(|(min_hex, max_hex)| {
                        (r >= min_hex.r && r <= max_hex.r)
                            && (g >= min_hex.g && g <= max_hex.g)
                            && (b >= min_hex.b && b <= max_hex.b)
                            && (a >= min_hex.a && a <= max_hex.a)
                    });

                    if matches {
                        matching_pixels += 1;
                    }
                }
            }

            let perc = matching_pixels as f32 / total_pixels as f32;

            if perc >= self.threshold {
                detected_on.push((idx, DetectedItem::new(perc)));
            }
        }

        if detected_on.len() > 0 {
            let mut det = Detection::new();

            for (idx, item) in detected_on {
                det.add_image(idx, item);
            }

            Ok(Some(det))
        } else {
            Ok(None)
        }
    }
}
