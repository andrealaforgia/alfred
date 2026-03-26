;;; name: browse-mode
;;; version: 2.2.0
;;; description: Pure Lisp folder browser — renders directory listing as buffer text
;;; depends: vim-keybindings

;; ---------------------------------------------------------------------------
;; Keymap
;; ---------------------------------------------------------------------------

(make-keymap "browse-mode")

(define-key "browse-mode" "Char:j" "browser-cursor-down")
(define-key "browse-mode" "Char:k" "browser-cursor-up")
(define-key "browse-mode" "Down" "browser-cursor-down")
(define-key "browse-mode" "Up" "browser-cursor-up")
(define-key "browse-mode" "Char:g" "browser-jump-first")
(define-key "browse-mode" "Char:G" "browser-jump-last")
(define-key "browse-mode" "Enter" "browser-enter")
(define-key "browse-mode" "Char:l" "browser-enter")
(define-key "browse-mode" "Char:h" "browser-parent")
(define-key "browse-mode" "Backspace" "browser-parent")
(define-key "browse-mode" "Char:q" "browser-quit")
(define-key "browse-mode" "Ctrl:e" "toggle-sidebar")
(define-key "browse-mode" "Ctrl:b" "browse")

(set-cursor-shape "browse" "block")

;; ---------------------------------------------------------------------------
;; Browser state
;; ---------------------------------------------------------------------------

(define browser-empty-str (str-concat (list)))
(define browser-current-dir browser-empty-str)
(define browser-root-dir browser-empty-str)
(define browser-entries (list))
(define browser-cursor 0)
(define browser-history (list))

;; Browser search/filter state
(define browser-search-active nil)
(define browser-search-query (str-concat (list)))
(define browser-pre-search-cursor 0)
(define browser-filtered-entries (list))

;; ---------------------------------------------------------------------------
;; Helpers — no local (define) inside lambdas, use args or inline
;; ---------------------------------------------------------------------------

;; Check if entry name contains query (case-insensitive substring match)
(define browser-entry-matches
  (lambda (entry query)
    (str-contains (str-lower (first entry)) (str-lower query))))

;; Filter entries by query — returns only entries whose name contains query
(define browser-filter-entries
  (lambda (entries query)
    (if (= (str-length query) 0)
      entries
      (filter (lambda (e) (browser-entry-matches e query)) entries))))

;; Format entry: prefix + name + suffix
(define browser-format-entry
  (lambda (entry idx)
    (str-concat
      (list
        (if (= idx browser-cursor) " > " "   ")
        (first entry)
        (if (= (nth 1 entry) "dir") "/" browser-empty-str)))))

;; Recursive line builder
(define browser-build-lines
  (lambda (entries idx)
    (if (= (length entries) 0)
      browser-empty-str
      (if (= (length entries) 1)
        (browser-format-entry (first entries) idx)
        (str-concat
          (list
            (browser-format-entry (first entries) idx)
            newline
            (browser-build-lines (rest entries) (+ idx 1))))))))

;; ---------------------------------------------------------------------------
;; Colors
;; ---------------------------------------------------------------------------

(define browser-color-blue "#89b4fa")
(define browser-color-pink "#f5c2e7")
(define browser-color-gray "#cdd6f4")

;; ---------------------------------------------------------------------------
;; Style: apply per-line colors after buffer-set-content
;; ---------------------------------------------------------------------------

;; Style a single entry line (line-num is the buffer line, idx is the entry index)
(define browser-color-cursor-fg "#1e1e2e")
(define browser-color-cursor-bg "#cdd6f4")

(define browser-style-entry
  (lambda (entry idx line-num)
    (if (= idx browser-cursor)
      (set-line-background line-num browser-color-cursor-fg browser-color-cursor-bg)
      (if (= (nth 1 entry) "dir")
        (set-line-style line-num 0 (str-length (buffer-get-line line-num)) browser-color-blue)
        (set-line-style line-num 0 (str-length (buffer-get-line line-num)) browser-color-gray)))))

;; Recursively style all entry lines; entries start at buffer line 2
(define browser-style-entries
  (lambda (entries idx)
    (if (= (length entries) 0)
      nil
      (begin
        (browser-style-entry (first entries) idx (+ idx 2))
        (if (> (length entries) 1)
          (browser-style-entries (rest entries) (+ idx 1))
          nil)))))

;; Return the entries currently being displayed (filtered or full)
(define browser-display-entries
  (lambda ()
    (if browser-search-active
      browser-filtered-entries
      browser-entries)))

;; Apply all line styles for the browser view
(define browser-apply-styles
  (lambda ()
    (clear-line-styles)
    (clear-line-backgrounds)
    (set-line-style 0 0 (str-length (buffer-get-line 0))
      (if browser-search-active browser-color-pink browser-color-blue))
    (if (> (length (browser-display-entries)) 0)
      (browser-style-entries (browser-display-entries) 0)
      nil)))

;; ---------------------------------------------------------------------------
;; Render: rebuild buffer text from current state
;; ---------------------------------------------------------------------------

;; Header line: shows search prompt when filtering, directory path otherwise
(define browser-header-line
  (lambda ()
    (if browser-search-active
      (str-concat (list " / " browser-search-query))
      (str-concat (list " " browser-current-dir)))))

(define browser-render
  (lambda ()
    (buffer-set-content
      (str-concat
        (list
          (browser-header-line) newline
          newline
          (if (= (length (browser-display-entries)) 0)
            (if browser-search-active
              "   (no matches)"
              "   (empty directory)")
            (browser-build-lines (browser-display-entries) 0)))))
    (browser-apply-styles)))

;; ---------------------------------------------------------------------------
;; Load entries for a directory
;; ---------------------------------------------------------------------------

(define browser-add-parent-entry
  (lambda (dir entries)
    (if (= dir browser-root-dir)
      entries
      (if (= (path-parent dir) dir)
        entries
        (cons (list ".." "dir") entries)))))

(define browser-load-dir
  (lambda (dir)
    (set browser-current-dir dir)
    (set browser-entries (browser-add-parent-entry dir (list-dir dir)))
    (set browser-cursor 0)
    (browser-render)))

;; ---------------------------------------------------------------------------
;; Commands
;; ---------------------------------------------------------------------------

(define-command "browser-cursor-down"
  (lambda ()
    (if (< browser-cursor (- (length (browser-display-entries)) 1))
      (set browser-cursor (+ browser-cursor 1))
      nil)
    (browser-render)))

(define-command "browser-cursor-up"
  (lambda ()
    (if (> browser-cursor 0)
      (set browser-cursor (- browser-cursor 1))
      nil)
    (browser-render)))

(define-command "browser-jump-first"
  (lambda ()
    (set browser-cursor 0)
    (browser-render)))

(define-command "browser-jump-last"
  (lambda ()
    (if (> (length (browser-display-entries)) 0)
      (set browser-cursor (- (length (browser-display-entries)) 1))
      nil)
    (browser-render)))

;; Helper to get the currently selected entry (works in both normal and search mode)
(define browser-selected-entry
  (lambda ()
    (nth browser-cursor (browser-display-entries))))

(define-command "browser-enter"
  (lambda ()
    (if (= (length (browser-display-entries)) 0)
      nil
      (if (= (nth 1 (browser-selected-entry)) "dir")
        (if (= (first (browser-selected-entry)) "..")
          (begin
            (if browser-search-active (browser-search-dismiss) nil)
            (browser-do-parent))
          (begin
            (if browser-search-active (browser-search-dismiss) nil)
            (browser-do-enter-dir (first (browser-selected-entry)))))
        (begin
          (if browser-search-active (browser-search-dismiss) nil)
          (open-file
            (path-join browser-current-dir
              (first (browser-selected-entry)))))))))

(define browser-do-enter-dir
  (lambda (name)
    (set browser-history
      (cons (list browser-current-dir browser-cursor) browser-history))
    (browser-load-dir (path-join browser-current-dir name))))

(define browser-do-parent
  (lambda ()
    (set browser-history
      (cons (list browser-current-dir browser-cursor) browser-history))
    (browser-load-dir (path-parent browser-current-dir))))

(define-command "browser-parent"
  (lambda ()
    (if (= (path-parent browser-current-dir) browser-current-dir)
      nil
      (browser-do-go-parent))))

(define browser-do-go-parent
  (lambda ()
    (if (> (length browser-history) 0)
      (set browser-cursor (nth 1 (first browser-history)))
      nil)
    (if (> (length browser-history) 0)
      (set browser-history (rest browser-history))
      nil)
    (set browser-current-dir (path-parent browser-current-dir))
    (set browser-entries
      (browser-add-parent-entry browser-current-dir (list-dir browser-current-dir)))
    (if (> browser-cursor (- (length browser-entries) 1))
      (set browser-cursor (- (length browser-entries) 1))
      nil)
    (browser-render)))

(define-command "browser-quit"
  (lambda () (quit)))

;; ---------------------------------------------------------------------------
;; Browser search/filter commands
;; ---------------------------------------------------------------------------

;; Dismiss search: clear search state, stay on current entries
(define browser-search-dismiss
  (lambda ()
    (set browser-search-active nil)
    (set browser-search-query (str-concat (list)))
    (set browser-filtered-entries (list))))

;; Start search: save cursor, activate search mode, switch keymap
(define-command "browser-start-search"
  (lambda ()
    (set browser-pre-search-cursor browser-cursor)
    (set browser-search-active 1)
    (set browser-search-query (str-concat (list)))
    (set browser-filtered-entries browser-entries)
    (set browser-cursor 0)
    (set-active-keymap "browser-search-mode")
    (browser-render)))

;; Apply filter: update filtered entries and reset cursor
(define browser-search-apply-filter
  (lambda ()
    (set browser-filtered-entries
      (browser-filter-entries browser-entries browser-search-query))
    (set browser-cursor 0)
    (browser-render)))

;; Append a character to the search query and re-filter
(define browser-search-append-char
  (lambda (ch)
    (set browser-search-query (str-concat (list browser-search-query ch)))
    (browser-search-apply-filter)))

;; Backspace: remove last character or cancel if query is empty
(define-command "browser-search-backspace"
  (lambda ()
    (if (= (str-length browser-search-query) 0)
      (browser-search-do-cancel)
      (begin
        (set browser-search-query
          (str-substring browser-search-query 0
            (- (str-length browser-search-query) 1)))
        (browser-search-apply-filter)))))

;; Cancel search: restore pre-search cursor and return to browse keymap
(define browser-search-do-cancel
  (lambda ()
    (set browser-cursor browser-pre-search-cursor)
    (browser-search-dismiss)
    (set-active-keymap "browse-mode")
    (browser-render)))

(define-command "browser-search-cancel"
  (lambda () (browser-search-do-cancel)))

;; Enter from search: open selected entry (browser-enter handles search dismiss)
(define-command "browser-search-enter"
  (lambda ()
    (if (= (length (browser-display-entries)) 0)
      nil
      (if (= (nth 1 (browser-selected-entry)) "dir")
        (if (= (first (browser-selected-entry)) "..")
          (begin
            (browser-search-dismiss)
            (set-active-keymap "browse-mode")
            (browser-do-parent))
          (begin
            (browser-search-dismiss)
            (set-active-keymap "browse-mode")
            (browser-do-enter-dir (first (browser-selected-entry)))))
        (begin
          (browser-search-dismiss)
          (set-active-keymap "browse-mode")
          (open-file
            (path-join browser-current-dir
              (first (browser-selected-entry)))))))))

;; Cursor navigation within search results
(define-command "browser-search-cursor-down"
  (lambda ()
    (if (< browser-cursor (- (length (browser-display-entries)) 1))
      (set browser-cursor (+ browser-cursor 1))
      nil)
    (browser-render)))

(define-command "browser-search-cursor-up"
  (lambda ()
    (if (> browser-cursor 0)
      (set browser-cursor (- browser-cursor 1))
      nil)
    (browser-render)))

;; Return to browser from normal mode via :browse or Ctrl-b
(define-command "browse"
  (lambda ()
    (if (= browser-root-dir browser-empty-str)
      (message "No browse directory set")
      (begin
        (browser-load-dir browser-current-dir)
        (set-mode "browse")
        (set-active-keymap "browse-mode")))))

;; Ctrl-b in normal mode toggles to browser
(define-key "normal-mode" "Ctrl:b" "browse")

;; ---------------------------------------------------------------------------
;; Interactive file tree sidebar (left panel with focus)
;; ---------------------------------------------------------------------------

(define sidebar-visible nil)
(define sidebar-width 30)
(define sidebar-entries (list))
(define sidebar-current-dir browser-empty-str)
(define sidebar-saved-mode browser-empty-str)
(define sidebar-saved-keymaps (list))

;; Filetree keymap for when sidebar is focused
(make-keymap "filetree-mode")
(define-key "filetree-mode" "Char:j" "sidebar-cursor-down")
(define-key "filetree-mode" "Char:k" "sidebar-cursor-up")
(define-key "filetree-mode" "Down" "sidebar-cursor-down")
(define-key "filetree-mode" "Up" "sidebar-cursor-up")
(define-key "filetree-mode" "Enter" "sidebar-enter")
(define-key "filetree-mode" "Char:l" "sidebar-enter")
(define-key "filetree-mode" "Char:q" "sidebar-unfocus")
(define-key "filetree-mode" "Escape" "sidebar-unfocus")
(define-key "filetree-mode" "Ctrl:e" "toggle-sidebar")

;; Prepend ".." entry for parent navigation unless at root
(define sidebar-add-parent-entry
  (lambda (dir entries)
    (if (= dir browser-root-dir)
      entries
      (if (= (path-parent dir) dir)
        entries
        (cons (list ".." "dir") entries)))))

;; Format a sidebar entry with cursor indicator
(define sidebar-format-panel-entry
  (lambda (entry idx cursor)
    (str-concat (list
      (if (= idx cursor) " > " "   ")
      (first entry)
      (if (= (nth 1 entry) "dir") "/" browser-empty-str)))))

;; Populate sidebar panel lines from entries list (offset by header)
(define sidebar-populate-with-offset
  (lambda (entries idx cursor-entry-idx)
    (if (= idx (length entries))
      nil
      (begin
        (set-panel-line "filetree" (+ idx sidebar-header-offset)
          (sidebar-format-panel-entry (nth idx entries) idx cursor-entry-idx))
        (sidebar-populate-with-offset entries (+ idx 1) cursor-entry-idx)))))

;; Apply per-line colors to sidebar entries
(define sidebar-style-entry
  (lambda (entry idx cursor)
    (if (= idx cursor)
      (set-panel-line-style "filetree" (+ idx sidebar-header-offset) 0
        (str-length (sidebar-format-panel-entry entry idx cursor))
        browser-color-pink)
      (if (= (nth 1 entry) "dir")
        (set-panel-line-style "filetree" (+ idx sidebar-header-offset) 0
          (str-length (sidebar-format-panel-entry entry idx cursor))
          browser-color-blue)
        (set-panel-line-style "filetree" (+ idx sidebar-header-offset) 0
          (str-length (sidebar-format-panel-entry entry idx cursor))
          browser-color-gray)))))

;; Recursively style all sidebar entries
(define sidebar-style-entries
  (lambda (entries idx cursor)
    (if (= idx (length entries))
      nil
      (begin
        (sidebar-style-entry (nth idx entries) idx cursor)
        (sidebar-style-entries entries (+ idx 1) cursor)))))

;; Apply all styles to sidebar
(define sidebar-apply-styles
  (lambda ()
    (clear-panel-line-styles "filetree")
    ;; Re-style header
    (set-panel-line-style "filetree" 0 0
      (+ (str-length sidebar-current-dir) 1) browser-color-blue)
    (if (> (length sidebar-entries) 0)
      (sidebar-style-entries sidebar-entries 0
        (- (panel-cursor-line "filetree") sidebar-header-offset))
      nil)))

;; Load sidebar entries for a directory -- clears old lines and resets cursor
;; Line 0 = header (directory path), Line 1 = separator, Lines 2+ = entries
(define sidebar-header-offset 2)

(define sidebar-load
  (lambda (dir)
    (clear-panel-lines "filetree")
    (set sidebar-current-dir dir)
    (set sidebar-entries (sidebar-add-parent-entry dir (list-dir dir)))
    (panel-set-cursor "filetree" sidebar-header-offset)
    ;; Header line
    (set-panel-line "filetree" 0 (str-concat (list " " dir)))
    (set-panel-line-style "filetree" 0 0 (+ (str-length dir) 1) browser-color-blue)
    ;; Separator
    (set-panel-line "filetree" 1 (str-concat (list)))
    ;; Entries start at line 2
    (sidebar-populate-with-offset sidebar-entries 0 (- (panel-cursor-line "filetree") sidebar-header-offset))
    (sidebar-apply-styles)))

;; Toggle sidebar visibility + focus
(define sidebar-created nil)

;; Helper: focus the sidebar (shared by toggle and re-focus paths)
(define sidebar-do-focus
  (lambda ()
    (set sidebar-saved-mode (current-mode))
    (focus-panel "filetree")
    (set-mode "panel-filetree")
    (set-active-keymap "filetree-mode")))

;; Helper: open the sidebar from scratch
(define sidebar-do-open
  (lambda ()
    (set sidebar-visible 1)
    (if sidebar-created
      nil
      (begin
        (define-panel "filetree" "left" sidebar-width)
        (set-panel-priority "filetree" 10)
        (set sidebar-created 1)))
    (set-panel-style "filetree" "#6c7086" "#1e1e2e")
    (set-panel-size "filetree" sidebar-width)
    (sidebar-load
      (if (= sidebar-current-dir browser-empty-str)
        browser-root-dir
        sidebar-current-dir))
    (sidebar-do-focus)))

(define-command "toggle-sidebar"
  (lambda ()
    (if (= browser-root-dir browser-empty-str)
      (message "No browse directory set")
      (if sidebar-visible
        ;; Sidebar is visible — check if focused
        (if (= (focused-panel) "filetree")
          ;; Focused: close it
          (begin
            (set sidebar-visible nil)
            (set-panel-size "filetree" 0)
            (unfocus-panel)
            (set-mode "normal")
            (set-active-keymap "normal-mode"))
          ;; Visible but unfocused: re-focus it
          (sidebar-do-focus))
        ;; Not visible: open + focus
        (sidebar-do-open)))))

;; Sidebar commands — re-populate lines after cursor move to update ">" indicator
(define sidebar-refresh
  (lambda ()
    (sidebar-populate-with-offset sidebar-entries 0
      (- (panel-cursor-line "filetree") sidebar-header-offset))
    (sidebar-apply-styles)))

(define-command "sidebar-cursor-down"
  (lambda ()
    (panel-cursor-down "filetree")
    (sidebar-refresh)))

(define-command "sidebar-cursor-up"
  (lambda ()
    (if (> (panel-cursor-line "filetree") sidebar-header-offset)
      (panel-cursor-up "filetree")
      nil)
    (sidebar-refresh)))

;; Helper: entry index from visual cursor (subtract header offset)
(define sidebar-entry-index
  (lambda ()
    (- (panel-cursor-line "filetree") sidebar-header-offset)))

;; Helper to get current sidebar entry name
(define sidebar-current-name
  (lambda ()
    (first (nth (sidebar-entry-index) sidebar-entries))))

;; Helper to get current sidebar entry type
(define sidebar-current-type
  (lambda ()
    (nth 1 (nth (sidebar-entry-index) sidebar-entries))))

(define-command "sidebar-enter"
  (lambda ()
    (if (= (length sidebar-entries) 0)
      nil
      (if (= (sidebar-current-type) "dir")
        (if (= (sidebar-current-name) "..")
          (sidebar-load (path-parent sidebar-current-dir))
          (sidebar-load (path-join sidebar-current-dir (sidebar-current-name))))
        (begin
          (unfocus-panel)
          (set-mode "normal")
          (set-active-keymap "normal-mode")
          (clear-line-styles)
          (open-file (path-join sidebar-current-dir (sidebar-current-name))))))))

(define-command "sidebar-unfocus"
  (lambda ()
    (unfocus-panel)
    (set-mode "normal")
    (set-active-keymap "normal-mode")))

;; Ctrl-e in normal mode toggles sidebar
(define-key "normal-mode" "Ctrl:e" "toggle-sidebar")

;; ---------------------------------------------------------------------------
;; Activation: check CLI argument on load
;; ---------------------------------------------------------------------------

(define browser-cli-arg (cli-argument))

(if (= browser-cli-arg browser-empty-str)
  nil
  (if (is-dir? browser-cli-arg)
    (begin
      (set browser-root-dir browser-cli-arg)
      (set browser-current-dir browser-cli-arg)
      (browser-load-dir browser-cli-arg)
      (set-mode "browse")
      (set-active-keymap "browse-mode"))
    (begin
      (set browser-root-dir (path-parent browser-cli-arg))
      (set browser-current-dir (path-parent browser-cli-arg))
      nil)))
