use serde_json::Value;
use tezbar_geohash_native::{decode_geohash, encode_coordinates};

#[test]
fn exact_original_tsx_and_pinned_dependencies_oracle() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/oracle.json")).unwrap();
    for v in fixture["encode"].as_array().unwrap() {
        compare(
            encode_coordinates(
                v["latitude"].as_str().unwrap(),
                v["longitude"].as_str().unwrap(),
            ),
            v,
        );
    }
    for v in fixture["decode"].as_array().unwrap() {
        compare(decode_geohash(v["geohash"].as_str().unwrap()), v);
    }
}
fn compare(actual: Result<Vec<tezbar_geohash_native::Row>, String>, expected: &Value) {
    if let Some(error) = expected.get("error") {
        assert_eq!(actual.unwrap_err(), error.as_str().unwrap(), "{expected}");
    } else {
        assert_eq!(
            serde_json::to_value(actual.unwrap()).unwrap(),
            expected["rows"],
            "{expected}"
        );
    }
}
