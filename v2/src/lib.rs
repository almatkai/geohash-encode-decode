//! Geohash calculations compatible with the original Raycast extension.
//! MIT port of ngeohash 0.6.3; see NOTICE and licenses/.
use serde::{Deserialize, Serialize};

const ALPHABET: &[u8] = b"0123456789bcdefghjkmnpqrstuvwxyz";
pub const COMMANDS: &[&str] = &["encode-coordinates", "decode-geohash"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Row {
    pub section: String,
    pub title: String,
    pub value: String,
    pub link: Option<String>,
}

fn whitespace(c: char) -> bool {
    matches!(c, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}' |
        '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' |
        '\u{205f}' | '\u{3000}' | '\u{feff}')
}

/// @sindresorhus/is.numericString followed by Number and [-180,180].
/// The latitude uses the same bounds as longitude in the source.
pub fn coordinate(input: &str) -> Result<f64, String> {
    let s = input.trim_matches(whitespace);
    let invalid = || "Invalid coordinates entered".to_owned();
    if s.is_empty() {
        return Err(invalid());
    }
    let radix = [
        ("0x", 16),
        ("0X", 16),
        ("0o", 8),
        ("0O", 8),
        ("0b", 2),
        ("0B", 2),
    ]
    .into_iter()
    .find_map(|(p, r)| s.strip_prefix(p).map(|v| (v, r)));
    let value = if let Some((digits, radix)) = radix {
        if digits.is_empty() {
            return Err(invalid());
        }
        // Any integer over 180 is rejected, so no large-integer rounding is needed.
        let mut n = 0u32;
        for c in digits.chars() {
            let d = c
                .to_digit(radix)
                .filter(|_| c.is_ascii())
                .ok_or_else(invalid)?;
            n = n.saturating_mul(radix).saturating_add(d);
        }
        f64::from(n)
    } else {
        // Rust accepts spellings such as "inf" that ECMAScript Number does not.
        // Validate the ECMAScript decimal grammar before parsing binary64.
        let bytes = s.as_bytes();
        let mut i = usize::from(matches!(bytes.first(), Some(b'+' | b'-')));
        let start = i;
        while bytes.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        let mut count = i - start;
        if bytes.get(i) == Some(&b'.') {
            i += 1;
            let start = i;
            while bytes.get(i).is_some_and(u8::is_ascii_digit) {
                i += 1;
            }
            count += i - start;
        }
        if count == 0 {
            return Err(invalid());
        }
        if matches!(bytes.get(i), Some(b'e' | b'E')) {
            i += 1;
            if matches!(bytes.get(i), Some(b'+' | b'-')) {
                i += 1;
            }
            let start = i;
            while bytes.get(i).is_some_and(u8::is_ascii_digit) {
                i += 1;
            }
            if start == i {
                return Err(invalid());
            }
        }
        if i != bytes.len() {
            return Err(invalid());
        }
        s.parse::<f64>().map_err(|_| invalid())?
    };
    if !value.is_finite() || !(-180.0..=180.0).contains(&value) {
        return Err(invalid());
    }
    Ok(value)
}

// Strict > midpoint, longitude first. Out-of-range latitudes saturate naturally
// under binary subdivision, exactly as ngeohash; do not normalize or wrap them.
fn encode(lat: f64, lon: f64, length: usize) -> String {
    let mut bounds: [[f64; 2]; 2] = [[-180.0, 180.0], [-90.0, 90.0]];
    let values = [lon, lat];
    let mut axis = 0;
    let mut result = String::with_capacity(length);
    for _ in 0..length {
        let mut code = 0;
        for _ in 0..5 {
            let b = &mut bounds[axis];
            let mid = (b[0] + b[1]) / 2.0;
            let upper = values[axis] > mid;
            code = (code << 1) + usize::from(upper);
            b[usize::from(!upper)] = mid;
            axis = 1 - axis;
        }
        result.push(ALPHABET[code] as char);
    }
    result
}

pub fn shape_link(hash: &str) -> String {
    format!("https://www.geohash.es/decode?geohash={hash}")
}

pub fn encode_coordinates(latitude: &str, longitude: &str) -> Result<Vec<Row>, String> {
    let lat = coordinate(latitude)?;
    let lon = coordinate(longitude)?;
    Ok((1..=12)
        .map(|n| {
            let hash = encode(lat, lon, n);
            Row {
                section: String::new(),
                title: hash.clone(),
                value: format!("Precision {n}"),
                link: Some(shape_link(&hash)),
            }
        })
        .collect())
}

/// ECMAScript Number::toString, also used by JSON.stringify for finite numbers.
pub fn js_number(n: f64) -> String {
    ryu_js::Buffer::new().format(n).to_owned()
}

pub fn decode_geohash(hash: &str) -> Result<Vec<Row>, String> {
    if hash.is_empty() || !hash.bytes().all(|b| ALPHABET.contains(&b)) {
        return Err("The entered geohash contains invalid characters".into());
    }
    // No artificial precision cap: long hashes retain the source's binary64
    // collapse and neighbor behavior. Runtime and output space are linear.
    let mut bounds: [[f64; 2]; 2] = [[-180.0, 180.0], [-90.0, 90.0]];
    let mut axis = 0;
    for byte in hash.bytes() {
        let code = ALPHABET
            .iter()
            .position(|b| *b == byte)
            .expect("validated alphabet");
        for bit in (0..5).rev() {
            let b = &mut bounds[axis];
            let mid = (b[0] + b[1]) / 2.0;
            b[usize::from((code >> bit) & 1 == 0)] = mid;
            axis = 1 - axis;
        }
    }
    let [[west, east], [south, north]] = bounds;
    let lat = (south + north) / 2.0;
    let lon = (west + east) / 2.0;
    let lat_err = (north - lat) * 2.0;
    let lon_err = (east - lon) * 2.0;
    let neighbors = [
        (1., 0.),
        (1., 1.),
        (0., 1.),
        (-1., 1.),
        (-1., 0.),
        (-1., -1.),
        (0., -1.),
        (1., -1.),
    ]
    .map(|(dy, dx)| {
        let y = (lat + dy * lat_err).clamp(-90., 90.);
        let x = lon + dx * lon_err;
        let x = if x > 180. {
            -180. + x % 180.
        } else if x < -180. {
            180. + x % 180.
        } else {
            x
        };
        encode(y, x, hash.len())
    })
    .join(",");
    let ring = [
        [west, south],
        [east, south],
        [east, north],
        [west, north],
        [west, south],
    ];
    let pairs = ring.map(|[x, y]| [js_number(x), js_number(y)]);
    let json_ring = pairs
        .iter()
        .map(|p| format!("[{},{}]", p[0], p[1]))
        .collect::<Vec<_>>()
        .join(",");
    let wkt_ring = pairs
        .iter()
        .map(|p| format!("{} {}", p[0], p[1]))
        .collect::<Vec<_>>()
        .join(", ");
    let shape = shape_link(hash);
    let point = format!("POINT ({} {})", js_number(lon), js_number(lat));
    let maps = format!(
        "https://www.google.com/maps/search/?api=1&query={},{}",
        js_number(lat),
        js_number(lon)
    );
    Ok([
        ("Base Info", "Geohash", hash.to_owned(), Some(shape.clone())),
        ("Base Info", "Precision", hash.len().to_string(), None),
        ("Base Info", "Neighbors", neighbors, None),
        ("Position", "Latitude", js_number(lat), None),
        ("Position", "Longitude", js_number(lon), None),
        ("Position", "Centroid (WKT)", point, Some(maps)),
        (
            "Shape",
            "Polygon shape (GeoJSON)",
            format!("{{\"type\":\"Polygon\",\"coordinates\":[[{json_ring}]]}}"),
            Some(shape.clone()),
        ),
        (
            "Shape",
            "Polygon shape (WKT)",
            format!("POLYGON (({wkt_ring}))"),
            Some(shape),
        ),
    ]
    .into_iter()
    .map(|(s, t, v, l)| Row {
        section: s.into(),
        title: t.into(),
        value: v,
        link: l,
    })
    .collect())
}
