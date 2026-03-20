//! M3 Acceptance Tests: Plugin System -- Discovery, Loading, Lifecycle
//!
//! What M3 proves: Can discover, load, initialize, and unload Lisp plugins.
//!
//! Driving ports exercised:
//!   - PluginRegistry::load_all() — discover and load plugins from directory
//!   - PluginRegistry::unload() — unload a specific plugin
//!   - PluginRegistry::list_plugins() — query discovered plugins
//!   - discovery::scan() — scan directory for plugin metadata
//!   - CommandRegistry — verify commands registered/removed by plugins
//!   - EditorState — observe state changes from plugin commands
//!
//! Tests use temporary directories with Lisp plugin fixtures to ensure
//! isolation. Each test creates its own plugin directory.

mod helpers;

// ---------------------------------------------------------------------------
// Walking Skeleton
// ---------------------------------------------------------------------------

/// WS-3: Plugin loads and its command is callable.
///
/// Given a test plugin in the plugins directory with an init function
///   that registers a command called "test-greet"
/// When the editor starts and loads plugins
/// Then the "test-greet" command is registered
/// And executing the command produces the expected effect
#[test]
#[ignore]
fn given_test_plugin_when_loaded_then_command_is_registered_and_callable() {
    // Given: a test plugin that registers a "test-greet" command
    let init_lisp = r#"
        (define-plugin
          :name "test-plugin"
          :version "0.1.0"
          :description "A test plugin")

        (defn init []
          (define-command "test-greet"
            (lambda () (message "Hello from plugin"))))
    "#;
    let (_dir, plugins_path) = helpers::create_test_plugin_dir("test-plugin", init_lisp);
    let _ = plugins_path;

    // When: the editor loads plugins
    // let mut state = EditorState::new();
    // let mut runtime = LispRuntime::new();
    // bridge::register_core_primitives(&mut runtime, &mut state).unwrap();
    // let mut registry = PluginRegistry::new();
    // registry.load_all(&plugins_path, &mut runtime, &mut state).unwrap();

    // Then: "test-greet" command is registered
    // assert!(state.commands.has("test-greet"));

    // And: executing it produces the expected effect
    // state.commands.execute("test-greet", &mut state).unwrap();
    // assert_eq!(state.message, Some("Hello from plugin".to_string()));

    todo!("Implement when alfred-plugin crate exists");
}

// ---------------------------------------------------------------------------
// Happy Path
// ---------------------------------------------------------------------------

/// M3-H1: Multiple plugins discovered with metadata.
///
/// Given multiple plugins in the plugins directory
/// When the editor starts
/// Then all plugins are discovered and their metadata is available
#[test]
#[ignore]
fn given_multiple_plugins_when_editor_starts_then_all_discovered_with_metadata() {
    // Given: two plugins
    let plugins = [
        ("plugin-a", r#"
            (define-plugin :name "plugin-a" :version "1.0.0" :description "Plugin A")
            (defn init [] nil)
        "#),
        ("plugin-b", r#"
            (define-plugin :name "plugin-b" :version "2.0.0" :description "Plugin B")
            (defn init [] nil)
        "#),
    ];
    let (_dir, plugins_path) = helpers::create_multi_plugin_dir(&plugins);
    let _ = plugins_path;

    // When: editor starts and loads plugins
    // let plugins = discovery::scan(&plugins_path);

    // Then: both plugins discovered with correct metadata
    // assert_eq!(plugins.len(), 2);
    // let names: Vec<&str> = plugins.iter().map(|p| p.name.as_str()).collect();
    // assert!(names.contains(&"plugin-a"));
    // assert!(names.contains(&"plugin-b"));

    todo!("Implement when alfred-plugin crate exists");
}

/// M3-H2: Unloading a plugin removes its commands.
///
/// Given a loaded plugin that registered a command
/// When the plugin is unloaded
/// Then its registered commands are removed from the registry
#[test]
#[ignore]
fn given_loaded_plugin_when_unloaded_then_commands_removed() {
    // Given: plugin is loaded and command is registered
    let init_lisp = r#"
        (define-plugin :name "removable" :version "0.1.0" :description "Test")
        (defn init []
          (define-command "removable-cmd" (lambda () (message "hi"))))
    "#;
    let (_dir, plugins_path) = helpers::create_test_plugin_dir("removable", init_lisp);
    let _ = plugins_path;

    // Load the plugin
    // assert!(state.commands.has("removable-cmd"));

    // When: unload the plugin
    // registry.unload("removable", &mut state).unwrap();

    // Then: command is gone
    // assert!(!state.commands.has("removable-cmd"));

    todo!("Implement when alfred-plugin crate exists");
}

/// M3-H3: Plugins with dependencies loaded in topological order.
///
/// Given a plugin with a declared dependency on another plugin
/// When plugins are loaded
/// Then the dependency is loaded before the dependent
#[test]
#[ignore]
fn given_plugin_with_dependency_when_loaded_then_dependency_loaded_first() {
    // Given: plugin-b depends on plugin-a
    let plugins = [
        ("plugin-a", r#"
            (define-plugin :name "plugin-a" :version "1.0.0" :description "Base")
            (defn init [] (define-command "base-cmd" (lambda () nil)))
        "#),
        ("plugin-b", r#"
            (define-plugin :name "plugin-b" :version "1.0.0"
              :description "Depends on A" :dependencies ["plugin-a"])
            (defn init []
              (define-command "derived-cmd" (lambda () (execute-command "base-cmd"))))
        "#),
    ];
    let (_dir, plugins_path) = helpers::create_multi_plugin_dir(&plugins);
    let _ = plugins_path;

    // When: plugins loaded
    // registry.load_all(&plugins_path, &mut runtime, &mut state).unwrap();

    // Then: both commands exist (proving plugin-a loaded before plugin-b)
    // assert!(state.commands.has("base-cmd"));
    // assert!(state.commands.has("derived-cmd"));

    todo!("Implement when alfred-plugin crate exists");
}

// ---------------------------------------------------------------------------
// Error Path
// ---------------------------------------------------------------------------

/// M3-E1: Missing plugins directory does not crash.
///
/// Given the plugins directory does not exist
/// When the editor starts
/// Then it starts normally with no plugins loaded
#[test]
#[ignore]
fn given_no_plugins_directory_when_editor_starts_then_starts_with_no_plugins() {
    // Given: a path that does not exist
    let nonexistent = std::path::PathBuf::from("/tmp/alfred-test-nonexistent-plugins-dir");

    // When: attempt to load plugins
    // let result = registry.load_all(&nonexistent, &mut runtime, &mut state);

    // Then: no crash, no plugins loaded
    // assert!(result.is_ok()); // OR: result is Ok with warning
    // assert_eq!(registry.list_plugins().len(), 0);
    let _ = nonexistent;

    todo!("Implement when alfred-plugin crate exists");
}

/// M3-E2: Plugin with syntax error does not prevent others from loading.
///
/// Given a plugin with a syntax error in init.lisp
/// When the editor loads plugins
/// Then the broken plugin reports an error and other plugins still load
#[test]
#[ignore]
fn given_plugin_with_syntax_error_when_loading_then_others_still_load() {
    // Given: one good plugin, one broken plugin
    let plugins = [
        ("good-plugin", r#"
            (define-plugin :name "good-plugin" :version "1.0.0" :description "Works")
            (defn init [] (define-command "good-cmd" (lambda () nil)))
        "#),
        ("broken-plugin", r#"
            (define-plugin :name "broken-plugin" :version "1.0.0"
            ; Missing closing paren -- syntax error
        "#),
    ];
    let (_dir, plugins_path) = helpers::create_multi_plugin_dir(&plugins);
    let _ = plugins_path;

    // When: load plugins
    // registry.load_all(&plugins_path, &mut runtime, &mut state).unwrap();

    // Then: good plugin loaded, broken plugin has error status
    // assert!(state.commands.has("good-cmd"));
    // let broken = registry.get_plugin("broken-plugin");
    // assert!(matches!(broken.status, PluginStatus::Error(_)));

    todo!("Implement when alfred-plugin crate exists");
}

/// M3-E3: Plugin whose init function throws an error is marked as errored.
///
/// Given a plugin whose init function throws an error
/// When the editor loads plugins
/// Then the failing plugin is marked as errored and other plugins are unaffected
#[test]
#[ignore]
fn given_plugin_with_init_error_when_loading_then_marked_errored_others_unaffected() {
    // Given: plugin whose init throws
    let plugins = [
        ("healthy-plugin", r#"
            (define-plugin :name "healthy" :version "1.0.0" :description "OK")
            (defn init [] (define-command "healthy-cmd" (lambda () nil)))
        "#),
        ("failing-plugin", r#"
            (define-plugin :name "failing" :version "1.0.0" :description "Fails on init")
            (defn init [] (error "init failed intentionally"))
        "#),
    ];
    let (_dir, plugins_path) = helpers::create_multi_plugin_dir(&plugins);
    let _ = plugins_path;

    // When: load
    // registry.load_all(&plugins_path, &mut runtime, &mut state).unwrap();

    // Then: healthy loaded, failing has error status
    // assert!(state.commands.has("healthy-cmd"));
    // let failing = registry.get_plugin("failing");
    // assert!(matches!(failing.status, PluginStatus::Error(_)));

    todo!("Implement when alfred-plugin crate exists");
}

/// M3-E4: Plugin with missing dependency reports error.
///
/// Given a plugin declares a dependency that does not exist
/// When the editor loads plugins
/// Then the plugin reports an error about the missing dependency
#[test]
#[ignore]
fn given_plugin_with_missing_dependency_when_loading_then_reports_error() {
    // Given: plugin depends on "nonexistent-dep"
    let init_lisp = r#"
        (define-plugin :name "needy" :version "1.0.0"
          :description "Needs missing dep" :dependencies ["nonexistent-dep"])
        (defn init [] nil)
    "#;
    let (_dir, plugins_path) = helpers::create_test_plugin_dir("needy", init_lisp);
    let _ = plugins_path;

    // When: load
    // registry.load_all(&plugins_path, &mut runtime, &mut state).unwrap();

    // Then: plugin has error status mentioning missing dependency
    // let needy = registry.get_plugin("needy");
    // assert!(matches!(needy.status, PluginStatus::Error(ref msg) if msg.contains("nonexistent-dep")));

    todo!("Implement when alfred-plugin crate exists");
}

/// M3-E5: Directory without init.lisp is skipped.
///
/// Given a plugin directory with no init.lisp file
/// When the editor scans for plugins
/// Then the directory is skipped without error
#[test]
#[ignore]
fn given_directory_without_init_lisp_when_scanning_then_skipped() {
    // Given: a directory with a random file but no init.lisp
    let dir = tempfile::TempDir::new().unwrap();
    let plugin_dir = dir.path().join("not-a-plugin");
    std::fs::create_dir_all(&plugin_dir).unwrap();
    std::fs::write(plugin_dir.join("README.md"), "not a plugin").unwrap();

    // When: scan
    // let plugins = discovery::scan(dir.path());

    // Then: no plugins found, no error
    // assert_eq!(plugins.len(), 0);

    todo!("Implement when alfred-plugin crate exists");
}

// ---------------------------------------------------------------------------
// Edge Cases
// ---------------------------------------------------------------------------

/// M3-EC1: Empty plugins directory results in zero plugins.
///
/// Given an empty plugins directory
/// When the editor starts
/// Then it starts normally with zero plugins
#[test]
#[ignore]
fn given_empty_plugins_directory_when_editor_starts_then_zero_plugins() {
    // Given: empty directory
    let (_dir, plugins_path) = helpers::create_empty_plugins_dir();
    let _ = plugins_path;

    // When: load
    // registry.load_all(&plugins_path, &mut runtime, &mut state).unwrap();

    // Then: zero plugins
    // assert_eq!(registry.list_plugins().len(), 0);

    todo!("Implement when alfred-plugin crate exists");
}

/// M3-EC2: Plugin registering multiple commands has all removed on unload.
///
/// Given a plugin that registers three commands
/// When the plugin is unloaded
/// Then all three commands are removed
#[test]
#[ignore]
fn given_plugin_with_three_commands_when_unloaded_then_all_three_removed() {
    // Given: plugin registers three commands
    let init_lisp = r#"
        (define-plugin :name "multi" :version "1.0.0" :description "Multi-cmd")
        (defn init []
          (define-command "cmd-a" (lambda () nil))
          (define-command "cmd-b" (lambda () nil))
          (define-command "cmd-c" (lambda () nil)))
    "#;
    let (_dir, plugins_path) = helpers::create_test_plugin_dir("multi", init_lisp);
    let _ = plugins_path;

    // Load and verify all three registered
    // assert!(state.commands.has("cmd-a"));
    // assert!(state.commands.has("cmd-b"));
    // assert!(state.commands.has("cmd-c"));

    // When: unload
    // registry.unload("multi", &mut state).unwrap();

    // Then: all three removed
    // assert!(!state.commands.has("cmd-a"));
    // assert!(!state.commands.has("cmd-b"));
    // assert!(!state.commands.has("cmd-c"));

    todo!("Implement when alfred-plugin crate exists");
}
