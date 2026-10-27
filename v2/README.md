# Geohash · Rust Native V2

This is a standalone Rust port of **Geohash by alrico88**, alongside the retained
V1 extension in the repository root. It implements `encode-coordinates` and
`decode-geohash` without a JavaScript runtime, subprocess, network request or
persistent extension store. The original icon is retained byte for byte.

The canonical package ID is `com.tezbar.native.geohash-encode-decode`; its schema 2
manifest replaces `raycast.geohash-encode-decode`. V1 remains available as fallback.
**A host registration and native app rebuild are required.** Installing JSON or
copying the folder into a profile cannot add compiled Rust dispatch.

## Behavior and compatibility

- Encode uses the original text fields and produces all 12 precisions. Copy copies
  the hash; Open in Browser opens its `https://www.geohash.es/decode?geohash=…` URL.
- Decode accepts nonempty lowercase `0123456789bcdefghjkmnpqrstuvwxyz` only. It
  preserves all eight source rows, their order and sections: Base Info (Geohash,
  Precision, Neighbors), Position (Latitude, Longitude, Centroid (WKT)), Shape
  (Polygon shape (GeoJSON), Polygon shape (WKT)). Every value can be copied.
  Geohash and polygon rows link to geohash.es; the centroid links to Google Maps.
- Coordinate validation intentionally accepts **both** latitude and longitude in
  `[-180, 180]`, matching `is.numericString` + `Number` in the original. Decimal,
  exponent, unsigned hexadecimal/binary/octal and ECMAScript whitespace work;
  empty/whitespace-only strings, NaN, infinities and out-of-range values do not.
  `ngeohash` subdivision saturates latitudes outside ±90 at the corresponding
  pole, rather than wrapping or rejecting them. Midpoint ties take the lower cell.
- Neighbors are comma-separated with no spaces, clockwise from north, exactly as
  `neighbors(hash).toString()`. Pole clamping, dateline wrapping, polygon ring order
  (SW, SE, NE, NW, SW), WKT punctuation and ECMAScript binary64 number formatting
  match the pinned libraries, including negative zero and small scientific values.
- There is **no artificial decode length limit**. Long hashes preserve the source's
  binary64 precision collapse, degenerate polygons and repeated/rounded neighbors.
  Work and output memory grow linearly with hash length. Precision is character
  count, not a claim of additional geographic accuracy beyond binary64 resolution.
- Invalid submissions show the original error text and a Back action. Back restores
  drafts; users can correct and resubmit. Controlled fields use `props.value` and
  actual beta `formValues.value` callbacks. Field IDs stay stable within a form so
  queued keystrokes work; submit, Back, successful close, dispose and relaunch
  invalidate the prior screen's actions. Refresh does not erase drafts or revive IDs.
- `closeOnAction` defaults to true. A successful copy/open is followed by the real
  `window:hide` route; the session resets to its empty form. Beta exposes no real
  `popToRoot` API: resetting this command's form is the navigation adaptation,
  not a claim to reset the host's navigation stack. False keeps results interactive.
  Clipboard/browser errors are returned and remain retryable. If hiding fails after
  a successful effect, retrying that same action retries **only hiding**, avoiding
  duplicate effects; Back cancels the pending close and restores the draft.

Actions are accepted only when issued for the current command/screen. Nonces are
process-wide monotonic, revisions are checked, and disposal retains a tombstone.
Result values and URLs are recomputed from separately committed, revalidated input;
copy/open never trusts payload text, URL, drafts or stored arbitrary output. Only
constructed HTTPS geohash.es / Google Maps targets can reach `sdk::open`. The
adapter accepts beta `extension:invoke-action` payloads (`formValues`), plus the
`values` / `data` object form variants and draft-only submission. Unsupported
commands/operations and malformed values fail before any effects or state changes.
Search and load-more are client-side/no-ops for these finite lists.

## Build and verify

From this directory (Rust **1.85.0 or later**):

```sh
cargo test --locked --all-targets
cargo +1.85.0 test --locked --all-targets
cargo fmt --all -- --check
rustfmt --edition 2021 --check tezbar_adapter.rs
cargo clippy --locked --all-targets -- -D warnings
cargo check --locked --all-targets --target x86_64-pc-windows-gnu
cargo clippy --locked --all-targets --target x86_64-pc-windows-gnu -- -D warnings
```

Tests include 525 vectors (231 encode, 294 decode), not Rust-generated expected
values: the oracle executes the **pinned original TSX components** with pure React
and Raycast UI mocks and the real locked JS libraries. It captures rendered rows,
copy contents and browser links, including invalid inputs, all precisions, ties,
poles/dateline, latitudes 90–180, numeric string spellings, Unicode whitespace,
scientific values and hashes over 1,000 characters. Seeded vectors also cover
ordinary points and 1–65-character hashes. Rust compares every string exactly.

`tests/contract.rs` compiles the production adapter **unchanged** beside byte-exact
beta `common.rs` and `v2_sdk.rs`. Only Tauri's desktop surface is stubbed; the real
CoreContext router, CoreResult, SDK and their upstream tests run. Adapter tests cover
real form and action routing, drafts, validation, Back, refresh, default/false close
preferences, both Err and error envelopes, failure/retry ordering, stale/fabricated
actions, nonce/revision tombstones and no profile/store effects. All clipboard,
browser and window operations are stubs. The manifest/icon and source/API fixture
hashes are checked by Cargo tests, including in an extracted crate package.

Optional full JSON Schema and renderer/oracle regeneration checks:

```sh
# Install schema tooling into ignored local build output, not the system Python.
python3 -m pip install --no-cache-dir --target target/schema-tools jsonschema==4.23.0
PYTHONPATH=target/schema-tools python3 scripts/validate-manifest.py
npm ci --ignore-scripts --prefix oracle
node --import ./oracle/node_modules/tsx/dist/loader.mjs oracle/generate.cjs
node oracle/check-renderer.cjs
```

The renderer check executes actual AST-selected functions from the pinned beta
form/list/navigation renderer against views emitted by the Rust adapter. It verifies
controlled drafts, submission IDs, row accessories, sections, action references and
Back wiring. Beta reads `accessories: [{text: …}]`, so the adapter maps V1's
`accessoryTitle` to that supported wire representation.

To verify distributability without depending on the surrounding V1 checkout:

```sh
# New uncommitted v2/ files require --allow-dirty during implementation only.
cargo package --locked --allow-dirty
mkdir -p target/package-review
# Extract into a fresh subdirectory if this version was already extracted.
tar -xzf target/package/tezbar-geohash-native-0.2.0.crate -C target/package-review
cargo test --locked --all-targets --manifest-path target/package-review/tezbar-geohash-native-0.2.0/Cargo.toml
```

No normal Cargo test or runtime execution needs Node, Python, a Tezbar checkout,
network access after dependency download, or an active desktop session.

## Vendor into Tezbar beta

The adapter targets **fd300e276356587726566026f072563eb9c34cb1**. Read that revision's
`docs/NATIVE_V2_FORMAT.md` and `schemas/extension-v2.schema.json` before integration.
Perform these changes only in a separately authorized host checkout:

1. Put this crate's complete distributable contents at
   **`src-tauri/native-extensions/geohash-encode-decode/`**. Keep Cargo.lock, notices,
   tests and fixtures. Omit build output and oracle node_modules. This is a stable
   vendored path; do not depend on this development clone or a temporary directory.
2. Add this dependency to the host `src-tauri/Cargo.toml`:

   ```toml
   tezbar-geohash-native = { path = "native-extensions/geohash-encode-decode" }
   ```

3. Copy the **unchanged** `tezbar_adapter.rs` to
   `src-tauri/src/native_core/extensions/v2_geohash.rs`. Add `mod v2_geohash;` in
   `src-tauri/src/native_core/extensions/mod.rs`. The adapter's `super::v2_sdk` and
   `crate::native_core::common` paths bind to the actual host modules.
4. In `src-tauri/src/native_core/extensions/v2_runtime.rs`:
   - Add `"geohash-encode-decode"` to the allow-list in `slug()`.
   - Add `"geohash-encode-decode" => super::v2_geohash::commands(),` in `commands()`.
   - Add a `handle()` arm forwarding the existing arguments unchanged:

   ```rust
   "geohash-encode-decode" => {
       super::v2_geohash::handle(ctx, command, operation, payload, state).await
   }
   ```

5. Append this directory's `manifest.json` as one object in
   `src-tauri/src/native_core/extensions/v2_catalog.json`. Do not replace the other
   entries. Validate the combined catalog and extend the pinned host test's hardcoded
   count (11 → 12 in `v2_catalog.rs::bundled_ids_and_coverage_are_consistent`). Check
   for other host coverage snapshots/counts affected by a new compiled adapter.
6. Copy `command-icon.png`, without transformation, to
   **`src/renderer/public/extensions/icons/geohash-encode-decode.png`**.
   This is the beta host's Vite public directory; the repository-root `public/`
   directory is not the correct asset destination. Its URL must be the manifest's
   `/extensions/icons/geohash-encode-decode.png`. Verify the SHA-256 below.
7. Rebuild the host renderer and native binary, restart the rebuilt app, then install
   the canonical Native V2 entry through the host's native catalog/install flow.
   Check `scripts/extensions-v2-list.mjs` against the rebuilt binary: both commands
   must report compiled `rust` coverage on macOS/Windows. Neither a native marker
   nor this manifest independently supplies executable code. Retain V1 for fallback.
8. Test both commands on **each real OS**: typing, invalid/retry, Back, all rows and
   precisions, copy contents, constructed browser links, and closeOnAction true/false.
   Record commit-specific QA artifacts before changing manifest verification states.

Preference values are supplied by beta NativeSessions. The adapter uses no file
migration or storage. The pinned host falls back to legacy preferences only in its
known `extensions/<slug>` and `packages/raycast.<slug>` paths when native preferences
are absent; no migration from other V1 IDs/locations is asserted. Keep V1 data intact
and set the native checkbox explicitly if the host does not discover that preference.

## Verification record and limits

2026-10-24, branch `feat/native-v2-rust`:

- Current Rust (1.98.0) and Rust 1.85.0 tests, formatter and strict Clippy passed.
- Windows GNU `cargo check --all-targets` and strict Clippy passed. This is
  cross-compilation/type checking, **not** execution on Windows.
- Full pinned beta JSON Schema validation and icon/hash checks passed.
- Optional source-oracle generation and actual beta renderer parser checks passed.
- `cargo package` and extracted-package tests passed with included fixtures.
- Retained V1 passed disposable `pnpm test` and `pnpm typecheck`; original root
  source, manifest, asset, lockfile and license were unchanged.
- **macOS desktop UI: not-run. Windows desktop UI: not-run.** Real clipboard,
  browser/window integration and host registration/rebuild were not performed.
  Linux has no declared support; adapter `commands()` is empty on Linux.

No full app build was performed: host edits and profile changes are outside this
implementation clone's authorization. Host API drift after the pinned beta must be
reviewed and tested before use. The crate is a compiled-in integration package,
not a dynamic plugin, executable installer or standalone desktop application.

## Provenance and licenses

See `NOTICE`, `LICENSE`, `licenses/`, and both fixture `provenance.json` files.
Source: Raycast commit `6734d85992290066fd31a7f5bee3cca3d3545a1a`, extension path
`extensions/geohash-encode-decode`, author **alrico88**. Rust port: **almatkai**.
Extension code is MIT; byte-exact beta fixtures are Apache-2.0. The crate's combined
SPDX packaging expression is `MIT AND Apache-2.0`; dependency licenses remain their
own. The optional JS oracle does not run in the extension.

Original icon: 172526 bytes; SHA-256:
`f59d02918ee4879a95940e3df317ff70ebeb901582967188b034b73daf0ee834`.
