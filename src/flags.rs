//! Fail-closed argv and environment resolution through flags-2-env.

use std::collections::BTreeMap;
use std::io::Write;

use flags2env::BundledFlags2Env;
use tempfile::NamedTempFile;

const CONTRACT: &str = include_str!("../.cli-flags.toml");

pub fn resolve() -> Result<BTreeMap<String, String>, String> {
    resolve_from(&std::env::args().collect::<Vec<_>>(), std::env::vars())
}

fn resolve_from(
    argv: &[String],
    environment: impl IntoIterator<Item = (String, String)>,
) -> Result<BTreeMap<String, String>, String> {
    let mut contract = NamedTempFile::new()
        .map_err(|_| "cannot create embedded flags-2-env contract".to_owned())?;
    contract
        .write_all(CONTRACT.as_bytes())
        .map_err(|_| "cannot materialize embedded flags-2-env contract".to_owned())?;
    let path = contract
        .path()
        .to_str()
        .ok_or_else(|| "flags-2-env contract path is not valid UTF-8".to_owned())?;

    let parser = BundledFlags2Env::new();
    parser
        .audit_config(Some(path))
        .map_err(|_| "flags-2-env contract audit failed".to_owned())?;
    let parsed = parser
        .parse_structured(argv, Some(path))
        .map_err(|_| "flags-2-env parsing failed".to_owned())?;

    if !parsed.unknown_options.is_empty() || !parsed.errors.is_empty() || !parsed.extras.is_empty() {
        return Err(format!(
            "invalid command-line arguments: unknown={}, errors={}, extras={}",
            parsed.unknown_options.len(),
            parsed.errors.len(),
            parsed.extras.len()
        ));
    }

    // `.cli-flags.toml` has env.load=false, so plaintext dotenv discovery is
    // not part of this privileged server boundary. Process environment is the
    // baseline; explicitly declared non-secret CLI values override it.
    let mut raw = environment.into_iter().collect::<BTreeMap<_, _>>();
    raw.extend(parsed.provided_flags);

    let typed = parser
        .coerce::<serde_json::Map<String, serde_json::Value>, _>(&raw, Some(path))
        .map_err(|_| "flags-2-env typed configuration failed".to_owned())?;
    typed
        .into_iter()
        .filter(|(_, value)| !value.is_null())
        .map(|(name, value)| scalar_string(&name, value).map(|value| (name, value)))
        .collect()
}

fn scalar_string(name: &str, value: serde_json::Value) -> Result<String, String> {
    match value {
        serde_json::Value::String(value) => Ok(value),
        serde_json::Value::Bool(value) => Ok(value.to_string()),
        serde_json::Value::Number(value) => Ok(value.to_string()),
        _ => Err(format!(
            "flags-2-env returned a non-scalar value for {name}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_options_fail_closed_without_echoing_values() {
        let error = resolve_from(
            &[
                "server".to_owned(),
                "--definitely-unknown=do-not-echo".to_owned(),
            ],
            std::iter::empty(),
        )
        .expect_err("unknown option");
        assert!(error.contains("unknown=1"));
        assert!(!error.contains("definitely-unknown"));
        assert!(!error.contains("do-not-echo"));
    }

    #[test]
    fn command_line_overrides_environment_for_declared_non_secret_values() {
        let resolved = resolve_from(
            &[
                "server".to_owned(),
                "--api-base=https://cli.example.test".to_owned(),
            ],
            [("DECLMIG_API_BASE".to_owned(), "https://env.example.test".to_owned())],
        )
        .expect("resolved configuration");
        assert_eq!(
            resolved.get("DECLMIG_API_BASE").map(String::as_str),
            Some("https://cli.example.test")
        );
    }

    #[test]
    fn nats_url_has_no_command_line_surface() {
        let error = resolve_from(
            &[
                "server".to_owned(),
                "--declmig-nats-url=nats://user:secret@example.test".to_owned(),
            ],
            std::iter::empty(),
        )
        .expect_err("NATS URL must not be accepted on argv");
        assert!(error.contains("unknown=1"));
        assert!(!error.contains("user:secret"));
    }
}
