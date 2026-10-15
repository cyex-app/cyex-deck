//! V1051-03 A v4 paint into the one renderer: same `Paint`, same `style()`, same CSS.

use super::{Layer, Paint, Shadow, Stop};
use crate::seventv::account::model::{
    V4Color, V4Image, V4Paint, V4Shadow, V4Stop, V4Ty, best_webp, still_webp,
};

pub(crate) fn from_v4(p: &V4Paint) -> Paint {
    let mut color = None;
    let mut layers = Vec::new();
    for l in &p.data.layers {
        match &l.ty {
            V4Ty::SingleColor { color: c } => color = Some(packed(c)),
            V4Ty::Linear {
                angle,
                repeating,
                stops,
            } => layers.push(Layer {
                function: Some("LINEAR_GRADIENT".into()),
                repeat: *repeating,
                angle: *angle,
                stops: stops.iter().map(stop).collect(),
                ..Layer::default()
            }),
            V4Ty::Radial {
                shape,
                repeating,
                stops,
            } => layers.push(Layer {
                function: Some("RADIAL_GRADIENT".into()),
                repeat: *repeating,
                shape: Some(shape.to_lowercase()),
                stops: stops.iter().map(stop).collect(),
                ..Layer::default()
            }),
            V4Ty::Image { images } => layers.push(image_layer(images)),
            V4Ty::Other => {}
        }
    }
    let (layer, gradients) = if layers.len() == 1 {
        (layers.remove(0), Vec::new())
    } else {
        (Layer::default(), layers)
    };
    Paint {
        id: p.id.clone(),
        name: p.name.clone(),
        layer,
        gradients,
        shadows: p.data.shadows.iter().map(shadow).collect(),
        color,
    }
}

fn image_layer(images: &[V4Image]) -> Layer {
    let main = best_webp(images, 1).and_then(V4Image::safe);
    Layer {
        function: Some("URL".into()),
        // A static picture has no `_static` twin, so it is its own still.
        still_url: still_webp(images, 1)
            .and_then(V4Image::safe)
            .or_else(|| main.clone()),
        image_url: main,
        ..Layer::default()
    }
}

fn stop(s: &V4Stop) -> Stop {
    Stop {
        at: s.at,
        color: packed(&s.color),
    }
}

fn shadow(s: &V4Shadow) -> Shadow {
    Shadow {
        x_offset: s.offset_x,
        y_offset: s.offset_y,
        radius: s.blur,
        color: packed(&s.color),
    }
}

/// `#RRGGBBAA` (or `#RRGGBB`) as 7TV's packed RGBA.
fn packed(c: &V4Color) -> i64 {
    let h = c.hex.trim_start_matches('#');
    let h = if h.len() == 6 {
        format!("{h}ff")
    } else {
        h.to_owned()
    };
    i64::from(u32::from_str_radix(&h, 16).unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seventv::account::model::{PaintEntry, parse_user};

    fn cyexbot() -> Vec<PaintEntry> {
        let path = format!(
            "{}/tests/fixtures/7tv/v4_inventory_cyexbot.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        parse_user(v.pointer("/data/users/userByConnection").unwrap())
            .unwrap()
            .unwrap()
            .paints
    }

    #[test]
    fn linear_paint_with_shadows() {
        let css = cyexbot()[0].paint.style().css;
        assert!(css.starts_with("background-image:linear-gradient("), "{css}");
        assert!(css.contains("--paint-shadow:drop-shadow("), "{css}");
    }

    #[test]
    fn image_paint_has_an_explicit_still() {
        let e = &cyexbot()[2];
        assert!(e.animated);
        let s = e.paint.style();
        assert!(s.css.contains("/1x.webp"), "{}", s.css);
        let still = s.still.unwrap();
        assert!(still.contains("/1x_static.webp"), "{still}");
    }

    #[test]
    fn hex_with_alpha() {
        let p = Paint {
            id: "x".into(),
            name: "x".into(),
            layer: Layer::default(),
            gradients: Vec::new(),
            shadows: Vec::new(),
            color: Some(packed(&V4Color {
                hex: "#ff000080".into(),
            })),
        };
        assert_eq!(p.style().css, "color:rgba(255,0,0,0.502);");
    }

    #[test]
    fn same_css_as_the_v3_twin() {
        let v4: V4Paint = serde_json::from_str(
            r##"{"id":"p","name":"p","data":{"layers":[{"ty":{"__typename":"PaintLayerTypeLinearGradient","angle":90,"repeating":true,
            "stops":[{"at":0,"color":{"hex":"#FF0000FF"}},{"at":0.5,"color":{"hex":"#00FF0080"}}]}}],
            "shadows":[{"color":{"hex":"#000000FF"},"offsetX":1,"offsetY":2,"blur":3}]}}"##,
        )
        .unwrap();
        let v3: Paint = serde_json::from_str(
            r#"{"id":"p","name":"p","function":"LINEAR_GRADIENT","angle":90,"repeat":true,
            "stops":[{"at":0,"color":-16776961},{"at":0.5,"color":16711808}],
            "shadows":[{"x_offset":1,"y_offset":2,"radius":3,"color":255}]}"#,
        )
        .unwrap();
        assert_eq!(from_v4(&v4).style().css, v3.style().css);
    }
}
