use crate::args::DEFAULT_SOURCE_REGISTRY_PATH;
use crate::registry::SourceRegistry;
use std::error::Error;
use std::path::PathBuf;

pub(crate) async fn run_healthcheck(raw_args: &[String]) -> Result<(), Box<dyn Error>> {
    let source_registry = healthcheck_source_registry(raw_args)?;
    SourceRegistry::load(&source_registry).await?;
    Ok(())
}

pub(crate) fn healthcheck_requested(raw_args: &[String]) -> bool {
    raw_args.iter().skip(1).any(|arg| arg == "--healthcheck")
}

fn healthcheck_source_registry(raw_args: &[String]) -> Result<PathBuf, String> {
    let mut source_registry = PathBuf::from(DEFAULT_SOURCE_REGISTRY_PATH);
    let mut index = 1;
    while index < raw_args.len() {
        match raw_args[index].as_str() {
            "--healthcheck" => index += 1,
            "--source-registry" => {
                let Some(value) = raw_args.get(index + 1) else {
                    return Err("--source-registry requires an absolute path".to_owned());
                };
                let path = PathBuf::from(value);
                if !path.is_absolute() {
                    return Err("--source-registry requires an absolute path".to_owned());
                }
                source_registry = path;
                index += 2;
            }
            other => return Err(format!("unsupported healthcheck argument: {other}")),
        }
    }
    Ok(source_registry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_healthcheck_mode() {
        assert!(healthcheck_requested(&[
            "intel-crawl-app".to_owned(),
            "--healthcheck".to_owned()
        ]));
        assert!(!healthcheck_requested(&["intel-crawl-app".to_owned()]));
    }

    #[test]
    fn healthcheck_uses_default_registry_path() {
        let source_registry = healthcheck_source_registry(&[
            "intel-crawl-app".to_owned(),
            "--healthcheck".to_owned(),
        ])
        .unwrap();

        assert_eq!(source_registry, PathBuf::from(DEFAULT_SOURCE_REGISTRY_PATH));
    }

    #[test]
    fn healthcheck_accepts_explicit_absolute_registry_path() {
        let source_registry = healthcheck_source_registry(&[
            "intel-crawl-app".to_owned(),
            "--healthcheck".to_owned(),
            "--source-registry".to_owned(),
            "/tmp/source-registry.json".to_owned(),
        ])
        .unwrap();

        assert_eq!(source_registry, PathBuf::from("/tmp/source-registry.json"));
    }

    #[test]
    fn healthcheck_rejects_relative_registry_path() {
        let error = healthcheck_source_registry(&[
            "intel-crawl-app".to_owned(),
            "--healthcheck".to_owned(),
            "--source-registry".to_owned(),
            "source-registry.json".to_owned(),
        ])
        .unwrap_err();

        assert!(error.contains("--source-registry requires an absolute path"));
    }
}
