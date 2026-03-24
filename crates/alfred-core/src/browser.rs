//! Browser: pure domain types and functions for the folder browser.
//!
//! This module contains the data types and pure transformation functions
//! for Alfred's directory browser feature. All types are pure data with
//! no IO dependencies — filesystem operations happen in alfred-bin (startup)
//! and alfred-tui (event loop).

use std::path::PathBuf;

/// The mode name for browse mode.
pub const MODE_BROWSE: &str = "browse";

/// Discriminates the type of a filesystem entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryKind {
    /// The synthetic `../` entry for navigating to the parent directory.
    ParentDir,
    /// A directory entry, displayed with trailing `/`.
    Directory,
    /// A regular file entry.
    File,
    /// A symbolic link. `target_kind` indicates what it resolves to.
    Symlink { target_is_dir: bool },
}

/// A single entry in a directory listing. Immutable value type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    pub name: String,
    pub kind: EntryKind,
    pub is_hidden: bool,
}

/// A stack entry recording where the user was before entering a subdirectory.
#[derive(Debug, Clone)]
pub struct NavigationEntry {
    pub dir: PathBuf,
    pub cursor_index: usize,
}

/// The aggregate state of the folder browser.
#[derive(Debug, Clone)]
pub struct BrowserState {
    pub root_dir: PathBuf,
    pub current_dir: PathBuf,
    pub entries: Vec<DirEntry>,
    pub cursor_index: usize,
    pub scroll_offset: usize,
    pub navigation_history: Vec<NavigationEntry>,
    pub show_hidden: bool,
}

/// Returns the display name for an entry.
pub fn display_name(entry: &DirEntry) -> String {
    match &entry.kind {
        EntryKind::ParentDir => "../".to_string(),
        EntryKind::Directory => format!("{}/", entry.name),
        EntryKind::File => entry.name.clone(),
        EntryKind::Symlink { target_is_dir } => {
            if *target_is_dir {
                format!("{}/", entry.name)
            } else {
                entry.name.clone()
            }
        }
    }
}

/// Returns true if the entry is a directory (or symlink to directory, or parent dir).
pub fn is_directory_entry(entry: &DirEntry) -> bool {
    matches!(
        entry.kind,
        EntryKind::Directory
            | EntryKind::ParentDir
            | EntryKind::Symlink {
                target_is_dir: true
            }
    )
}

/// Sorts entries: ParentDir first, then directories alphabetically, then files alphabetically.
/// Case-insensitive sorting within each group.
pub fn sort_entries(entries: &mut [DirEntry]) {
    entries.sort_by(|a, b| {
        let order_a = sort_key(&a.kind);
        let order_b = sort_key(&b.kind);
        order_a
            .cmp(&order_b)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
}

/// Returns a sort key for entry kind ordering.
fn sort_key(kind: &EntryKind) -> u8 {
    match kind {
        EntryKind::ParentDir => 0,
        EntryKind::Directory
        | EntryKind::Symlink {
            target_is_dir: true,
        } => 1,
        EntryKind::File
        | EntryKind::Symlink {
            target_is_dir: false,
        } => 2,
    }
}

/// Creates a new BrowserState for the given directory with pre-read entries.
pub fn new_browser_state(
    root_dir: PathBuf,
    current_dir: PathBuf,
    mut entries: Vec<DirEntry>,
) -> BrowserState {
    // Add parent dir entry if not at root
    if current_dir.parent().is_some() {
        entries.push(DirEntry {
            name: "..".to_string(),
            kind: EntryKind::ParentDir,
            is_hidden: false,
        });
    }

    sort_entries(&mut entries);

    BrowserState {
        root_dir,
        current_dir,
        entries,
        cursor_index: 0,
        scroll_offset: 0,
        navigation_history: Vec::new(),
        show_hidden: false,
    }
}

/// Moves the browser cursor down by one, clamped to the last entry.
pub fn cursor_down(state: &mut BrowserState) {
    if !state.entries.is_empty() && state.cursor_index < state.entries.len() - 1 {
        state.cursor_index += 1;
    }
}

/// Moves the browser cursor up by one, clamped to 0.
pub fn cursor_up(state: &mut BrowserState) {
    if state.cursor_index > 0 {
        state.cursor_index -= 1;
    }
}

/// Jumps the cursor to the first entry.
pub fn jump_first(state: &mut BrowserState) {
    state.cursor_index = 0;
}

/// Jumps the cursor to the last entry.
pub fn jump_last(state: &mut BrowserState) {
    if !state.entries.is_empty() {
        state.cursor_index = state.entries.len() - 1;
    }
}

/// Returns the entry at the current cursor position, if any.
pub fn current_entry(state: &BrowserState) -> Option<&DirEntry> {
    state.entries.get(state.cursor_index)
}

/// Returns the full path of the entry at the current cursor position.
pub fn current_entry_path(state: &BrowserState) -> Option<PathBuf> {
    current_entry(state).map(|entry| match entry.kind {
        EntryKind::ParentDir => state
            .current_dir
            .parent()
            .unwrap_or(&state.current_dir)
            .to_path_buf(),
        _ => state.current_dir.join(&entry.name),
    })
}

/// Updates the scroll offset to ensure the cursor is visible within the viewport.
pub fn ensure_cursor_visible(state: &mut BrowserState, visible_height: usize) {
    if visible_height == 0 {
        return;
    }
    if state.cursor_index < state.scroll_offset {
        state.scroll_offset = state.cursor_index;
    }
    if state.cursor_index >= state.scroll_offset + visible_height {
        state.scroll_offset = state.cursor_index - visible_height + 1;
    }
}

/// Navigates into a subdirectory. Pushes current state onto navigation history.
/// The caller must provide the new entries (read from filesystem).
pub fn enter_directory(state: &mut BrowserState, new_dir: PathBuf, mut new_entries: Vec<DirEntry>) {
    // Push current position onto history
    state.navigation_history.push(NavigationEntry {
        dir: state.current_dir.clone(),
        cursor_index: state.cursor_index,
    });

    // Add parent dir entry
    if new_dir.parent().is_some() {
        new_entries.push(DirEntry {
            name: "..".to_string(),
            kind: EntryKind::ParentDir,
            is_hidden: false,
        });
    }

    sort_entries(&mut new_entries);

    state.current_dir = new_dir;
    state.entries = new_entries;
    state.cursor_index = 0;
    state.scroll_offset = 0;
}

/// Navigates to the parent directory. Restores cursor from navigation history.
/// The caller must provide the parent entries (read from filesystem).
pub fn go_to_parent(
    state: &mut BrowserState,
    parent_dir: PathBuf,
    mut parent_entries: Vec<DirEntry>,
) {
    let restored_cursor = state
        .navigation_history
        .pop()
        .map(|entry| entry.cursor_index);

    // Add parent dir entry
    if parent_dir.parent().is_some() {
        parent_entries.push(DirEntry {
            name: "..".to_string(),
            kind: EntryKind::ParentDir,
            is_hidden: false,
        });
    }

    sort_entries(&mut parent_entries);

    let cursor = restored_cursor
        .unwrap_or(0)
        .min(parent_entries.len().saturating_sub(1));

    state.current_dir = parent_dir;
    state.entries = parent_entries;
    state.cursor_index = cursor;
    state.scroll_offset = 0;
}

/// Returns the visible entries for rendering, based on scroll_offset and visible_height.
pub fn visible_entries(state: &BrowserState, visible_height: usize) -> &[DirEntry] {
    let start = state.scroll_offset;
    let end = (start + visible_height).min(state.entries.len());
    &state.entries[start..end]
}

/// Returns the relative path of current_dir from root_dir for display.
pub fn display_path(state: &BrowserState) -> String {
    state
        .current_dir
        .strip_prefix(&state.root_dir)
        .unwrap_or(&state.current_dir)
        .to_string_lossy()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file_entry(name: &str) -> DirEntry {
        DirEntry {
            name: name.to_string(),
            kind: EntryKind::File,
            is_hidden: name.starts_with('.'),
        }
    }

    fn dir_entry(name: &str) -> DirEntry {
        DirEntry {
            name: name.to_string(),
            kind: EntryKind::Directory,
            is_hidden: name.starts_with('.'),
        }
    }

    // -----------------------------------------------------------------------
    // display_name
    // -----------------------------------------------------------------------

    #[test]
    fn given_file_entry_when_display_name_then_returns_name() {
        assert_eq!(display_name(&file_entry("main.rs")), "main.rs");
    }

    #[test]
    fn given_dir_entry_when_display_name_then_returns_name_with_slash() {
        assert_eq!(display_name(&dir_entry("src")), "src/");
    }

    #[test]
    fn given_parent_dir_when_display_name_then_returns_dot_dot_slash() {
        let entry = DirEntry {
            name: "..".to_string(),
            kind: EntryKind::ParentDir,
            is_hidden: false,
        };
        assert_eq!(display_name(&entry), "../");
    }

    // -----------------------------------------------------------------------
    // sort_entries
    // -----------------------------------------------------------------------

    #[test]
    fn given_mixed_entries_when_sorted_then_parent_first_dirs_then_files() {
        let mut entries = vec![
            file_entry("zebra.txt"),
            dir_entry("src"),
            file_entry("alpha.rs"),
            dir_entry("docs"),
            DirEntry {
                name: "..".to_string(),
                kind: EntryKind::ParentDir,
                is_hidden: false,
            },
        ];
        sort_entries(&mut entries);

        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["..", "docs", "src", "alpha.rs", "zebra.txt"]);
    }

    #[test]
    fn given_case_mixed_entries_when_sorted_then_case_insensitive() {
        let mut entries = vec![
            file_entry("Readme.md"),
            file_entry("cargo.toml"),
            file_entry("Build.rs"),
        ];
        sort_entries(&mut entries);

        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["Build.rs", "cargo.toml", "Readme.md"]);
    }

    // -----------------------------------------------------------------------
    // cursor movement
    // -----------------------------------------------------------------------

    #[test]
    fn given_browser_at_first_when_cursor_down_then_moves_to_second() {
        let mut state = new_browser_state(
            PathBuf::from("/tmp"),
            PathBuf::from("/tmp"),
            vec![file_entry("a.txt"), file_entry("b.txt")],
        );
        state.cursor_index = 0;
        cursor_down(&mut state);
        assert_eq!(state.cursor_index, 1);
    }

    #[test]
    fn given_browser_at_last_when_cursor_down_then_stays() {
        let mut state = new_browser_state(
            PathBuf::from("/tmp"),
            PathBuf::from("/tmp"),
            vec![file_entry("a.txt")],
        );
        // entries: [../, a.txt] -> last index is 1
        let last = state.entries.len() - 1;
        state.cursor_index = last;
        cursor_down(&mut state);
        assert_eq!(state.cursor_index, last);
    }

    #[test]
    fn given_browser_at_second_when_cursor_up_then_moves_to_first() {
        let mut state = new_browser_state(
            PathBuf::from("/tmp"),
            PathBuf::from("/tmp"),
            vec![file_entry("a.txt"), file_entry("b.txt")],
        );
        state.cursor_index = 1;
        cursor_up(&mut state);
        assert_eq!(state.cursor_index, 0);
    }

    #[test]
    fn given_browser_at_first_when_cursor_up_then_stays() {
        let mut state = new_browser_state(
            PathBuf::from("/tmp"),
            PathBuf::from("/tmp"),
            vec![file_entry("a.txt")],
        );
        cursor_up(&mut state);
        assert_eq!(state.cursor_index, 0);
    }

    #[test]
    fn given_browser_when_jump_last_then_cursor_at_end() {
        let mut state = new_browser_state(
            PathBuf::from("/tmp"),
            PathBuf::from("/tmp"),
            vec![
                file_entry("a.txt"),
                file_entry("b.txt"),
                file_entry("c.txt"),
            ],
        );
        jump_last(&mut state);
        // entries: [../, a.txt, b.txt, c.txt] -> last index is 3
        assert_eq!(state.cursor_index, state.entries.len() - 1);
    }

    #[test]
    fn given_browser_at_end_when_jump_first_then_cursor_at_start() {
        let mut state = new_browser_state(
            PathBuf::from("/tmp"),
            PathBuf::from("/tmp"),
            vec![file_entry("a.txt"), file_entry("b.txt")],
        );
        state.cursor_index = 1;
        jump_first(&mut state);
        assert_eq!(state.cursor_index, 0);
    }

    // -----------------------------------------------------------------------
    // scroll visibility
    // -----------------------------------------------------------------------

    #[test]
    fn given_cursor_below_viewport_when_ensure_visible_then_scrolls_down() {
        let mut state = new_browser_state(
            PathBuf::from("/tmp"),
            PathBuf::from("/tmp"),
            (0..20)
                .map(|i| file_entry(&format!("file{}.txt", i)))
                .collect(),
        );
        state.cursor_index = 15;
        state.scroll_offset = 0;
        ensure_cursor_visible(&mut state, 10);
        assert!(state.scroll_offset > 0);
        assert!(state.cursor_index < state.scroll_offset + 10);
    }

    #[test]
    fn given_cursor_above_viewport_when_ensure_visible_then_scrolls_up() {
        let mut state = new_browser_state(
            PathBuf::from("/tmp"),
            PathBuf::from("/tmp"),
            (0..20)
                .map(|i| file_entry(&format!("file{}.txt", i)))
                .collect(),
        );
        state.cursor_index = 2;
        state.scroll_offset = 10;
        ensure_cursor_visible(&mut state, 10);
        assert_eq!(state.scroll_offset, 2);
    }

    // -----------------------------------------------------------------------
    // navigation
    // -----------------------------------------------------------------------

    #[test]
    fn given_browser_when_enter_directory_then_history_pushed_and_entries_updated() {
        let mut state = new_browser_state(
            PathBuf::from("/project"),
            PathBuf::from("/project"),
            vec![dir_entry("src"), file_entry("main.rs")],
        );
        state.cursor_index = 0; // on src/

        let new_entries = vec![file_entry("lib.rs"), file_entry("main.rs")];
        enter_directory(&mut state, PathBuf::from("/project/src"), new_entries);

        assert_eq!(state.current_dir, PathBuf::from("/project/src"));
        assert_eq!(state.cursor_index, 0);
        assert_eq!(state.navigation_history.len(), 1);
        assert_eq!(state.navigation_history[0].dir, PathBuf::from("/project"));
    }

    #[test]
    fn given_browser_in_subdir_when_go_to_parent_then_history_popped_and_cursor_restored() {
        let mut state = new_browser_state(
            PathBuf::from("/project"),
            PathBuf::from("/project/src"),
            vec![file_entry("lib.rs")],
        );
        state.navigation_history.push(NavigationEntry {
            dir: PathBuf::from("/project"),
            cursor_index: 2,
        });

        let parent_entries = vec![dir_entry("src"), dir_entry("docs"), file_entry("main.rs")];
        go_to_parent(&mut state, PathBuf::from("/project"), parent_entries);

        assert_eq!(state.current_dir, PathBuf::from("/project"));
        assert_eq!(state.cursor_index, 2); // restored
        assert!(state.navigation_history.is_empty());
    }

    // -----------------------------------------------------------------------
    // current_entry / current_entry_path
    // -----------------------------------------------------------------------

    #[test]
    fn given_browser_with_entries_when_current_entry_at_file_then_returns_file() {
        let mut state = new_browser_state(
            PathBuf::from("/tmp"),
            PathBuf::from("/tmp"),
            vec![file_entry("hello.txt")],
        );
        // entries: [../, hello.txt] -> file is at index 1
        state.cursor_index = 1;
        let entry = current_entry(&state).unwrap();
        assert_eq!(entry.name, "hello.txt");
    }

    #[test]
    fn given_browser_on_file_when_current_entry_path_then_returns_full_path() {
        let mut state = new_browser_state(
            PathBuf::from("/tmp"),
            PathBuf::from("/tmp"),
            vec![file_entry("hello.txt")],
        );
        // Navigate cursor to the file entry (past ../)
        state.cursor_index = 1;
        let path = current_entry_path(&state).unwrap();
        assert_eq!(path, PathBuf::from("/tmp/hello.txt"));
    }

    #[test]
    fn given_browser_on_parent_dir_when_current_entry_path_then_returns_parent() {
        let state = new_browser_state(
            PathBuf::from("/tmp"),
            PathBuf::from("/tmp"),
            vec![file_entry("hello.txt")],
        );
        // cursor at 0 = ../ entry
        let path = current_entry_path(&state).unwrap();
        assert_eq!(path, PathBuf::from("/"));
    }

    #[test]
    fn given_root_dir_browser_when_no_parent_entry_then_no_dot_dot() {
        let state = new_browser_state(
            PathBuf::from("/"),
            PathBuf::from("/"),
            vec![file_entry("hello.txt")],
        );
        // root dir has no parent, so no ../ entry
        let entry = current_entry(&state).unwrap();
        assert_eq!(entry.name, "hello.txt");
    }

    // -----------------------------------------------------------------------
    // display_path
    // -----------------------------------------------------------------------

    #[test]
    fn given_browser_at_root_when_display_path_then_returns_empty() {
        let state = new_browser_state(PathBuf::from("/project"), PathBuf::from("/project"), vec![]);
        assert_eq!(display_path(&state), "");
    }

    #[test]
    fn given_browser_in_subdir_when_display_path_then_returns_relative() {
        let mut state =
            new_browser_state(PathBuf::from("/project"), PathBuf::from("/project"), vec![]);
        state.current_dir = PathBuf::from("/project/src/core");
        assert_eq!(display_path(&state), "src/core");
    }

    // -----------------------------------------------------------------------
    // is_directory_entry
    // -----------------------------------------------------------------------

    #[test]
    fn given_directory_entry_when_is_directory_then_returns_true() {
        assert!(is_directory_entry(&dir_entry("src")));
    }

    #[test]
    fn given_file_entry_when_is_directory_then_returns_false() {
        assert!(!is_directory_entry(&file_entry("main.rs")));
    }

    #[test]
    fn given_parent_dir_when_is_directory_then_returns_true() {
        let entry = DirEntry {
            name: "..".to_string(),
            kind: EntryKind::ParentDir,
            is_hidden: false,
        };
        assert!(is_directory_entry(&entry));
    }

    #[test]
    fn given_browser_with_multiple_entries_when_visible_entries_then_returns_all() {
        let state = new_browser_state(
            PathBuf::from("/tmp/test"),
            PathBuf::from("/tmp/test"),
            vec![
                dir_entry("alpha_dir"),
                dir_entry("beta_dir"),
                file_entry("delta.rs"),
                file_entry("gamma.txt"),
            ],
        );
        assert_eq!(
            state.entries.len(),
            5,
            "Expected 5 entries (../ + 2 dirs + 2 files)"
        );
        let visible = visible_entries(&state, 20);
        assert_eq!(
            visible.len(),
            5,
            "All 5 entries should be visible in 20-row viewport"
        );
    }
}
