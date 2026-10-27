// Compiled-in adapter for Tezbar beta fd300e2. Include this as a sibling of v2_sdk.
// MIT; original Raycast Geohash extension by alrico88. See NOTICE.
use super::v2_sdk as sdk;
use crate::native_core::common::{CoreContext, CoreResult};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use tezbar_geohash_native::{decode_geohash, encode_coordinates, Row, COMMANDS};

static NEXT_NONCE: AtomicU64 = AtomicU64::new(1);

pub fn commands() -> &'static [&'static str] {
    if cfg!(any(target_os = "macos", target_os = "windows")) {
        COMMANDS
    } else {
        &[]
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Draft {
    latitude: String,
    longitude: String,
    geohash: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Session {
    command: String,
    nonce: u64,
    revision: u64,
    active: bool,
    draft: Draft,
    // Validated again when rendering/executing; never save arbitrary output or URLs.
    result: Option<Draft>,
    error: Option<String>,
    pending_hide: Option<String>,
}
impl Session {
    fn new(command: &str) -> Result<Self, String> {
        let nonce = NEXT_NONCE
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .map_err(|_| "Geohash session nonce exhausted")?;
        Ok(Self {
            command: command.into(),
            nonce,
            revision: 1,
            active: true,
            draft: Draft::default(),
            result: None,
            error: None,
            pending_hide: None,
        })
    }
    fn bump(&mut self) -> Result<(), String> {
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or("Geohash session revision exhausted")?;
        self.pending_hide = None;
        Ok(())
    }
    fn id(&self, action: &str) -> String {
        format!("geohash:{}:{}:{action}", self.nonce, self.revision)
    }
    fn rows(&self) -> Result<Vec<Row>, String> {
        let input = self
            .result
            .as_ref()
            .ok_or("Geohash result is not available")?;
        calculate(&self.command, input)
    }
}
fn calculate(command: &str, input: &Draft) -> Result<Vec<Row>, String> {
    match command {
        "encode-coordinates" => encode_coordinates(&input.latitude, &input.longitude),
        "decode-geohash" => decode_geohash(&input.geohash),
        _ => Err("Unsupported Geohash command".into()),
    }
}
fn title(command: &str) -> &'static str {
    if command == "encode-coordinates" {
        "Encode as Geohash"
    } else {
        "Decode Geohash"
    }
}
fn action(s: &Session, key: &str, title: &str, kind: &str) -> Value {
    json!({"id":s.id(key),"title":title,"kind":kind})
}
fn render(s: &Session) -> CoreResult {
    if !s.active {
        return Err("Geohash session was disposed".into());
    }
    let mut actions = vec![];
    let root = if s.result.is_none() && s.error.is_none() {
        actions.push(action(
            s,
            "submit",
            if s.command == "encode-coordinates" {
                "Encode Coordinates"
            } else {
                "Decode Geohash"
            },
            "submit-form",
        ));
        let fields = if s.command == "encode-coordinates" {
            vec![
                (
                    "latitude",
                    "Latitude",
                    s.draft.latitude.as_str(),
                    "E.g., 40.4169",
                ),
                (
                    "longitude",
                    "Longitude",
                    s.draft.longitude.as_str(),
                    "E.g., -3.7035",
                ),
            ]
        } else {
            vec![(
                "geohash",
                "Geohash to decode",
                s.draft.geohash.as_str(),
                "E.g., ezmju",
            )]
        };
        let children = fields.into_iter().map(|(id,title,value,placeholder)| {
            let key = format!("field-{id}");
            actions.push(action(s,&key,&format!("Update {title}"),"action"));
            json!({"type":"Form.TextField","props":{"id":id,"title":title,"value":value,"placeholder":placeholder,"actionId":s.id(&key)}})
        }).collect::<Vec<_>>();
        json!({"type":"Form","props":{},"children":children})
    } else {
        let mut sections: Vec<Value> = vec![];
        if let Some(error) = &s.error {
            sections.push(json!({"type":"List.Item","props":{"id":"invalid","title":error,"icon":"ExclamationMark","actionIds":[s.id("back")]}}));
        } else {
            let rows = s.rows()?;
            for (i, row) in rows.iter().enumerate() {
                let copy = format!("copy-{i}");
                actions.push(action(s, &copy, "Copy to Clipboard", "action"));
                let mut ids = vec![s.id(&copy)];
                if row.link.is_some() {
                    let open = format!("open-{i}");
                    actions.push(action(s, &open, "Open in Browser", "action"));
                    ids.push(s.id(&open));
                }
                let node = json!({"type":"List.Item","props":{"id":format!("row-{i}"),"title":row.title,"accessories":[{"text":row.value}],"actionIds":ids}});
                if row.section.is_empty() {
                    sections.push(node);
                } else {
                    if sections
                        .last()
                        .is_none_or(|section| section["props"]["title"] != row.section)
                    {
                        sections.push(json!({"type":"List.Section","props":{"title":row.section},"children":[]}));
                    }
                    sections.last_mut().expect("section created")["children"]
                        .as_array_mut()
                        .expect("array")
                        .push(node);
                }
            }
        }
        actions.push(action(s, "back", "Back", "action"));
        json!({"type":"List","props":{"backActionId":s.id("back"),"__hasServerSearch":false},"children":sections})
    };
    Ok(sdk::view(title(&s.command), root, actions))
}
fn save(state: &mut Value, s: &Session) -> Result<(), String> {
    *state = serde_json::to_value(s).map_err(|e| e.to_string())?;
    Ok(())
}
fn form(payload: &Value) -> Result<Option<&serde_json::Map<String, Value>>, String> {
    for key in ["formValues", "values", "data"] {
        if let Some(v) = payload.get(key) {
            return v
                .as_object()
                .map(Some)
                .ok_or_else(|| format!("{key} must be an object"));
        }
    }
    Ok(None)
}
fn apply_form(s: &mut Session, payload: &Value) -> Result<(), String> {
    let Some(form) = form(payload)? else {
        return Ok(());
    };
    let keys: &[&str] = if s.command == "encode-coordinates" {
        &["latitude", "longitude"]
    } else {
        &["geohash"]
    };
    for key in keys {
        if let Some(value) = form.get(*key) {
            let text = value
                .as_str()
                .ok_or_else(|| format!("{key} must be a string"))?;
            match *key {
                "latitude" => s.draft.latitude = text.into(),
                "longitude" => s.draft.longitude = text.into(),
                _ => s.draft.geohash = text.into(),
            }
        }
    }
    Ok(())
}
// Core errors normally use Err; preserve failures even if a router returns an
// explicit error envelope instead. A failed effect never triggers window hiding.
fn effect_result(value: Value) -> Result<(), String> {
    if value.get("ok") == Some(&json!(false)) || value.get("success") == Some(&json!(false)) {
        return Err(value["error"]
            .as_str()
            .or_else(|| value["message"].as_str())
            .unwrap_or("Host operation failed")
            .to_owned());
    }
    Ok(())
}

pub async fn handle(
    ctx: &CoreContext,
    command: &str,
    operation: &str,
    payload: &Value,
    state: &mut Value,
) -> CoreResult {
    // Validate before state mutation or any host effect, including disposal.
    if !COMMANDS.contains(&command) {
        return Err(format!("Unsupported Geohash command: {command}"));
    }
    if ![
        "extension:run-command",
        "extension:invoke-action",
        "extension:refresh-session",
        "extension:search-text-changed",
        "extension:load-more",
        "extension:dispose-session",
    ]
    .contains(&operation)
    {
        return Err(format!("Unsupported Geohash operation: {operation}"));
    }
    if !state.is_object() {
        return Err("Geohash session state must be an object".into());
    }
    if operation == "extension:run-command" {
        let s = Session::new(command)?;
        let view = render(&s)?;
        save(state, &s)?;
        return Ok(view);
    }
    let mut s: Session =
        serde_json::from_value(state.clone()).map_err(|_| "Geohash session is not initialized")?;
    if s.command != command {
        return Err("Geohash session command mismatch".into());
    }
    if operation == "extension:dispose-session" {
        s.bump()?;
        s.active = false;
        s.draft = Draft::default();
        s.result = None;
        s.error = None;
        save(state, &s)?; // Retain nonce/revision tombstone; never reset counters.
        return Ok(sdk::unchanged());
    }
    if !s.active {
        return Err("Geohash session was disposed".into());
    }
    if operation == "extension:refresh-session" {
        return render(&s);
    }
    if operation == "extension:search-text-changed" || operation == "extension:load-more" {
        return Ok(sdk::unchanged());
    }
    let id = payload["actionId"]
        .as_str()
        .ok_or("Geohash actionId must be a string")?;
    let view = render(&s)?;
    if !view["actions"]
        .as_array()
        .expect("actions")
        .iter()
        .any(|a| a["id"] == id)
    {
        return Err("Unknown or stale Geohash action".into());
    }
    let prefix = format!("geohash:{}:{}:", s.nonce, s.revision);
    let key = id.strip_prefix(&prefix).ok_or("Invalid Geohash action")?;
    match key {
        "submit" => {
            apply_form(&mut s, payload)?;
            s.bump()?;
            match calculate(command, &s.draft) {
                Ok(_) => {
                    s.result = Some(s.draft.clone());
                    s.error = None;
                }
                Err(error) => {
                    s.result = None;
                    s.error = Some(error);
                }
            }
        }
        "back" => {
            s.bump()?;
            s.result = None;
            s.error = None;
        }
        key if key.starts_with("field-") => {
            let value = form(payload)?
                .and_then(|v| v.get("value"))
                .or_else(|| payload.get("value"))
                .and_then(Value::as_str)
                .ok_or("Geohash field value must be a string")?;
            match &key[6..] {
                "latitude" => s.draft.latitude = value.into(),
                "longitude" => s.draft.longitude = value.into(),
                "geohash" => s.draft.geohash = value.into(),
                _ => return Err("Unknown Geohash field".into()),
            }
            // Keep field IDs stable within one form so queued keystrokes remain
            // valid. Submit/back/dispose/relaunch rotate the form generation.
        }
        key => {
            let (kind, index) = key.split_once('-').ok_or("Invalid Geohash result action")?;
            let i = index
                .parse::<usize>()
                .map_err(|_| "Invalid Geohash row index")?;
            let rows = s.rows()?;
            let row = rows.get(i).ok_or("Unknown Geohash row")?;
            // An action's output always comes from validated committed input.
            // Ignore all supplied form values, URLs, text and action metadata.
            if s.pending_hide.as_deref() != Some(id) {
                if s.pending_hide.is_some() {
                    return Err("Retry the pending window close or go Back".into());
                }
                match kind {
                    "copy" => effect_result(
                        sdk::copy(
                            ctx,
                            if command == "encode-coordinates" {
                                &row.title
                            } else {
                                &row.value
                            },
                        )
                        .await?,
                    )?,
                    "open" => {
                        let url = row
                            .link
                            .as_deref()
                            .ok_or("This Geohash row has no browser action")?;
                        // Both constructors above use fixed HTTPS hosts, validated
                        // lowercase geohashes or finite numbers; no public URL RPC.
                        effect_result(sdk::open(ctx, url).await?)?;
                    }
                    _ => return Err("Unknown Geohash result action".into()),
                }
                let close = payload["preferences"]["closeOnAction"]
                    .as_bool()
                    .unwrap_or(true);
                if !close {
                    return Ok(sdk::unchanged());
                }
                s.pending_hide = Some(id.into());
                save(state, &s)?; // A hide retry must not repeat a successful copy/open.
            }
            ctx.request("window:hide", vec![])
                .await
                .and_then(effect_result)
                .map_err(|e| {
                    format!("Action succeeded, but closing the window failed; retry to close: {e}")
                })?;
            s.bump()?;
            s.result = None;
            s.error = None;
            s.draft = Draft::default();
        }
    }
    let view = render(&s)?;
    save(state, &s)?;
    Ok(view)
}
