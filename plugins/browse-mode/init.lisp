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
(define-key "browse-mode" "Char:/" "browser-start-search")
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

;; Project-wide search state (Ctrl-p)
(define project-search-active nil)
(define project-search-query (str-concat (list)))
(define project-search-cache (list))
(define project-search-results (list))

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

;; ---------------------------------------------------------------------------
;; Browser search keymap — per-character commands
;; ---------------------------------------------------------------------------

(make-keymap "browser-search-mode")

;; Navigation and control keys
(define-key "browser-search-mode" "Escape" "browser-search-cancel")
(define-key "browser-search-mode" "Enter" "browser-search-enter")
(define-key "browser-search-mode" "Backspace" "browser-search-backspace")
(define-key "browser-search-mode" "Down" "browser-search-cursor-down")
(define-key "browser-search-mode" "Up" "browser-search-cursor-up")

;; Per-character commands for search input: lowercase a-z
(define-command "browser-search-char-a" (lambda () (browser-search-append-char "a")))
(define-command "browser-search-char-b" (lambda () (browser-search-append-char "b")))
(define-command "browser-search-char-c" (lambda () (browser-search-append-char "c")))
(define-command "browser-search-char-d" (lambda () (browser-search-append-char "d")))
(define-command "browser-search-char-e" (lambda () (browser-search-append-char "e")))
(define-command "browser-search-char-f" (lambda () (browser-search-append-char "f")))
(define-command "browser-search-char-g" (lambda () (browser-search-append-char "g")))
(define-command "browser-search-char-h" (lambda () (browser-search-append-char "h")))
(define-command "browser-search-char-i" (lambda () (browser-search-append-char "i")))
(define-command "browser-search-char-j" (lambda () (browser-search-append-char "j")))
(define-command "browser-search-char-k" (lambda () (browser-search-append-char "k")))
(define-command "browser-search-char-l" (lambda () (browser-search-append-char "l")))
(define-command "browser-search-char-m" (lambda () (browser-search-append-char "m")))
(define-command "browser-search-char-n" (lambda () (browser-search-append-char "n")))
(define-command "browser-search-char-o" (lambda () (browser-search-append-char "o")))
(define-command "browser-search-char-p" (lambda () (browser-search-append-char "p")))
(define-command "browser-search-char-q" (lambda () (browser-search-append-char "q")))
(define-command "browser-search-char-r" (lambda () (browser-search-append-char "r")))
(define-command "browser-search-char-s" (lambda () (browser-search-append-char "s")))
(define-command "browser-search-char-t" (lambda () (browser-search-append-char "t")))
(define-command "browser-search-char-u" (lambda () (browser-search-append-char "u")))
(define-command "browser-search-char-v" (lambda () (browser-search-append-char "v")))
(define-command "browser-search-char-w" (lambda () (browser-search-append-char "w")))
(define-command "browser-search-char-x" (lambda () (browser-search-append-char "x")))
(define-command "browser-search-char-y" (lambda () (browser-search-append-char "y")))
(define-command "browser-search-char-z" (lambda () (browser-search-append-char "z")))

;; Keybindings for lowercase a-z
(define-key "browser-search-mode" "Char:a" "browser-search-char-a")
(define-key "browser-search-mode" "Char:b" "browser-search-char-b")
(define-key "browser-search-mode" "Char:c" "browser-search-char-c")
(define-key "browser-search-mode" "Char:d" "browser-search-char-d")
(define-key "browser-search-mode" "Char:e" "browser-search-char-e")
(define-key "browser-search-mode" "Char:f" "browser-search-char-f")
(define-key "browser-search-mode" "Char:g" "browser-search-char-g")
(define-key "browser-search-mode" "Char:h" "browser-search-char-h")
(define-key "browser-search-mode" "Char:i" "browser-search-char-i")
(define-key "browser-search-mode" "Char:j" "browser-search-char-j")
(define-key "browser-search-mode" "Char:k" "browser-search-char-k")
(define-key "browser-search-mode" "Char:l" "browser-search-char-l")
(define-key "browser-search-mode" "Char:m" "browser-search-char-m")
(define-key "browser-search-mode" "Char:n" "browser-search-char-n")
(define-key "browser-search-mode" "Char:o" "browser-search-char-o")
(define-key "browser-search-mode" "Char:p" "browser-search-char-p")
(define-key "browser-search-mode" "Char:q" "browser-search-char-q")
(define-key "browser-search-mode" "Char:r" "browser-search-char-r")
(define-key "browser-search-mode" "Char:s" "browser-search-char-s")
(define-key "browser-search-mode" "Char:t" "browser-search-char-t")
(define-key "browser-search-mode" "Char:u" "browser-search-char-u")
(define-key "browser-search-mode" "Char:v" "browser-search-char-v")
(define-key "browser-search-mode" "Char:w" "browser-search-char-w")
(define-key "browser-search-mode" "Char:x" "browser-search-char-x")
(define-key "browser-search-mode" "Char:y" "browser-search-char-y")
(define-key "browser-search-mode" "Char:z" "browser-search-char-z")

;; Per-character commands for digits 0-9
(define-command "browser-search-char-0" (lambda () (browser-search-append-char "0")))
(define-command "browser-search-char-1" (lambda () (browser-search-append-char "1")))
(define-command "browser-search-char-2" (lambda () (browser-search-append-char "2")))
(define-command "browser-search-char-3" (lambda () (browser-search-append-char "3")))
(define-command "browser-search-char-4" (lambda () (browser-search-append-char "4")))
(define-command "browser-search-char-5" (lambda () (browser-search-append-char "5")))
(define-command "browser-search-char-6" (lambda () (browser-search-append-char "6")))
(define-command "browser-search-char-7" (lambda () (browser-search-append-char "7")))
(define-command "browser-search-char-8" (lambda () (browser-search-append-char "8")))
(define-command "browser-search-char-9" (lambda () (browser-search-append-char "9")))

;; Keybindings for digits 0-9
(define-key "browser-search-mode" "Char:0" "browser-search-char-0")
(define-key "browser-search-mode" "Char:1" "browser-search-char-1")
(define-key "browser-search-mode" "Char:2" "browser-search-char-2")
(define-key "browser-search-mode" "Char:3" "browser-search-char-3")
(define-key "browser-search-mode" "Char:4" "browser-search-char-4")
(define-key "browser-search-mode" "Char:5" "browser-search-char-5")
(define-key "browser-search-mode" "Char:6" "browser-search-char-6")
(define-key "browser-search-mode" "Char:7" "browser-search-char-7")
(define-key "browser-search-mode" "Char:8" "browser-search-char-8")
(define-key "browser-search-mode" "Char:9" "browser-search-char-9")

;; Per-character commands for common symbols in filenames
(define-command "browser-search-char-dash" (lambda () (browser-search-append-char "-")))
(define-command "browser-search-char-underscore" (lambda () (browser-search-append-char "_")))
(define-command "browser-search-char-dot" (lambda () (browser-search-append-char ".")))
(define-command "browser-search-char-space" (lambda () (browser-search-append-char " ")))

;; Keybindings for common symbols
(define-key "browser-search-mode" "Char:-" "browser-search-char-dash")
(define-key "browser-search-mode" "Char:_" "browser-search-char-underscore")
(define-key "browser-search-mode" "Char:." "browser-search-char-dot")
(define-key "browser-search-mode" "Char: " "browser-search-char-space")

;; ---------------------------------------------------------------------------
;; Project-wide search (Ctrl-p) — searches recursively from browser-root-dir
;; ---------------------------------------------------------------------------

;; Check if a relative path contains the query (case-insensitive)
(define project-entry-matches
  (lambda (entry query)
    (str-contains (str-lower (first entry)) (str-lower query))))

;; Filter project cache entries by query
(define project-filter-entries
  (lambda (entries query)
    (if (= (str-length query) 0)
      entries
      (filter (lambda (e) (project-entry-matches e query)) entries))))

;; Header offset for project search sidebar: line 0 = header, line 1 = separator
(define project-search-header-offset 2)

;; Populate sidebar panel lines with project search results
(define project-search-populate-results
  (lambda (results idx)
    (if (= idx (length results))
      nil
      (begin
        (set-panel-line "filetree" (+ idx project-search-header-offset)
          (str-concat (list
            (if (= idx 0) " > " "   ")
            (first (nth idx results)))))
        (project-search-populate-results results (+ idx 1))))))

;; Apply styles to project search results
(define project-search-style-result
  (lambda (result idx cursor)
    (if (= idx cursor)
      (set-panel-line-style "filetree" (+ idx project-search-header-offset) 0
        (str-length (str-concat (list
          (if (= idx cursor) " > " "   ")
          (first (nth idx result)))))
        browser-color-pink)
      (set-panel-line-style "filetree" (+ idx project-search-header-offset) 0
        (str-length (str-concat (list
          (if (= idx cursor) " > " "   ")
          (first (nth idx result)))))
        browser-color-gray))))

;; Recursively style all project search results
(define project-search-style-results
  (lambda (results idx cursor)
    (if (= idx (length results))
      nil
      (begin
        (project-search-style-result results idx cursor)
        (project-search-style-results results (+ idx 1) cursor)))))

;; Render the project search sidebar
(define project-search-render
  (lambda ()
    (clear-panel-lines "filetree")
    (clear-panel-line-styles "filetree")
    ;; Header: "Search: query"
    (set-panel-line "filetree" 0
      (str-concat (list " Search: " project-search-query)))
    (set-panel-line-style "filetree" 0 0
      (+ (str-length project-search-query) 9) browser-color-blue)
    ;; Separator
    (set-panel-line "filetree" 1 (str-concat (list)))
    ;; Results
    (if (> (length project-search-results) 0)
      (begin
        (project-search-populate-results project-search-results 0)
        (panel-set-cursor "filetree" project-search-header-offset)
        (project-search-style-results project-search-results 0 0))
      nil)))

;; Apply filter and re-render
(define project-search-apply-filter
  (lambda ()
    (set project-search-results
      (project-filter-entries project-search-cache project-search-query))
    (project-search-render)))

;; Append a character to the project search query and re-filter
(define project-search-append-char
  (lambda (ch)
    (set project-search-query (str-concat (list project-search-query ch)))
    (project-search-apply-filter)))

;; Reset project search state
(define project-search-reset
  (lambda ()
    (set project-search-active nil)
    (set project-search-query (str-concat (list)))
    (set project-search-results (list))))

;; Start project search: populate cache, open sidebar, switch keymap
(define-command "project-search-start"
  (lambda ()
    (if (= browser-root-dir browser-empty-str)
      (message "No browse directory set")
      (begin
        (set project-search-cache (list-dir-recursive browser-root-dir))
        (set project-search-results project-search-cache)
        (set project-search-active 1)
        (set project-search-query (str-concat (list)))
        ;; Ensure sidebar is open
        (if sidebar-created
          nil
          (begin
            (define-panel "filetree" "left" sidebar-width)
            (set-panel-priority "filetree" 10)
            (set sidebar-created 1)))
        (set sidebar-visible 1)
        (set-panel-style "filetree" "#6c7086" "#1e1e2e")
        (set-panel-size "filetree" sidebar-width)
        (focus-panel "filetree")
        (project-search-render)
        (set-active-keymap "project-search-mode")))))

;; Cancel project search: close sidebar, return to normal mode
(define project-search-do-cancel
  (lambda ()
    (project-search-reset)
    (set sidebar-visible nil)
    (set-panel-size "filetree" 0)
    (unfocus-panel)
    (set-mode "normal")
    (set-active-keymap "normal-mode")))

(define-command "project-search-cancel"
  (lambda () (project-search-do-cancel)))

;; Backspace: remove last char or cancel if query is empty
(define-command "project-search-backspace"
  (lambda ()
    (if (= (str-length project-search-query) 0)
      (project-search-do-cancel)
      (begin
        (set project-search-query
          (str-substring project-search-query 0
            (- (str-length project-search-query) 1)))
        (if (= (str-length project-search-query) 0)
          (project-search-do-cancel)
          (project-search-apply-filter))))))

;; Enter: open the selected result file
(define-command "project-search-enter"
  (lambda ()
    (if (= (length project-search-results) 0)
      nil
      (begin
        ;; Get cursor position in results (cursor line minus header offset)
        (open-file (path-join browser-root-dir
          (first (nth (- (panel-cursor-line "filetree") project-search-header-offset)
                      project-search-results))))
        (project-search-reset)
        (set sidebar-visible nil)
        (set-panel-size "filetree" 0)
        (unfocus-panel)
        (set-mode "normal")
        (set-active-keymap "normal-mode")))))

;; ---------------------------------------------------------------------------
;; Project search keymap — per-character commands
;; ---------------------------------------------------------------------------

(make-keymap "project-search-mode")

;; Navigation and control keys
(define-key "project-search-mode" "Escape" "project-search-cancel")
(define-key "project-search-mode" "Enter" "project-search-enter")
(define-key "project-search-mode" "Backspace" "project-search-backspace")

;; Per-character commands for search input: lowercase a-z
(define-command "project-search-char-a" (lambda () (project-search-append-char "a")))
(define-command "project-search-char-b" (lambda () (project-search-append-char "b")))
(define-command "project-search-char-c" (lambda () (project-search-append-char "c")))
(define-command "project-search-char-d" (lambda () (project-search-append-char "d")))
(define-command "project-search-char-e" (lambda () (project-search-append-char "e")))
(define-command "project-search-char-f" (lambda () (project-search-append-char "f")))
(define-command "project-search-char-g" (lambda () (project-search-append-char "g")))
(define-command "project-search-char-h" (lambda () (project-search-append-char "h")))
(define-command "project-search-char-i" (lambda () (project-search-append-char "i")))
(define-command "project-search-char-j" (lambda () (project-search-append-char "j")))
(define-command "project-search-char-k" (lambda () (project-search-append-char "k")))
(define-command "project-search-char-l" (lambda () (project-search-append-char "l")))
(define-command "project-search-char-m" (lambda () (project-search-append-char "m")))
(define-command "project-search-char-n" (lambda () (project-search-append-char "n")))
(define-command "project-search-char-o" (lambda () (project-search-append-char "o")))
(define-command "project-search-char-p" (lambda () (project-search-append-char "p")))
(define-command "project-search-char-q" (lambda () (project-search-append-char "q")))
(define-command "project-search-char-r" (lambda () (project-search-append-char "r")))
(define-command "project-search-char-s" (lambda () (project-search-append-char "s")))
(define-command "project-search-char-t" (lambda () (project-search-append-char "t")))
(define-command "project-search-char-u" (lambda () (project-search-append-char "u")))
(define-command "project-search-char-v" (lambda () (project-search-append-char "v")))
(define-command "project-search-char-w" (lambda () (project-search-append-char "w")))
(define-command "project-search-char-x" (lambda () (project-search-append-char "x")))
(define-command "project-search-char-y" (lambda () (project-search-append-char "y")))
(define-command "project-search-char-z" (lambda () (project-search-append-char "z")))

;; Keybindings for lowercase a-z
(define-key "project-search-mode" "Char:a" "project-search-char-a")
(define-key "project-search-mode" "Char:b" "project-search-char-b")
(define-key "project-search-mode" "Char:c" "project-search-char-c")
(define-key "project-search-mode" "Char:d" "project-search-char-d")
(define-key "project-search-mode" "Char:e" "project-search-char-e")
(define-key "project-search-mode" "Char:f" "project-search-char-f")
(define-key "project-search-mode" "Char:g" "project-search-char-g")
(define-key "project-search-mode" "Char:h" "project-search-char-h")
(define-key "project-search-mode" "Char:i" "project-search-char-i")
(define-key "project-search-mode" "Char:j" "project-search-char-j")
(define-key "project-search-mode" "Char:k" "project-search-char-k")
(define-key "project-search-mode" "Char:l" "project-search-char-l")
(define-key "project-search-mode" "Char:m" "project-search-char-m")
(define-key "project-search-mode" "Char:n" "project-search-char-n")
(define-key "project-search-mode" "Char:o" "project-search-char-o")
(define-key "project-search-mode" "Char:p" "project-search-char-p")
(define-key "project-search-mode" "Char:q" "project-search-char-q")
(define-key "project-search-mode" "Char:r" "project-search-char-r")
(define-key "project-search-mode" "Char:s" "project-search-char-s")
(define-key "project-search-mode" "Char:t" "project-search-char-t")
(define-key "project-search-mode" "Char:u" "project-search-char-u")
(define-key "project-search-mode" "Char:v" "project-search-char-v")
(define-key "project-search-mode" "Char:w" "project-search-char-w")
(define-key "project-search-mode" "Char:x" "project-search-char-x")
(define-key "project-search-mode" "Char:y" "project-search-char-y")
(define-key "project-search-mode" "Char:z" "project-search-char-z")

;; Per-character commands for uppercase A-Z
(define-command "project-search-char-A" (lambda () (project-search-append-char "A")))
(define-command "project-search-char-B" (lambda () (project-search-append-char "B")))
(define-command "project-search-char-C" (lambda () (project-search-append-char "C")))
(define-command "project-search-char-D" (lambda () (project-search-append-char "D")))
(define-command "project-search-char-E" (lambda () (project-search-append-char "E")))
(define-command "project-search-char-F" (lambda () (project-search-append-char "F")))
(define-command "project-search-char-G" (lambda () (project-search-append-char "G")))
(define-command "project-search-char-H" (lambda () (project-search-append-char "H")))
(define-command "project-search-char-I" (lambda () (project-search-append-char "I")))
(define-command "project-search-char-J" (lambda () (project-search-append-char "J")))
(define-command "project-search-char-K" (lambda () (project-search-append-char "K")))
(define-command "project-search-char-L" (lambda () (project-search-append-char "L")))
(define-command "project-search-char-M" (lambda () (project-search-append-char "M")))
(define-command "project-search-char-N" (lambda () (project-search-append-char "N")))
(define-command "project-search-char-O" (lambda () (project-search-append-char "O")))
(define-command "project-search-char-P" (lambda () (project-search-append-char "P")))
(define-command "project-search-char-Q" (lambda () (project-search-append-char "Q")))
(define-command "project-search-char-R" (lambda () (project-search-append-char "R")))
(define-command "project-search-char-S" (lambda () (project-search-append-char "S")))
(define-command "project-search-char-T" (lambda () (project-search-append-char "T")))
(define-command "project-search-char-U" (lambda () (project-search-append-char "U")))
(define-command "project-search-char-V" (lambda () (project-search-append-char "V")))
(define-command "project-search-char-W" (lambda () (project-search-append-char "W")))
(define-command "project-search-char-X" (lambda () (project-search-append-char "X")))
(define-command "project-search-char-Y" (lambda () (project-search-append-char "Y")))
(define-command "project-search-char-Z" (lambda () (project-search-append-char "Z")))

;; Keybindings for uppercase A-Z
(define-key "project-search-mode" "Char:A" "project-search-char-A")
(define-key "project-search-mode" "Char:B" "project-search-char-B")
(define-key "project-search-mode" "Char:C" "project-search-char-C")
(define-key "project-search-mode" "Char:D" "project-search-char-D")
(define-key "project-search-mode" "Char:E" "project-search-char-E")
(define-key "project-search-mode" "Char:F" "project-search-char-F")
(define-key "project-search-mode" "Char:G" "project-search-char-G")
(define-key "project-search-mode" "Char:H" "project-search-char-H")
(define-key "project-search-mode" "Char:I" "project-search-char-I")
(define-key "project-search-mode" "Char:J" "project-search-char-J")
(define-key "project-search-mode" "Char:K" "project-search-char-K")
(define-key "project-search-mode" "Char:L" "project-search-char-L")
(define-key "project-search-mode" "Char:M" "project-search-char-M")
(define-key "project-search-mode" "Char:N" "project-search-char-N")
(define-key "project-search-mode" "Char:O" "project-search-char-O")
(define-key "project-search-mode" "Char:P" "project-search-char-P")
(define-key "project-search-mode" "Char:Q" "project-search-char-Q")
(define-key "project-search-mode" "Char:R" "project-search-char-R")
(define-key "project-search-mode" "Char:S" "project-search-char-S")
(define-key "project-search-mode" "Char:T" "project-search-char-T")
(define-key "project-search-mode" "Char:U" "project-search-char-U")
(define-key "project-search-mode" "Char:V" "project-search-char-V")
(define-key "project-search-mode" "Char:W" "project-search-char-W")
(define-key "project-search-mode" "Char:X" "project-search-char-X")
(define-key "project-search-mode" "Char:Y" "project-search-char-Y")
(define-key "project-search-mode" "Char:Z" "project-search-char-Z")

;; Per-character commands for digits 0-9
(define-command "project-search-char-0" (lambda () (project-search-append-char "0")))
(define-command "project-search-char-1" (lambda () (project-search-append-char "1")))
(define-command "project-search-char-2" (lambda () (project-search-append-char "2")))
(define-command "project-search-char-3" (lambda () (project-search-append-char "3")))
(define-command "project-search-char-4" (lambda () (project-search-append-char "4")))
(define-command "project-search-char-5" (lambda () (project-search-append-char "5")))
(define-command "project-search-char-6" (lambda () (project-search-append-char "6")))
(define-command "project-search-char-7" (lambda () (project-search-append-char "7")))
(define-command "project-search-char-8" (lambda () (project-search-append-char "8")))
(define-command "project-search-char-9" (lambda () (project-search-append-char "9")))

;; Keybindings for digits 0-9
(define-key "project-search-mode" "Char:0" "project-search-char-0")
(define-key "project-search-mode" "Char:1" "project-search-char-1")
(define-key "project-search-mode" "Char:2" "project-search-char-2")
(define-key "project-search-mode" "Char:3" "project-search-char-3")
(define-key "project-search-mode" "Char:4" "project-search-char-4")
(define-key "project-search-mode" "Char:5" "project-search-char-5")
(define-key "project-search-mode" "Char:6" "project-search-char-6")
(define-key "project-search-mode" "Char:7" "project-search-char-7")
(define-key "project-search-mode" "Char:8" "project-search-char-8")
(define-key "project-search-mode" "Char:9" "project-search-char-9")

;; Per-character commands for common symbols in filenames
(define-command "project-search-char-dash" (lambda () (project-search-append-char "-")))
(define-command "project-search-char-underscore" (lambda () (project-search-append-char "_")))
(define-command "project-search-char-dot" (lambda () (project-search-append-char ".")))
(define-command "project-search-char-slash" (lambda () (project-search-append-char "/")))
(define-command "project-search-char-space" (lambda () (project-search-append-char " ")))

;; Keybindings for common symbols
(define-key "project-search-mode" "Char:-" "project-search-char-dash")
(define-key "project-search-mode" "Char:_" "project-search-char-underscore")
(define-key "project-search-mode" "Char:." "project-search-char-dot")
(define-key "project-search-mode" "Char:/" "project-search-char-slash")
(define-key "project-search-mode" "Char: " "project-search-char-space")

;; Ctrl-p in normal mode activates project search
(define-key "normal-mode" "Ctrl:p" "project-search-start")

;; ---------------------------------------------------------------------------

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
