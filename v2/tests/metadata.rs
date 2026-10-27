use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;
const ROOT: &str = env!("CARGO_MANIFEST_DIR");

#[test]
fn original_icon_and_byte_exact_pinned_beta_and_source_fixtures() {
    let icon = include_bytes!("../command-icon.png");
    assert_eq!(icon.len(), 172526);
    assert_eq!(
        format!("{:x}", Sha256::digest(icon)),
        "f59d02918ee4879a95940e3df317ff70ebeb901582967188b034b73daf0ee834"
    );
    for name in ["beta", "upstream"] {
        let base = Path::new(ROOT).join("tests/fixtures").join(name);
        let data: Value =
            serde_json::from_slice(&std::fs::read(base.join("provenance.json")).unwrap()).unwrap();
        let rev = if name == "beta" {
            "fd300e276356587726566026f072563eb9c34cb1"
        } else {
            "6734d85992290066fd31a7f5bee3cca3d3545a1a"
        };
        assert_eq!(data["commit"], rev);
        for file in data["files"].as_array().unwrap() {
            let relative = file["file"].as_str().unwrap_or_else(|| {
                file["path"]
                    .as_str()
                    .unwrap()
                    .strip_prefix("extensions/geohash-encode-decode/")
                    .unwrap_or("LICENSE")
            });
            let bytes = std::fs::read(base.join(relative)).unwrap();
            assert_eq!(
                bytes.len() as u64,
                file["bytes"].as_u64().unwrap(),
                "{name}/{relative}"
            );
            assert_eq!(
                format!("{:x}", Sha256::digest(&bytes)),
                file["sha256"].as_str().unwrap(),
                "{name}/{relative}"
            );
        }
    }
}
#[test]
fn manifest_metadata_and_intended_coverage_match_the_adapter() {
    let v: Value = serde_json::from_str(include_str!("../manifest.json")).unwrap();
    assert_eq!(v["schemaVersion"], 2);
    assert_eq!(v["id"], "com.tezbar.native.geohash-encode-decode");
    assert_eq!(v["replaces"], json!(["raycast.geohash-encode-decode"]));
    assert_eq!(v["icon"], "/extensions/icons/geohash-encode-decode.png");
    assert_eq!(v["runtime"], "rust");
    assert!(v.get("author").is_none());
    assert!(v["description"].as_str().unwrap().contains("alrico88"));
    assert_eq!(v["platforms"], json!(["macOS", "Windows"]));
    let commands = v["commands"].as_array().unwrap();
    assert_eq!(commands.len(), tezbar_geohash_native::COMMANDS.len());
    for (cmd, expected) in commands.iter().zip(tezbar_geohash_native::COMMANDS) {
        assert_eq!(cmd["name"], *expected);
        assert_eq!(cmd["runtime"], "rust");
        assert_eq!(cmd["platforms"], v["platforms"]);
    }
    assert_eq!(v["preferences"][0]["name"], "closeOnAction");
    assert_eq!(v["preferences"][0]["default"], true);
    for (record, platform) in v["verification"]
        .as_array()
        .unwrap()
        .iter()
        .zip(["macOS", "Windows"])
    {
        assert_eq!(record["state"], "not-run");
        assert_eq!(record["platform"], platform);
    }
}
