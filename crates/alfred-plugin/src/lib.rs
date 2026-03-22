//! Alfred Plugin -- plugin system for the Alfred text editor.

pub mod discovery;
pub mod error;
pub mod metadata;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // -- Acceptance test --

    #[test]
    fn scan_discovers_plugin_with_valid_metadata() {
        let tmp = TempDir::new().unwrap();
        let plugin_dir = tmp.path().join("my-plugin");
        fs::create_dir(&plugin_dir).unwrap();
        fs::write(
            plugin_dir.join("init.lisp"),
            ";;; name: my-plugin\n\
             ;;; version: 0.1.0\n\
             ;;; description: A test plugin\n\
             ;;; depends: dep1, dep2\n\
             \n\
             (defun hello () \"hello\")\n",
        )
        .unwrap();

        let (plugins, errors) = discovery::scan(tmp.path());

        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        assert_eq!(plugins.len(), 1);
        let p = &plugins[0];
        assert_eq!(p.name, "my-plugin");
        assert_eq!(p.version, "0.1.0");
        assert_eq!(p.description, "A test plugin");
        assert_eq!(p.dependencies, vec!["dep1", "dep2"]);
        assert_eq!(p.source_path, plugin_dir.join("init.lisp"));
    }

    // -- Unit tests --

    #[test]
    fn scan_returns_empty_for_nonexistent_directory() {
        let tmp = TempDir::new().unwrap();
        let missing = tmp.path().join("does-not-exist");

        let (plugins, errors) = discovery::scan(&missing);

        assert!(plugins.is_empty());
        assert!(errors.is_empty());
    }

    #[test]
    fn scan_returns_empty_for_directory_with_no_subdirs() {
        let tmp = TempDir::new().unwrap();
        // plugins dir exists but has no plugin subdirs
        let (plugins, errors) = discovery::scan(tmp.path());

        assert!(plugins.is_empty());
        assert!(errors.is_empty());
    }

    #[test]
    fn scan_reports_error_for_subdir_without_init_lisp() {
        let tmp = TempDir::new().unwrap();
        let plugin_dir = tmp.path().join("broken-plugin");
        fs::create_dir(&plugin_dir).unwrap();
        // No init.lisp created

        let (plugins, errors) = discovery::scan(tmp.path());

        assert!(plugins.is_empty());
        assert_eq!(errors.len(), 1);
        let err_msg = format!("{}", errors[0]);
        assert!(
            err_msg.contains("missing init.lisp"),
            "expected 'missing init.lisp' in error: {err_msg}"
        );
    }

    #[test]
    fn scan_reports_error_for_malformed_metadata() {
        let tmp = TempDir::new().unwrap();
        let plugin_dir = tmp.path().join("bad-plugin");
        fs::create_dir(&plugin_dir).unwrap();
        // init.lisp exists but has no metadata comments -- missing required name
        fs::write(plugin_dir.join("init.lisp"), "(defun hello () \"hello\")\n").unwrap();

        let (plugins, errors) = discovery::scan(tmp.path());

        assert!(plugins.is_empty());
        assert_eq!(errors.len(), 1);
        let err_msg = format!("{}", errors[0]);
        assert!(
            err_msg.contains("parse error"),
            "expected 'parse error' in error: {err_msg}"
        );
    }

    #[test]
    fn scan_discovers_multiple_plugins_and_collects_errors() {
        let tmp = TempDir::new().unwrap();

        // Valid plugin
        let good = tmp.path().join("good-plugin");
        fs::create_dir(&good).unwrap();
        fs::write(
            good.join("init.lisp"),
            ";;; name: good-plugin\n\
             ;;; version: 1.0.0\n\
             ;;; description: Works fine\n",
        )
        .unwrap();

        // Broken plugin (no init.lisp)
        let bad = tmp.path().join("bad-plugin");
        fs::create_dir(&bad).unwrap();

        let (plugins, errors) = discovery::scan(tmp.path());

        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "good-plugin");
        assert!(plugins[0].dependencies.is_empty());
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn scan_parses_plugin_with_no_dependencies() {
        let tmp = TempDir::new().unwrap();
        let plugin_dir = tmp.path().join("solo-plugin");
        fs::create_dir(&plugin_dir).unwrap();
        fs::write(
            plugin_dir.join("init.lisp"),
            ";;; name: solo-plugin\n\
             ;;; version: 2.0.0\n\
             ;;; description: No deps\n",
        )
        .unwrap();

        let (plugins, errors) = discovery::scan(tmp.path());

        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "solo-plugin");
        assert!(plugins[0].dependencies.is_empty());
    }
}
