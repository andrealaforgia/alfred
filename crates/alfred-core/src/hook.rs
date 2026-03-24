//! HookRegistry: named hook storage with callback registration and dispatch.
//!
//! Hooks are extension points that plugins use to inject behavior
//! (e.g., render-gutter-hook for line numbers). The hook system supports
//! registering callbacks, dispatching all callbacks for a hook, and
//! unregistering specific callbacks by ID.
//!
//! This module has no I/O dependencies -- HookRegistry is pure state.

use std::collections::HashMap;
use std::rc::Rc;

/// Type alias for hook callback closures.
pub type HookCallbackFn = dyn Fn(&[String]) -> Vec<String>;

/// Unique identifier for a registered hook callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HookId(pub usize);

/// A registered callback with its unique ID.
struct HookCallback {
    id: HookId,
    callback: Rc<HookCallbackFn>,
}

/// Registry mapping hook names to their registered callbacks.
pub struct HookRegistry {
    hooks: HashMap<String, Vec<HookCallback>>,
    next_id: usize,
}

impl HookRegistry {
    /// Creates an empty hook registry.
    pub fn new() -> Self {
        HookRegistry {
            hooks: HashMap::new(),
            next_id: 0,
        }
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Registers a callback for a named hook. Returns a unique HookId
/// that can be used to unregister the callback later.
pub fn register_hook(
    registry: &mut HookRegistry,
    hook_name: &str,
    callback: Rc<HookCallbackFn>,
) -> HookId {
    let id = HookId(registry.next_id);
    registry.next_id += 1;
    let entry = registry.hooks.entry(hook_name.to_string()).or_default();
    entry.push(HookCallback { id, callback });
    id
}

/// Dispatches a named hook, calling all registered callbacks with the
/// given arguments. Returns a Vec of results from each callback.
/// If the hook name has no registered callbacks, returns an empty Vec.
pub fn dispatch_hook(
    registry: &HookRegistry,
    hook_name: &str,
    args: &[String],
) -> Vec<Vec<String>> {
    registry
        .hooks
        .get(hook_name)
        .map(|callbacks| callbacks.iter().map(|cb| (cb.callback)(args)).collect())
        .unwrap_or_default()
}

/// Returns cloned Rc pointers to all callbacks registered for a hook.
///
/// This allows the caller to release the borrow on HookRegistry before
/// executing the callbacks, avoiding RefCell conflicts when callbacks
/// mutate EditorState (e.g., via Lisp primitives like `(message ...)`).
pub fn get_callbacks(registry: &HookRegistry, hook_name: &str) -> Vec<Rc<HookCallbackFn>> {
    registry
        .hooks
        .get(hook_name)
        .map(|callbacks| callbacks.iter().map(|cb| cb.callback.clone()).collect())
        .unwrap_or_default()
}

/// Removes a specific callback by its HookId from a named hook.
/// If the hook name or ID is not found, this is a no-op.
pub fn unregister_hook(registry: &mut HookRegistry, hook_name: &str, id: HookId) {
    if let Some(callbacks) = registry.hooks.get_mut(hook_name) {
        callbacks.retain(|cb| cb.id != id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    // -----------------------------------------------------------------------
    // Acceptance test: register a hook, dispatch it, verify callback behavior
    // -----------------------------------------------------------------------

    #[test]
    fn given_hook_registry_when_callback_registered_and_dispatched_then_callback_called_with_args_and_returns_values(
    ) {
        // Given: an empty HookRegistry
        let mut registry = HookRegistry::new();

        // And: a callback that receives args and returns transformed values
        let callback = Rc::new(|args: &[String]| -> Vec<String> {
            args.iter().map(|a| format!("processed:{a}")).collect()
        });

        // When: the callback is registered for "render-gutter-hook"
        let hook_id = register_hook(&mut registry, "render-gutter-hook", callback);

        // Then: dispatching the hook with arguments returns the callback's results
        let args = vec!["line1".to_string(), "line2".to_string()];
        let results = dispatch_hook(&registry, "render-gutter-hook", &args);

        assert_eq!(
            results.len(),
            1,
            "one callback registered, one result expected"
        );
        assert_eq!(
            results[0],
            vec!["processed:line1".to_string(), "processed:line2".to_string()]
        );

        // And: the returned HookId is valid (can be used for unregistration)
        assert_eq!(hook_id, HookId(0));
    }

    // -----------------------------------------------------------------------
    // Unit tests: HookRegistry behaviors
    // Test Budget: 5 behaviors x 2 = 10 max
    // -----------------------------------------------------------------------

    #[test]
    fn given_multiple_callbacks_when_dispatched_then_returns_all_results_in_registration_order() {
        let mut registry = HookRegistry::new();

        let cb1 = Rc::new(|_args: &[String]| -> Vec<String> { vec!["from_cb1".to_string()] });
        let cb2 = Rc::new(|_args: &[String]| -> Vec<String> { vec!["from_cb2".to_string()] });

        register_hook(&mut registry, "on-save", cb1);
        register_hook(&mut registry, "on-save", cb2);

        let results = dispatch_hook(&registry, "on-save", &[]);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], vec!["from_cb1".to_string()]);
        assert_eq!(results[1], vec!["from_cb2".to_string()]);
    }

    #[test]
    fn given_unknown_hook_when_dispatched_then_returns_empty() {
        let registry = HookRegistry::new();

        let results = dispatch_hook(&registry, "nonexistent-hook", &["arg".to_string()]);
        assert!(results.is_empty());
    }

    #[test]
    fn given_registered_callback_when_unregistered_by_id_then_dispatch_no_longer_calls_it() {
        let mut registry = HookRegistry::new();

        let call_count = Rc::new(RefCell::new(0));
        let count_clone = call_count.clone();
        let callback = Rc::new(move |_args: &[String]| -> Vec<String> {
            *count_clone.borrow_mut() += 1;
            vec!["called".to_string()]
        });

        let hook_id = register_hook(&mut registry, "on-open", callback);

        // Dispatch once -- callback is called
        dispatch_hook(&registry, "on-open", &[]);
        assert_eq!(*call_count.borrow(), 1);

        // Unregister, then dispatch again -- callback is NOT called
        unregister_hook(&mut registry, "on-open", hook_id);
        let results = dispatch_hook(&registry, "on-open", &[]);
        assert!(results.is_empty());
        assert_eq!(
            *call_count.borrow(),
            1,
            "callback should not be called after unregister"
        );
    }

    #[test]
    fn given_two_callbacks_when_one_unregistered_then_other_still_dispatches() {
        let mut registry = HookRegistry::new();

        let cb1 = Rc::new(|_args: &[String]| -> Vec<String> { vec!["survivor".to_string()] });
        let cb2 = Rc::new(|_args: &[String]| -> Vec<String> { vec!["removed".to_string()] });

        let _id1 = register_hook(&mut registry, "on-close", cb1);
        let id2 = register_hook(&mut registry, "on-close", cb2);

        unregister_hook(&mut registry, "on-close", id2);

        let results = dispatch_hook(&registry, "on-close", &[]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], vec!["survivor".to_string()]);
    }

    #[test]
    fn given_callbacks_on_different_hooks_when_dispatched_then_only_matching_hook_callbacks_run() {
        let mut registry = HookRegistry::new();

        let cb_save = Rc::new(|_args: &[String]| -> Vec<String> { vec!["saved".to_string()] });
        let cb_open = Rc::new(|_args: &[String]| -> Vec<String> { vec!["opened".to_string()] });

        register_hook(&mut registry, "on-save", cb_save);
        register_hook(&mut registry, "on-open", cb_open);

        let save_results = dispatch_hook(&registry, "on-save", &[]);
        assert_eq!(save_results.len(), 1);
        assert_eq!(save_results[0], vec!["saved".to_string()]);

        let open_results = dispatch_hook(&registry, "on-open", &[]);
        assert_eq!(open_results.len(), 1);
        assert_eq!(open_results[0], vec!["opened".to_string()]);
    }

    #[test]
    fn given_register_hook_then_each_call_returns_unique_incrementing_id() {
        let mut registry = HookRegistry::new();

        let cb = Rc::new(|_args: &[String]| -> Vec<String> { vec![] });

        let id1 = register_hook(&mut registry, "hook-a", cb.clone());
        let id2 = register_hook(&mut registry, "hook-a", cb.clone());
        let id3 = register_hook(&mut registry, "hook-b", cb);

        assert_eq!(id1, HookId(0));
        assert_eq!(id2, HookId(1));
        assert_eq!(id3, HookId(2));
    }
}
