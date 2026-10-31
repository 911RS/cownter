use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

fn read(rel: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)).unwrap()
}

fn handlers() -> BTreeSet<String> {
    let lib = read("src/lib.rs");
    let start = lib.find("generate_handler![").expect("handler list");
    let end = start + lib[start..].find(']').unwrap();
    lib[start..end].split("commands::").skip(1).map(|s| s.trim_end_matches(|c: char| !c.is_alphanumeric() && c != '_').trim().to_string()).collect()
}

fn quoted(text: &str) -> BTreeSet<String> {
    text.split('"').skip(1).step_by(2).map(str::to_string).collect()
}

#[test]
fn every_command_has_a_permission() {
    let manifest = quoted(&read("build.rs"));
    let missing: Vec<_> = handlers().into_iter().filter(|c| !manifest.contains(c)).collect();
    assert!(missing.is_empty(), "commands missing from build.rs COMMANDS: {missing:?}");
}

#[test]
fn every_command_is_allowed_in_some_window() {
    let mut allowed = quoted(&read("capabilities/default.json"));
    allowed.extend(quoted(&read("capabilities/settings.json")));
    let missing: Vec<_> = handlers().into_iter().filter(|c| !allowed.contains(&format!("allow-{}", c.replace('_', "-")))).collect();
    assert!(missing.is_empty(), "commands not allowed in any capability: {missing:?}");
}
