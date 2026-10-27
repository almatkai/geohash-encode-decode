// Compile byte-exact beta common.rs + v2_sdk.rs, with only the desktop Tauri
// surface stubbed. Their own tests also run. The production adapter is unchanged.
#![allow(dead_code)]
extern crate self as tauri;
#[derive(Clone)]
pub struct AppHandle;
pub trait Emitter {
    fn emit<S: serde::Serialize + Clone>(&self, channel: &str, payload: S) -> Result<(), String>;
}
impl Emitter for AppHandle {
    fn emit<S: serde::Serialize + Clone>(&self, _: &str, _: S) -> Result<(), String> {
        panic!("No desktop event/OS calls are permitted in this harness")
    }
}
#[path = "fixtures/beta/common.rs"]
pub mod common;
#[path = "fixtures/beta/v2_sdk.rs"]
pub mod v2_sdk;
pub mod native_core {
    pub use crate::common;
    pub mod extensions {
        pub use crate::v2_sdk;
        pub mod geohash {
            include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tezbar_adapter.rs"));
        }
    }
}
use common::{CoreContext, CorePaths};
use native_core::extensions::geohash::handle;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
type Calls = Arc<Mutex<Vec<(String, Vec<Value>)>>>;
fn context() -> (tempfile::TempDir, CoreContext, Calls) {
    let dir = tempfile::tempdir().unwrap();
    let ctx = CoreContext::headless(CorePaths {
        data_dir: dir.path().join("data"),
        config_dir: dir.path().join("config"),
        resource_dir: dir.path().join("resources"),
        temp_dir: dir.path().join("tmp"),
        home_dir: dir.path().into(),
    });
    let calls: Calls = Arc::default();
    let log = calls.clone();
    ctx.set_router(Arc::new(move |channel, args| {
        log.lock().unwrap().push((channel.clone(), args));
        Box::pin(async move {
            assert!(["shell:open", "clipboard:write", "window:hide"].contains(&channel.as_str()));
            Ok(Value::Null)
        })
    }))
    .unwrap();
    (dir, ctx, calls)
}
fn id(view: &Value, suffix: &str) -> String {
    view["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"].as_str().unwrap().ends_with(&format!(":{suffix}")))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .into()
}
async fn call(
    ctx: &CoreContext,
    cmd: &str,
    state: &mut Value,
    id: &str,
    values: Value,
    close: bool,
) -> common::CoreResult {
    handle(
        ctx,
        cmd,
        "extension:invoke-action",
        &json!({"actionId":id,"formValues":values,"preferences":{"closeOnAction":close}}),
        state,
    )
    .await
}
async fn launch(ctx: &CoreContext, cmd: &str, state: &mut Value) -> Value {
    handle(ctx, cmd, "extension:run-command", &json!({}), state)
        .await
        .unwrap()
}
async fn results(ctx: &CoreContext, cmd: &str, state: &mut Value) -> Value {
    let view = launch(ctx, cmd, state).await;
    let values = if cmd == "encode-coordinates" {
        json!({"latitude":"40.4169","longitude":"-3.7035"})
    } else {
        json!({"geohash":"ezmju"})
    };
    call(ctx, cmd, state, &id(&view, "submit"), values, false)
        .await
        .unwrap()
}
#[tokio::test]
async fn controlled_drafts_submit_back_refresh_invalid_retry_and_stale_actions() {
    let (_dir, ctx, calls) = context();
    let mut s = json!({});
    let cmd = "encode-coordinates";
    let first = launch(&ctx, cmd, &mut s).await;
    assert_eq!(first["root"]["type"], "Form");
    assert_eq!(first["root"]["children"][0]["props"]["value"], "");
    let original_field = id(&first, "field-latitude");
    let view = call(
        &ctx,
        cmd,
        &mut s,
        &id(&first, "field-latitude"),
        json!({"value":"123.456"}),
        false,
    )
    .await
    .unwrap();
    // Queued field events use the same form generation.
    assert_eq!(id(&view, "field-latitude"), original_field);
    let view = handle(
        &ctx,
        cmd,
        "extension:invoke-action",
        &json!({"actionId":id(&view,"field-longitude"),"value":"45"}),
        &mut s,
    )
    .await
    .unwrap();
    let refresh = handle(&ctx, cmd, "extension:refresh-session", &json!({}), &mut s)
        .await
        .unwrap();
    assert_eq!(refresh, view);
    assert_eq!(refresh["root"]["children"][0]["props"]["value"], "123.456");
    let view = call(&ctx, cmd, &mut s, &id(&view, "submit"), json!({}), false)
        .await
        .unwrap();
    assert_eq!(view["root"]["children"].as_array().unwrap().len(), 12);
    let oldcopy = id(&view, "copy-0");
    let back = call(&ctx, cmd, &mut s, &id(&view, "back"), json!({}), false)
        .await
        .unwrap();
    assert!(call(
        &ctx,
        cmd,
        &mut s,
        &original_field,
        json!({"value":"999"}),
        false
    )
    .await
    .is_err());
    assert_eq!(back["root"]["children"][0]["props"]["value"], "123.456");
    assert!(call(&ctx, cmd, &mut s, &oldcopy, json!({}), false)
        .await
        .is_err());
    let invalid = call(
        &ctx,
        cmd,
        &mut s,
        &id(&back, "submit"),
        json!({"latitude":"bad"}),
        false,
    )
    .await
    .unwrap();
    assert_eq!(
        invalid["root"]["children"][0]["props"]["title"],
        "Invalid coordinates entered"
    );
    assert_eq!(invalid["actions"].as_array().unwrap().len(), 1);
    let back = call(&ctx, cmd, &mut s, &id(&invalid, "back"), json!({}), false)
        .await
        .unwrap();
    assert_eq!(back["root"]["children"][0]["props"]["value"], "bad");
    let valid = handle(
        &ctx,
        cmd,
        "extension:invoke-action",
        &json!({"actionId":id(&back,"submit"),"values":{"latitude":"0","longitude":"0"}}),
        &mut s,
    )
    .await
    .unwrap();
    assert_eq!(valid["root"]["children"][0]["props"]["title"], "7");
    assert!(calls.lock().unwrap().is_empty());
}
#[tokio::test]
async fn routes_each_original_decoded_value_and_only_constructed_links() {
    let (_dir, ctx, calls) = context();
    let cmd = "decode-geohash";
    let mut s = json!({});
    let view = results(&ctx, cmd, &mut s).await;
    assert_eq!(view["root"]["children"].as_array().unwrap().len(), 3);
    let rows = tezbar_geohash_native::decode_geohash("ezmju").unwrap();
    for (i, row) in rows.iter().enumerate() {
        call(
            &ctx,
            cmd,
            &mut s,
            &id(&view, &format!("copy-{i}")),
            json!({"text":"forged","url":"https://evil.test","geohash":"zz"}),
            false,
        )
        .await
        .unwrap();
        assert_eq!(
            calls.lock().unwrap().last().unwrap(),
            &("clipboard:write".into(), vec![json!(row.value)])
        );
        if let Some(link) = &row.link {
            call(
                &ctx,
                cmd,
                &mut s,
                &id(&view, &format!("open-{i}")),
                json!({"url":"file:///etc/passwd"}),
                false,
            )
            .await
            .unwrap();
            assert_eq!(
                calls.lock().unwrap().last().unwrap(),
                &("shell:open".into(), vec![json!(link)])
            );
        }
    }
    assert!(!calls
        .lock()
        .unwrap()
        .iter()
        .any(|(c, _)| c == "window:hide"));
    let before = calls.lock().unwrap().len();
    for suffix in [
        "copy-99",
        "open-1",
        "open-2",
        "open-3",
        "open-4",
        "copy-00",
        "open-https://evil.test",
        "submit",
        "field-geohash",
    ] {
        let fabricated = format!("{}:{suffix}", id(&view, "back").rsplit_once(':').unwrap().0);
        assert!(
            call(&ctx, cmd, &mut s, &fabricated, json!({}), false)
                .await
                .is_err(),
            "{suffix}"
        );
    }
    assert_eq!(calls.lock().unwrap().len(), before);
    let refresh = handle(&ctx, cmd, "extension:refresh-session", &json!({}), &mut s)
        .await
        .unwrap();
    assert_eq!(refresh, view);
}
#[tokio::test]
async fn default_close_hides_then_resets_and_does_not_revive_old_actions() {
    let (dir, ctx, calls) = context();
    let cmd = "encode-coordinates";
    let mut s = json!({});
    let view = results(&ctx, cmd, &mut s).await;
    let copy = id(&view, "copy-11");
    let out = handle(
        &ctx,
        cmd,
        "extension:invoke-action",
        &json!({"actionId":copy}),
        &mut s,
    )
    .await
    .unwrap();
    assert_eq!(out["root"]["type"], "Form");
    assert_eq!(out["root"]["children"][0]["props"]["value"], "");
    assert_eq!(
        *calls.lock().unwrap(),
        vec![
            ("clipboard:write".into(), vec![json!("ezjmgtxjw0np")]),
            ("window:hide".into(), vec![])
        ]
    );
    assert!(call(&ctx, cmd, &mut s, &copy, json!({}), false)
        .await
        .is_err());
    let new = results(&ctx, cmd, &mut s).await;
    let old = id(&new, "copy-0");
    handle(&ctx, cmd, "extension:dispose-session", &json!({}), &mut s)
        .await
        .unwrap();
    assert_eq!(s["active"], false);
    assert!(s["revision"].as_u64().unwrap() > 1);
    assert!(call(&ctx, cmd, &mut s, &old, json!({}), false)
        .await
        .is_err());
    results(&ctx, cmd, &mut s).await;
    assert!(call(&ctx, cmd, &mut s, &old, json!({}), false)
        .await
        .is_err());
    // No extension storage, profile edits, clipboard or browser OS effects.
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}
#[tokio::test]
async fn validates_command_operation_and_form_shapes_before_mutation_or_effects() {
    let (_dir, ctx, calls) = context();
    let cmd = "decode-geohash";
    let mut s = json!({});
    let view = launch(&ctx, cmd, &mut s).await;
    let before = s.clone();
    for (command, op, payload) in [
        ("bad", "extension:run-command", json!({})),
        ("bad", "extension:dispose-session", json!({})),
        (
            "encode-coordinates",
            "extension:invoke-action",
            json!({"actionId":id(&view,"submit")}),
        ),
        (cmd, "invented:open", json!({"url":"https://evil.test"})),
        (
            cmd,
            "extension:invoke-action",
            json!({"actionId":id(&view,"submit"),"formValues":[]}),
        ),
        (
            cmd,
            "extension:invoke-action",
            json!({"actionId":id(&view,"submit"),"formValues":{"geohash":3}}),
        ),
        (
            cmd,
            "extension:invoke-action",
            json!({"actionId":id(&view,"field-geohash"),"formValues":{"value":false}}),
        ),
        (cmd, "extension:invoke-action", json!({"actionId":false})),
    ] {
        assert!(handle(&ctx, command, op, &payload, &mut s).await.is_err());
        assert_eq!(s, before);
    }
    for op in ["extension:load-more", "extension:search-text-changed"] {
        assert_eq!(
            handle(
                &ctx,
                cmd,
                op,
                &json!({"searchText":"https://evil.test"}),
                &mut s
            )
            .await
            .unwrap()["mode"],
            "unchanged"
        );
        assert_eq!(s, before);
    }
    let invalid = handle(
        &ctx,
        cmd,
        "extension:invoke-action",
        &json!({"actionId":id(&view,"submit"),"data":{"geohash":"EZMJU"}}),
        &mut s,
    )
    .await
    .unwrap();
    assert_eq!(
        invalid["root"]["children"][0]["props"]["title"],
        "The entered geohash contains invalid characters"
    );
    assert!(calls.lock().unwrap().is_empty());
}
#[tokio::test]
async fn effect_errors_and_error_envelopes_preserve_retry_and_hide_never_repeats_copy() {
    for envelope in [false, true] {
        let (_dir, ctx, _) = context();
        // A fresh real CoreContext uses a programmable stub router.
        let ctx = CoreContext::headless(ctx.paths.clone());
        let log: Calls = Arc::default();
        let calls = log.clone();
        ctx.set_router(Arc::new(move |c, a| {
            let mut log = log.lock().unwrap();
            log.push((c.clone(), a));
            let n = log.len();
            Box::pin(async move {
                if n == 1 || (c == "window:hide" && n == 3) {
                    if envelope {
                        Ok(json!({"ok":false,"error":"stub failure"}))
                    } else {
                        Err("stub failure".into())
                    }
                } else {
                    Ok(Value::Null)
                }
            })
        }))
        .unwrap();
        let cmd = "decode-geohash";
        let mut s = json!({});
        let view = results(&ctx, cmd, &mut s).await;
        let copy = id(&view, "copy-0");
        let before = s.clone();
        assert!(call(&ctx, cmd, &mut s, &copy, json!({}), true)
            .await
            .is_err());
        assert_eq!(s, before);
        assert_eq!(calls.lock().unwrap().len(), 1);
        assert!(call(&ctx, cmd, &mut s, &copy, json!({}), true)
            .await
            .is_err()); // copy succeeds, hide fails
        assert!(s["pending_hide"].is_string());
        let out = call(&ctx, cmd, &mut s, &copy, json!({}), true)
            .await
            .unwrap(); // only hide retries
        assert_eq!(out["root"]["type"], "Form");
        assert_eq!(
            calls
                .lock()
                .unwrap()
                .iter()
                .map(|v| v.0.as_str())
                .collect::<Vec<_>>(),
            [
                "clipboard:write",
                "clipboard:write",
                "window:hide",
                "window:hide"
            ]
        );
    }
}
#[tokio::test]
async fn browser_default_close_order_and_failed_browser_retry() {
    let (_dir, ctx, _) = context();
    let ctx = CoreContext::headless(ctx.paths.clone());
    let calls: Calls = Arc::default();
    let log = calls.clone();
    ctx.set_router(Arc::new(move |c, a| {
        let mut l = log.lock().unwrap();
        l.push((c, a));
        let n = l.len();
        Box::pin(async move {
            if n == 1 {
                Err("browser unavailable".into())
            } else {
                Ok(json!({"ok":true}))
            }
        })
    }))
    .unwrap();
    let cmd = "decode-geohash";
    let mut s = json!({});
    let view = results(&ctx, cmd, &mut s).await;
    let open = id(&view, "open-5");
    assert!(call(&ctx, cmd, &mut s, &open, json!({}), true)
        .await
        .is_err());
    assert_eq!(calls.lock().unwrap().len(), 1);
    call(&ctx, cmd, &mut s, &open, json!({}), true)
        .await
        .unwrap();
    assert_eq!(
        calls
            .lock()
            .unwrap()
            .iter()
            .map(|v| v.0.as_str())
            .collect::<Vec<_>>(),
        ["shell:open", "shell:open", "window:hide"]
    );
}

#[tokio::test]
async fn renderer_wire_contract_samples() {
    let (_dir, ctx, _) = context();
    let mut samples = vec![];
    for cmd in tezbar_geohash_native::COMMANDS {
        let mut s = json!({});
        let form = launch(&ctx, cmd, &mut s).await;
        let field = if *cmd == "encode-coordinates" {
            "latitude"
        } else {
            "geohash"
        };
        let value = if *cmd == "encode-coordinates" {
            "40.4169"
        } else {
            "ezmju"
        };
        let changed = call(
            &ctx,
            cmd,
            &mut s,
            &id(&form, &format!("field-{field}")),
            json!({"value":value}),
            false,
        )
        .await
        .unwrap();
        let rows = results(&ctx, cmd, &mut s).await;
        let back = call(&ctx, cmd, &mut s, &id(&rows, "back"), json!({}), false)
            .await
            .unwrap();
        assert_eq!(back["root"]["children"][0]["props"]["value"], value);
        let first = if *cmd == "encode-coordinates" {
            &rows["root"]["children"][0]
        } else {
            &rows["root"]["children"][0]["children"][0]
        };
        assert!(first["props"]["accessories"][0]["text"].is_string());
        samples
            .push(json!({"command":cmd,"form":form,"changed":changed,"results":rows,"back":back}));
    }
    if let Some(path) = std::env::var_os("TEZBAR_GEOHASH_RENDERER_FIXTURE") {
        std::fs::write(path, serde_json::to_vec_pretty(&samples).unwrap()).unwrap();
    }
}

#[tokio::test]
async fn result_actions_use_committed_inputs_even_when_drafts_change() {
    let (_dir, ctx, calls) = context();
    let cmd = "decode-geohash";
    let mut s = json!({});
    let view = results(&ctx, cmd, &mut s).await;
    // Drafts belong to the editable form, never to an already-issued result.
    s["draft"]["geohash"] = json!("https://evil.test");
    call(
        &ctx,
        cmd,
        &mut s,
        &id(&view, "copy-0"),
        json!({"geohash":"zz","text":"forged"}),
        false,
    )
    .await
    .unwrap();
    call(
        &ctx,
        cmd,
        &mut s,
        &id(&view, "open-0"),
        json!({"url":"file:///etc/passwd"}),
        false,
    )
    .await
    .unwrap();
    assert_eq!(
        *calls.lock().unwrap(),
        vec![
            ("clipboard:write".into(), vec![json!("ezmju")]),
            (
                "shell:open".into(),
                vec![json!("https://www.geohash.es/decode?geohash=ezmju")]
            )
        ]
    );
    // Even an internally corrupted committed input cannot become an arbitrary URL.
    s["result"]["geohash"] = json!("https://evil.test");
    assert!(
        call(&ctx, cmd, &mut s, &id(&view, "open-0"), json!({}), false)
            .await
            .is_err()
    );
    assert_eq!(calls.lock().unwrap().len(), 2);
}
