//! Alfred Plugin -- plugin system for the Alfred text editor.

pub mod discovery;
pub mod error;
pub mod metadata;
pub mod registry;

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::fs;
    use std::rc::Rc;
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

    // -- Registry acceptance test --

    #[test]
    fn load_plugin_evaluates_init_lisp_and_tracks_plugin_in_registry() {
        // Given: a temp dir with a plugin containing init.lisp
        let tmp = TempDir::new().unwrap();
        let plugin_dir = tmp.path().join("test-plugin");
        fs::create_dir(&plugin_dir).unwrap();
        fs::write(
            plugin_dir.join("init.lisp"),
            ";;; name: test-plugin\n\
             ;;; version: 0.1.0\n\
             ;;; description: A test plugin\n\
             (message \"test-plugin loaded\")\n",
        )
        .unwrap();

        // And: discover the plugin metadata
        let (plugins, _) = discovery::scan(tmp.path());
        let meta = plugins.into_iter().next().unwrap();

        // And: a Lisp runtime with editor state and bridge primitives
        let state = Rc::new(RefCell::new(alfred_core::editor_state::new(80, 24)));
        let runtime = alfred_lisp::runtime::LispRuntime::new();
        alfred_lisp::bridge::register_core_primitives(&runtime, state.clone());

        // And: an empty registry
        let mut reg = registry::PluginRegistry::new();

        // When: load plugin
        let result = registry::load_plugin(&mut reg, meta, &runtime);

        // Then: loading succeeds
        assert!(result.is_ok(), "load_plugin failed: {:?}", result.err());

        // And: the plugin is tracked in the registry
        let names = registry::list_plugins(&reg);
        assert_eq!(names.len(), 1);
        assert_eq!(names[0].name, "test-plugin");

        // And: the init.lisp was evaluated (message primitive was called)
        let editor = state.borrow();
        assert_eq!(
            editor.message,
            Some("test-plugin loaded".to_string()),
            "init.lisp should have called (message ...) setting editor message"
        );
    }

    // -- Registry unit tests --

    /// Helper: create a runtime with bridge primitives and shared editor state.
    fn setup_runtime() -> (
        alfred_lisp::runtime::LispRuntime,
        Rc<RefCell<alfred_core::editor_state::EditorState>>,
    ) {
        let state = Rc::new(RefCell::new(alfred_core::editor_state::new(80, 24)));
        let runtime = alfred_lisp::runtime::LispRuntime::new();
        alfred_lisp::bridge::register_core_primitives(&runtime, state.clone());
        (runtime, state)
    }

    /// Helper: create a temp plugin directory with the given init.lisp content.
    fn create_test_plugin(
        tmp: &TempDir,
        name: &str,
        init_content: &str,
    ) -> metadata::PluginMetadata {
        let plugin_dir = tmp.path().join(name);
        fs::create_dir(&plugin_dir).unwrap();
        fs::write(plugin_dir.join("init.lisp"), init_content).unwrap();
        let (plugins, _) = discovery::scan(tmp.path());
        plugins.into_iter().find(|p| p.name == name).unwrap()
    }

    #[test]
    fn unload_removes_plugin_from_registry() {
        let tmp = TempDir::new().unwrap();
        let meta = create_test_plugin(
            &tmp,
            "removable",
            ";;; name: removable\n;;; version: 0.1.0\n;;; description: temp\n(+ 1 1)\n",
        );
        let (runtime, _state) = setup_runtime();
        let mut reg = registry::PluginRegistry::new();
        registry::load_plugin(&mut reg, meta, &runtime).unwrap();

        let result = registry::unload_plugin(&mut reg, "removable");

        assert!(result.is_ok(), "unload should succeed: {:?}", result.err());
        assert!(registry::list_plugins(&reg).is_empty());
    }

    #[test]
    fn load_duplicate_plugin_returns_error() {
        let tmp = TempDir::new().unwrap();
        let meta = create_test_plugin(
            &tmp,
            "dup-plugin",
            ";;; name: dup-plugin\n;;; version: 0.1.0\n;;; description: dup\n(+ 1 1)\n",
        );
        let (runtime, _state) = setup_runtime();
        let mut reg = registry::PluginRegistry::new();
        registry::load_plugin(&mut reg, meta.clone(), &runtime).unwrap();

        let result = registry::load_plugin(&mut reg, meta, &runtime);

        assert!(result.is_err(), "duplicate load should fail");
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("already loaded"),
            "error should mention 'already loaded', got: {err_msg}"
        );
    }

    #[test]
    fn unload_nonexistent_plugin_returns_error() {
        let mut reg = registry::PluginRegistry::new();

        let result = registry::unload_plugin(&mut reg, "ghost");

        assert!(result.is_err(), "unload of missing plugin should fail");
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("not found"),
            "error should mention 'not found', got: {err_msg}"
        );
    }

    #[test]
    fn load_plugin_with_invalid_lisp_returns_init_error() {
        let tmp = TempDir::new().unwrap();
        let plugin_dir = tmp.path().join("bad-init");
        fs::create_dir(&plugin_dir).unwrap();
        // Valid metadata but invalid Lisp code
        fs::write(
            plugin_dir.join("init.lisp"),
            ";;; name: bad-init\n;;; version: 0.1.0\n;;; description: bad\n(undefined-fn)\n",
        )
        .unwrap();
        let (plugins, _) = discovery::scan(tmp.path());
        let meta = plugins.into_iter().next().unwrap();
        let (runtime, _state) = setup_runtime();
        let mut reg = registry::PluginRegistry::new();

        let result = registry::load_plugin(&mut reg, meta, &runtime);

        assert!(result.is_err(), "load with bad init should fail");
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("init error"),
            "error should mention 'init error', got: {err_msg}"
        );
        // Plugin should NOT be in the registry after failed load
        assert!(registry::list_plugins(&reg).is_empty());
    }

    // -- Acceptance test: command and hook tracking per plugin --

    #[test]
    fn unload_removes_plugin_commands_and_hooks_while_other_plugin_survives() {
        // Given: two plugins loaded with commands and hooks tracked per plugin
        let tmp = TempDir::new().unwrap();
        let meta_a = create_test_plugin(
            &tmp,
            "plugin-a",
            ";;; name: plugin-a\n;;; version: 1.0.0\n;;; description: first\n(+ 1 1)\n",
        );
        let meta_b = create_test_plugin(
            &tmp,
            "plugin-b",
            ";;; name: plugin-b\n;;; version: 1.0.0\n;;; description: second\n(+ 2 2)\n",
        );
        let (runtime, state) = setup_runtime();
        let mut reg = registry::PluginRegistry::new();

        // Load plugin-a and register commands/hooks for it
        registry::load_plugin(&mut reg, meta_a, &runtime).unwrap();
        {
            let mut editor = state.borrow_mut();
            alfred_core::command::register(
                &mut editor.commands,
                "cmd-a1".to_string(),
                alfred_core::command::CommandHandler::Native(|_s| Ok(())),
            );
            alfred_core::command::register(
                &mut editor.commands,
                "cmd-a2".to_string(),
                alfred_core::command::CommandHandler::Native(|_s| Ok(())),
            );
        }
        registry::track_command(&mut reg, "plugin-a", "cmd-a1");
        registry::track_command(&mut reg, "plugin-a", "cmd-a2");
        registry::track_hook(&mut reg, "plugin-a", "on-save-a");

        // Load plugin-b and register commands/hooks for it
        registry::load_plugin(&mut reg, meta_b, &runtime).unwrap();
        {
            let mut editor = state.borrow_mut();
            alfred_core::command::register(
                &mut editor.commands,
                "cmd-b1".to_string(),
                alfred_core::command::CommandHandler::Native(|_s| Ok(())),
            );
        }
        registry::track_command(&mut reg, "plugin-b", "cmd-b1");
        registry::track_hook(&mut reg, "plugin-b", "on-save-b");

        // When: unload plugin-a, passing the editor state for cleanup
        {
            let mut editor = state.borrow_mut();
            registry::unload_plugin_with_cleanup(&mut reg, "plugin-a", &mut editor.commands)
                .unwrap();
        }

        // Then: plugin-a's commands are removed from CommandRegistry
        {
            let editor = state.borrow();
            assert!(
                alfred_core::command::lookup(&editor.commands, "cmd-a1").is_none(),
                "cmd-a1 should be removed after unloading plugin-a"
            );
            assert!(
                alfred_core::command::lookup(&editor.commands, "cmd-a2").is_none(),
                "cmd-a2 should be removed after unloading plugin-a"
            );
        }

        // And: plugin-b's commands survive
        {
            let editor = state.borrow();
            assert!(
                alfred_core::command::lookup(&editor.commands, "cmd-b1").is_some(),
                "cmd-b1 should survive after unloading plugin-a"
            );
        }

        // And: plugin-a's hooks are removed from registry tracking
        assert!(
            registry::plugin_hooks(&reg, "plugin-a").is_none(),
            "plugin-a hooks should be gone after unload"
        );

        // And: plugin-b's hooks survive
        assert_eq!(
            registry::plugin_hooks(&reg, "plugin-b"),
            Some(vec!["on-save-b".to_string()]),
            "plugin-b hooks should survive after unloading plugin-a"
        );
    }

    // -- Unit tests: command and hook tracking --

    #[test]
    fn track_command_associates_command_with_plugin() {
        let tmp = TempDir::new().unwrap();
        let meta = create_test_plugin(
            &tmp,
            "tracker",
            ";;; name: tracker\n;;; version: 1.0.0\n;;; description: tracks\n(+ 1 1)\n",
        );
        let (runtime, _state) = setup_runtime();
        let mut reg = registry::PluginRegistry::new();
        registry::load_plugin(&mut reg, meta, &runtime).unwrap();

        registry::track_command(&mut reg, "tracker", "my-cmd");

        assert_eq!(
            registry::plugin_commands(&reg, "tracker"),
            Some(vec!["my-cmd".to_string()]),
        );
    }

    #[test]
    fn track_hook_associates_hook_with_plugin() {
        let tmp = TempDir::new().unwrap();
        let meta = create_test_plugin(
            &tmp,
            "hooker",
            ";;; name: hooker\n;;; version: 1.0.0\n;;; description: hooks\n(+ 1 1)\n",
        );
        let (runtime, _state) = setup_runtime();
        let mut reg = registry::PluginRegistry::new();
        registry::load_plugin(&mut reg, meta, &runtime).unwrap();

        registry::track_hook(&mut reg, "hooker", "on-save");

        assert_eq!(
            registry::plugin_hooks(&reg, "hooker"),
            Some(vec!["on-save".to_string()]),
        );
    }

    #[test]
    fn unload_with_cleanup_removes_tracked_commands_from_command_registry() {
        let tmp = TempDir::new().unwrap();
        let meta = create_test_plugin(
            &tmp,
            "removable",
            ";;; name: removable\n;;; version: 1.0.0\n;;; description: temp\n(+ 1 1)\n",
        );
        let (runtime, state) = setup_runtime();
        let mut reg = registry::PluginRegistry::new();
        registry::load_plugin(&mut reg, meta, &runtime).unwrap();

        // Register a command in CommandRegistry and track it
        {
            let mut editor = state.borrow_mut();
            alfred_core::command::register(
                &mut editor.commands,
                "doomed-cmd".to_string(),
                alfred_core::command::CommandHandler::Native(|_s| Ok(())),
            );
        }
        registry::track_command(&mut reg, "removable", "doomed-cmd");

        // Unload with cleanup
        {
            let mut editor = state.borrow_mut();
            registry::unload_plugin_with_cleanup(&mut reg, "removable", &mut editor.commands)
                .unwrap();
        }

        // Command should be gone
        let editor = state.borrow();
        assert!(alfred_core::command::lookup(&editor.commands, "doomed-cmd").is_none());
    }

    #[test]
    fn unload_with_cleanup_removes_tracked_hooks_from_registry() {
        let tmp = TempDir::new().unwrap();
        let meta = create_test_plugin(
            &tmp,
            "hook-plugin",
            ";;; name: hook-plugin\n;;; version: 1.0.0\n;;; description: hooks\n(+ 1 1)\n",
        );
        let (runtime, _state) = setup_runtime();
        let mut reg = registry::PluginRegistry::new();
        registry::load_plugin(&mut reg, meta, &runtime).unwrap();
        registry::track_hook(&mut reg, "hook-plugin", "on-open");

        // Unload with cleanup (no command registry needed for hook-only test)
        {
            let mut cmd_reg = alfred_core::command::CommandRegistry::new();
            registry::unload_plugin_with_cleanup(&mut reg, "hook-plugin", &mut cmd_reg).unwrap();
        }

        assert!(
            registry::plugin_hooks(&reg, "hook-plugin").is_none(),
            "hooks should be gone after unload"
        );
    }

    #[test]
    fn unload_preserves_other_plugin_commands_in_command_registry() {
        let tmp = TempDir::new().unwrap();
        let meta_a = create_test_plugin(
            &tmp,
            "keeper",
            ";;; name: keeper\n;;; version: 1.0.0\n;;; description: stays\n(+ 1 1)\n",
        );
        let meta_b = create_test_plugin(
            &tmp,
            "removed",
            ";;; name: removed\n;;; version: 1.0.0\n;;; description: goes\n(+ 2 2)\n",
        );
        let (runtime, state) = setup_runtime();
        let mut reg = registry::PluginRegistry::new();
        registry::load_plugin(&mut reg, meta_a, &runtime).unwrap();
        registry::load_plugin(&mut reg, meta_b, &runtime).unwrap();

        // Register commands for both
        {
            let mut editor = state.borrow_mut();
            alfred_core::command::register(
                &mut editor.commands,
                "keep-cmd".to_string(),
                alfred_core::command::CommandHandler::Native(|_s| Ok(())),
            );
            alfred_core::command::register(
                &mut editor.commands,
                "remove-cmd".to_string(),
                alfred_core::command::CommandHandler::Native(|_s| Ok(())),
            );
        }
        registry::track_command(&mut reg, "keeper", "keep-cmd");
        registry::track_command(&mut reg, "removed", "remove-cmd");

        // Unload "removed"
        {
            let mut editor = state.borrow_mut();
            registry::unload_plugin_with_cleanup(&mut reg, "removed", &mut editor.commands)
                .unwrap();
        }

        // "keeper" command survives
        let editor = state.borrow();
        assert!(
            alfred_core::command::lookup(&editor.commands, "keep-cmd").is_some(),
            "keep-cmd should survive unloading 'removed' plugin"
        );
        assert!(
            alfred_core::command::lookup(&editor.commands, "remove-cmd").is_none(),
            "remove-cmd should be gone after unloading 'removed' plugin"
        );
    }

    #[test]
    fn list_plugins_returns_all_loaded_plugins() {
        let tmp = TempDir::new().unwrap();
        let meta_a = create_test_plugin(
            &tmp,
            "alpha",
            ";;; name: alpha\n;;; version: 1.0.0\n;;; description: first\n(+ 1 1)\n",
        );
        let meta_b = create_test_plugin(
            &tmp,
            "beta",
            ";;; name: beta\n;;; version: 2.0.0\n;;; description: second\n(+ 2 2)\n",
        );
        let (runtime, _state) = setup_runtime();
        let mut reg = registry::PluginRegistry::new();
        registry::load_plugin(&mut reg, meta_a, &runtime).unwrap();
        registry::load_plugin(&mut reg, meta_b, &runtime).unwrap();

        let mut names: Vec<String> = registry::list_plugins(&reg)
            .iter()
            .map(|p| p.name.clone())
            .collect();
        names.sort();

        assert_eq!(names, vec!["alpha", "beta"]);
    }
}
