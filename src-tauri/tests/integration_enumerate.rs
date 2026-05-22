//! Runs the real enumeration against the host. Ignored by default so CI on
//! non-Ubuntu hosts stays green; run locally with `--ignored`.
#[test]
#[ignore]
fn enumerates_real_apps() {
    let agg = showcase_lib::commands::enumerate();
    println!("apps={} warnings={:?}", agg.apps.len(), agg.warnings);
    assert!(!agg.apps.is_empty(), "expected at least one installed app");
}
