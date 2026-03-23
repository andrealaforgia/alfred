//! Plugin discovery -- scans a directory for plugins.

use std::fs;
use std::path::Path;

use crate::error::PluginError;
use crate::metadata::PluginMetadata;

/// Scan a plugins directory for subdirectories containing init.lisp files.
///
/// Returns a tuple of successfully discovered plugins and any errors
/// encountered during discovery. Missing directory returns empty results.
pub fn scan(dir: &Path) -> (Vec<PluginMetadata>, Vec<PluginError>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return (vec![], vec![]),
    };

    let mut plugins = Vec::new();
    let mut errors = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        // Skip directories ending in .disabled
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.ends_with(".disabled") {
                continue;
            }
        }

        let init_path = path.join("init.lisp");
        if !init_path.exists() {
            errors.push(PluginError::MissingInitFile { path });
            continue;
        }

        let content = match fs::read_to_string(&init_path) {
            Ok(content) => content,
            Err(source) => {
                errors.push(PluginError::ReadError {
                    path: init_path,
                    source,
                });
                continue;
            }
        };

        match parse_metadata(&content, &init_path) {
            Ok(meta) => plugins.push(meta),
            Err(err) => errors.push(err),
        }
    }

    (plugins, errors)
}

/// Parse plugin metadata from the header comments of an init.lisp file.
///
/// Expected format: lines starting with `;;; key: value`.
fn parse_metadata(content: &str, init_path: &Path) -> Result<PluginMetadata, PluginError> {
    let mut name = None;
    let mut version = None;
    let mut description = None;
    let mut dependencies = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(";;; ") {
            if let Some((key, value)) = rest.split_once(": ") {
                let value = value.trim();
                match key.trim() {
                    "name" => name = Some(value.to_string()),
                    "version" => version = Some(value.to_string()),
                    "description" => description = Some(value.to_string()),
                    "depends" => {
                        dependencies = value
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                    _ => {}
                }
            }
        }
    }

    let name = name.ok_or_else(|| PluginError::ParseError {
        path: init_path.to_path_buf(),
        reason: "missing required field: name".to_string(),
    })?;

    Ok(PluginMetadata {
        name,
        version: version.unwrap_or_else(|| "0.0.0".to_string()),
        description: description.unwrap_or_default(),
        dependencies,
        source_path: init_path.to_path_buf(),
    })
}
