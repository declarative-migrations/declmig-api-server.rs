use std::{fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: &str) -> String {
    fs::read_to_string(root().join(path)).unwrap_or_else(|error| panic!("failed to read {path}: {error}"))
}

#[test]
fn server_policy_disables_dotenv_and_fails_closed() {
    let policy = read(".cli-flags.toml");
    assert!(policy.contains("[parse]"));
    assert!(policy.contains("allow_unknown = false"));
    assert!(policy.contains("[env]"));
    assert!(policy.contains("load = false"));
}

#[test]
fn nats_url_is_environment_only() {
    let policy = read(".cli-flags.toml");
    assert!(policy.contains("ignore = [\"DECLMIG_NATS_URL\"]"));
    assert!(!policy.contains("[flags.declmig-nats-url]"));
}

#[test]
fn policy_uses_current_alias_contract_not_retired_long_keys() {
    let policy = read(".cli-flags.toml");
    assert!(policy.contains("aliases = [\"api-base\"]"));
    assert!(!policy.lines().any(|line| line.trim_start().starts_with("long =")));
}

#[test]
fn runtime_embeds_policy_and_redacts_parser_details() {
    let source = read("src/flags.rs");
    assert!(source.contains("include_str!(\"../.cli-flags.toml\")"));
    assert!(source.contains("BundledFlags2Env"));
    assert!(!source.contains("Flags2Env::load"));
    assert!(source.contains("unknown={}, errors={}, extras={}"));
    assert!(!source.contains("parsed.errors.join"));
    assert!(!source.contains("unknown command-line option(s):"));
}

#[test]
fn flags2env_dependency_is_immutable() {
    let manifest = read("Cargo.toml");
    let dependency = manifest
        .lines()
        .find(|line| line.trim_start().starts_with("flags2env ="))
        .expect("flags2env dependency");
    assert!(dependency.contains("rev = \""));
    assert!(!dependency.contains("branch =") && !dependency.contains("tag ="));
}
