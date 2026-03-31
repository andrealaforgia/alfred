;;; name: browse-mode
;;; version: 3.0.0
;;; description: File tree sidebar panel with browse and recursive search states
;;; depends: vim-keybindings

;; ---------------------------------------------------------------------------
;; Constants
;; ---------------------------------------------------------------------------

(define browser-empty-str (str-concat (list)))
(define browser-panel-name "filetree")
(define browser-panel-width 30)
(define browser-header-offset 2)

;; Colors (Catppuccin Mocha)
(define browser-color-blue "#89b4fa")
(define browser-color-pink "#f5c2e7")
(define browser-color-gray "#cdd6f4")
(define browser-color-cursor-fg "#1e1e2e")
(define browser-color-cursor-bg "#cdd6f4")

;; ---------------------------------------------------------------------------
;; State variables
;; ---------------------------------------------------------------------------

(define browser-root-dir browser-empty-str)
(define browser-current-dir browser-empty-str)
(define browser-entries (list))
(define browser-visible nil)
(define browser-created nil)
(define browser-search-active nil)
(define browser-search-query (str-concat (list)))
(define browser-search-results (list))

;; ---------------------------------------------------------------------------
;; Helpers — pure utility functions
;; ---------------------------------------------------------------------------

;; Add ".." parent entry unless at root
(define browser-add-parent-entry
  (lambda (dir entries)
    (if (= dir browser-root-dir)
      entries
      (if (= (path-parent dir) dir)
        entries
        (cons (list ".." "dir") entries)))))

;; Format a browse entry with cursor indicator and type suffix
(define browser-format-entry
  (lambda (entry idx cursor)
    (str-concat (list
      (if (= idx cursor) " > " "   ")
      (first entry)
      (if (= (nth 1 entry) "dir") "/" browser-empty-str)))))

;; Format a search result with cursor indicator
(define browser-format-search-result
  (lambda (entry idx cursor)
    (str-concat (list
      (if (= idx cursor) " > " "   ")
      (first entry)))))

;; Check if entry name contains query (case-insensitive)
(define browser-entry-matches
  (lambda (entry query)
    (str-contains (str-lower (first entry)) (str-lower query))))

;; Filter search results by query
(define browser-filter-results
  (lambda (entries query)
    (if (= (str-length query) 0)
      entries
      (filter (lambda (e) (browser-entry-matches e query)) entries))))

;; ---------------------------------------------------------------------------
;; Panel rendering — populate lines and apply styles
;; ---------------------------------------------------------------------------

;; Populate browse-mode entries into panel lines starting at header-offset
(define browser-populate-browse-entries
  (lambda (entries idx cursor)
    (if (= idx (length entries))
      nil
      (begin
        (set-panel-line browser-panel-name (+ idx browser-header-offset)
          (browser-format-entry (nth idx entries) idx cursor))
        (browser-populate-browse-entries entries (+ idx 1) cursor)))))

;; Style a single browse entry line
(define browser-style-browse-entry
  (lambda (entry idx cursor)
    (if (= idx cursor)
      (set-panel-line-style browser-panel-name (+ idx browser-header-offset) 0
        (str-length (browser-format-entry entry idx cursor))
        browser-color-pink)
      (if (= (nth 1 entry) "dir")
        (set-panel-line-style browser-panel-name (+ idx browser-header-offset) 0
          (str-length (browser-format-entry entry idx cursor))
          browser-color-blue)
        (set-panel-line-style browser-panel-name (+ idx browser-header-offset) 0
          (str-length (browser-format-entry entry idx cursor))
          browser-color-gray)))))

;; Recursively style all browse entries
(define browser-style-browse-entries
  (lambda (entries idx cursor)
    (if (= idx (length entries))
      nil
      (begin
        (browser-style-browse-entry (nth idx entries) idx cursor)
        (browser-style-browse-entries entries (+ idx 1) cursor)))))

;; Populate search results into panel lines
(define browser-populate-search-results
  (lambda (results idx cursor)
    (if (= idx (length results))
      nil
      (begin
        (set-panel-line browser-panel-name (+ idx browser-header-offset)
          (browser-format-search-result (nth idx results) idx cursor))
        (browser-populate-search-results results (+ idx 1) cursor)))))

;; Style a single search result line
(define browser-style-search-result
  (lambda (result idx cursor)
    (if (= idx cursor)
      (set-panel-line-style browser-panel-name (+ idx browser-header-offset) 0
        (str-length (browser-format-search-result result idx cursor))
        browser-color-pink)
      (set-panel-line-style browser-panel-name (+ idx browser-header-offset) 0
        (str-length (browser-format-search-result result idx cursor))
        browser-color-gray))))

;; Recursively style all search results
(define browser-style-search-results
  (lambda (results idx cursor)
    (if (= idx (length results))
      nil
      (begin
        (browser-style-search-result (nth idx results) idx cursor)
        (browser-style-search-results results (+ idx 1) cursor)))))

;; ---------------------------------------------------------------------------
;; Render — rebuild the panel from current state
;; ---------------------------------------------------------------------------

(define browser-render-browse
  (lambda ()
    (clear-panel-lines browser-panel-name)
    (clear-panel-line-styles browser-panel-name)
    ;; Header: directory path
    (set-panel-line browser-panel-name 0
      (str-concat (list " " browser-current-dir)))
    (set-panel-line-style browser-panel-name 0 0
      (+ (str-length browser-current-dir) 1) browser-color-blue)
    ;; Separator
    (set-panel-line browser-panel-name 1 (str-concat (list)))
    ;; Entries
    (if (> (length browser-entries) 0)
      (begin
        (panel-set-cursor browser-panel-name browser-header-offset)
        (browser-populate-browse-entries browser-entries 0
          (- (panel-cursor-line browser-panel-name) browser-header-offset))
        (browser-style-browse-entries browser-entries 0
          (- (panel-cursor-line browser-panel-name) browser-header-offset)))
      nil)))

(define browser-render-search
  (lambda ()
    (clear-panel-lines browser-panel-name)
    (clear-panel-line-styles browser-panel-name)
    ;; Header: search prompt
    (set-panel-line browser-panel-name 0
      (str-concat (list " / " browser-search-query)))
    (set-panel-line-style browser-panel-name 0 0
      (+ (str-length browser-search-query) 3) browser-color-pink)
    ;; Separator
    (set-panel-line browser-panel-name 1 (str-concat (list)))
    ;; Results
    (if (> (length browser-search-results) 0)
      (begin
        (panel-set-cursor browser-panel-name browser-header-offset)
        (browser-populate-search-results browser-search-results 0
          (- (panel-cursor-line browser-panel-name) browser-header-offset))
        (browser-style-search-results browser-search-results 0
          (- (panel-cursor-line browser-panel-name) browser-header-offset)))
      (set-panel-line browser-panel-name browser-header-offset
        "   (no matches)"))))

(define browser-render
  (lambda ()
    (if browser-search-active
      (browser-render-search)
      (browser-render-browse))))

;; ---------------------------------------------------------------------------
;; Refresh — re-render entries and styles after cursor movement
;; ---------------------------------------------------------------------------

(define browser-refresh
  (lambda ()
    (if browser-search-active
      (begin
        (browser-populate-search-results browser-search-results 0
          (- (panel-cursor-line browser-panel-name) browser-header-offset))
        (clear-panel-line-styles browser-panel-name)
        (set-panel-line-style browser-panel-name 0 0
          (+ (str-length browser-search-query) 3) browser-color-pink)
        (browser-style-search-results browser-search-results 0
          (- (panel-cursor-line browser-panel-name) browser-header-offset)))
      (begin
        (browser-populate-browse-entries browser-entries 0
          (- (panel-cursor-line browser-panel-name) browser-header-offset))
        (clear-panel-line-styles browser-panel-name)
        (set-panel-line-style browser-panel-name 0 0
          (+ (str-length browser-current-dir) 1) browser-color-blue)
        (browser-style-browse-entries browser-entries 0
          (- (panel-cursor-line browser-panel-name) browser-header-offset))))))

;; ---------------------------------------------------------------------------
;; Directory loading
;; ---------------------------------------------------------------------------

(define browser-load-dir
  (lambda (dir)
    (set browser-current-dir dir)
    (set browser-entries (browser-add-parent-entry dir (list-dir dir)))
    (browser-render-browse)))

;; ---------------------------------------------------------------------------
;; Panel lifecycle — create, show, hide
;; ---------------------------------------------------------------------------

(define browser-ensure-panel
  (lambda ()
    (if browser-created
      nil
      (begin
        (define-panel browser-panel-name "left" browser-panel-width)
        (set-panel-priority browser-panel-name 10)
        (set browser-created 1)))
    (set-panel-style browser-panel-name "#6c7086" "#1e1e2e")))

(define browser-show-panel
  (lambda ()
    (browser-ensure-panel)
    (set-panel-size browser-panel-name browser-panel-width)
    (set browser-visible 1)))

(define browser-hide-panel
  (lambda ()
    (set-panel-size browser-panel-name 0)
    (set browser-visible nil)))

(define browser-focus-panel
  (lambda ()
    (focus-panel browser-panel-name)
    (set-mode "panel-browse")
    (set-active-keymap "browser-panel-mode")))

(define browser-unfocus-panel
  (lambda ()
    (unfocus-panel)
    (set-mode "normal")
    (set-active-keymap "normal-mode")))

;; ---------------------------------------------------------------------------
;; Browse-mode commands
;; ---------------------------------------------------------------------------

;; Cursor helpers
(define browser-current-entries
  (lambda ()
    (if browser-search-active
      browser-search-results
      browser-entries)))

(define browser-entry-index
  (lambda ()
    (- (panel-cursor-line browser-panel-name) browser-header-offset)))

(define browser-current-entry-name
  (lambda ()
    (first (nth (browser-entry-index) (browser-current-entries)))))

(define browser-current-entry-type
  (lambda ()
    (nth 1 (nth (browser-entry-index) (browser-current-entries)))))

;; Navigation
(define-command "browser-cursor-down"
  (lambda ()
    (panel-cursor-down browser-panel-name)
    (browser-refresh)))

(define-command "browser-cursor-up"
  (lambda ()
    (if (> (panel-cursor-line browser-panel-name) browser-header-offset)
      (panel-cursor-up browser-panel-name)
      nil)
    (browser-refresh)))

;; Enter: open file or descend into directory
(define-command "browser-enter"
  (lambda ()
    (if (= (length (browser-current-entries)) 0)
      nil
      (if browser-search-active
        ;; Search mode: open file, cancel search, unfocus
        (if (= (browser-current-entry-type) "file")
          (begin
            (open-file (path-join browser-root-dir (browser-current-entry-name)))
            (browser-cancel-search)
            (browser-unfocus-panel))
          nil)
        ;; Browse mode: open file or enter directory
        (if (= (browser-current-entry-type) "dir")
          (if (= (browser-current-entry-name) "..")
            (browser-load-dir (path-parent browser-current-dir))
            (browser-load-dir (path-join browser-current-dir (browser-current-entry-name))))
          (begin
            (open-file (path-join browser-current-dir (browser-current-entry-name)))
            (browser-unfocus-panel)))))))

;; Parent directory
(define-command "browser-parent"
  (lambda ()
    (if browser-search-active
      nil
      (if (= (path-parent browser-current-dir) browser-current-dir)
        nil
        (browser-load-dir (path-parent browser-current-dir))))))

;; Quit/Escape: unfocus (browse) or cancel search (search)
(define-command "browser-quit"
  (lambda ()
    (if browser-search-active
      (begin
        (browser-cancel-search)
        (browser-render-browse))
      (browser-unfocus-panel))))

;; ---------------------------------------------------------------------------
;; Search mode
;; ---------------------------------------------------------------------------

;; Cancel search and return to browse state
(define browser-cancel-search
  (lambda ()
    (set browser-search-active nil)
    (set browser-search-query (str-concat (list)))
    (set browser-search-results (list))
    (set-active-keymap "browser-panel-mode")
    (browser-render-browse)))

;; Start search: load recursive file list, switch keymap
(define-command "browser-start-search"
  (lambda ()
    (set browser-search-active 1)
    (set browser-search-query (str-concat (list)))
    (set browser-search-results (list-dir-recursive browser-root-dir))
    (set-active-keymap "browser-search-input")
    (browser-render-search)))

;; Append character to search query and re-filter
(define browser-search-append
  (lambda (ch)
    (set browser-search-query (str-concat (list browser-search-query ch)))
    (set browser-search-results
      (browser-filter-results (list-dir-recursive browser-root-dir) browser-search-query))
    (browser-render-search)))

;; Backspace: remove last character or cancel if empty
(define-command "browser-search-backspace"
  (lambda ()
    (if (= (str-length browser-search-query) 0)
      (browser-cancel-search)
      (begin
        (set browser-search-query
          (str-substring browser-search-query 0
            (- (str-length browser-search-query) 1)))
        (set browser-search-results
          (browser-filter-results (list-dir-recursive browser-root-dir) browser-search-query))
        (browser-render-search)))))

;; Cancel search via Escape
(define-command "browser-search-cancel"
  (lambda () (browser-cancel-search)))

;; Enter from search: open selected result
(define-command "browser-search-enter"
  (lambda ()
    (if (= (length browser-search-results) 0)
      nil
      (begin
        (open-file (path-join browser-root-dir
          (first (nth (- (panel-cursor-line browser-panel-name) browser-header-offset)
                      browser-search-results))))
        (browser-cancel-search)
        (browser-unfocus-panel)))))

;; Search cursor navigation
(define-command "browser-search-cursor-down"
  (lambda ()
    (panel-cursor-down browser-panel-name)
    (browser-refresh)))

(define-command "browser-search-cursor-up"
  (lambda ()
    (if (> (panel-cursor-line browser-panel-name) browser-header-offset)
      (panel-cursor-up browser-panel-name)
      nil)
    (browser-refresh)))

;; ---------------------------------------------------------------------------
;; Toggle sidebar (Ctrl-e from normal mode or panel mode)
;; ---------------------------------------------------------------------------

(define-command "toggle-sidebar"
  (lambda ()
    (if (= browser-root-dir browser-empty-str)
      (message "No browse directory set")
      (if browser-visible
        ;; Visible: hide panel, unfocus
        (begin
          (browser-hide-panel)
          (unfocus-panel)
          (set-mode "normal")
          (set-active-keymap "normal-mode"))
        ;; Hidden: show panel, reload entries, focus
        (begin
          (browser-show-panel)
          (browser-load-dir
            (if (= browser-current-dir browser-empty-str)
              browser-root-dir
              browser-current-dir))
          (browser-focus-panel))))))

;; ---------------------------------------------------------------------------
;; Keymaps
;; ---------------------------------------------------------------------------

;; Browser panel mode — active when panel is focused in browse state
(make-keymap "browser-panel-mode")
(define-key "browser-panel-mode" "Char:j" "browser-cursor-down")
(define-key "browser-panel-mode" "Char:k" "browser-cursor-up")
(define-key "browser-panel-mode" "Down" "browser-cursor-down")
(define-key "browser-panel-mode" "Up" "browser-cursor-up")
(define-key "browser-panel-mode" "Enter" "browser-enter")
(define-key "browser-panel-mode" "Char:l" "browser-enter")
(define-key "browser-panel-mode" "Char:h" "browser-parent")
(define-key "browser-panel-mode" "Backspace" "browser-parent")
(define-key "browser-panel-mode" "Char:/" "browser-start-search")
(define-key "browser-panel-mode" "Char:q" "browser-quit")
(define-key "browser-panel-mode" "Escape" "browser-quit")
(define-key "browser-panel-mode" "Ctrl:e" "toggle-sidebar")
(define-key "browser-panel-mode" "Ctrl:p" "overlay-file-search")

;; Browser search input mode — captures typed characters for search
(make-keymap "browser-search-input")
(define-key "browser-search-input" "Escape" "browser-search-cancel")
(define-key "browser-search-input" "Enter" "browser-search-enter")
(define-key "browser-search-input" "Backspace" "browser-search-backspace")
(define-key "browser-search-input" "Down" "browser-search-cursor-down")
(define-key "browser-search-input" "Up" "browser-search-cursor-up")

;; Per-character commands for search input: lowercase a-z
(define-command "browser-search-char-a" (lambda () (browser-search-append "a")))
(define-command "browser-search-char-b" (lambda () (browser-search-append "b")))
(define-command "browser-search-char-c" (lambda () (browser-search-append "c")))
(define-command "browser-search-char-d" (lambda () (browser-search-append "d")))
(define-command "browser-search-char-e" (lambda () (browser-search-append "e")))
(define-command "browser-search-char-f" (lambda () (browser-search-append "f")))
(define-command "browser-search-char-g" (lambda () (browser-search-append "g")))
(define-command "browser-search-char-h" (lambda () (browser-search-append "h")))
(define-command "browser-search-char-i" (lambda () (browser-search-append "i")))
(define-command "browser-search-char-j" (lambda () (browser-search-append "j")))
(define-command "browser-search-char-k" (lambda () (browser-search-append "k")))
(define-command "browser-search-char-l" (lambda () (browser-search-append "l")))
(define-command "browser-search-char-m" (lambda () (browser-search-append "m")))
(define-command "browser-search-char-n" (lambda () (browser-search-append "n")))
(define-command "browser-search-char-o" (lambda () (browser-search-append "o")))
(define-command "browser-search-char-p" (lambda () (browser-search-append "p")))
(define-command "browser-search-char-q" (lambda () (browser-search-append "q")))
(define-command "browser-search-char-r" (lambda () (browser-search-append "r")))
(define-command "browser-search-char-s" (lambda () (browser-search-append "s")))
(define-command "browser-search-char-t" (lambda () (browser-search-append "t")))
(define-command "browser-search-char-u" (lambda () (browser-search-append "u")))
(define-command "browser-search-char-v" (lambda () (browser-search-append "v")))
(define-command "browser-search-char-w" (lambda () (browser-search-append "w")))
(define-command "browser-search-char-x" (lambda () (browser-search-append "x")))
(define-command "browser-search-char-y" (lambda () (browser-search-append "y")))
(define-command "browser-search-char-z" (lambda () (browser-search-append "z")))

(define-key "browser-search-input" "Char:a" "browser-search-char-a")
(define-key "browser-search-input" "Char:b" "browser-search-char-b")
(define-key "browser-search-input" "Char:c" "browser-search-char-c")
(define-key "browser-search-input" "Char:d" "browser-search-char-d")
(define-key "browser-search-input" "Char:e" "browser-search-char-e")
(define-key "browser-search-input" "Char:f" "browser-search-char-f")
(define-key "browser-search-input" "Char:g" "browser-search-char-g")
(define-key "browser-search-input" "Char:h" "browser-search-char-h")
(define-key "browser-search-input" "Char:i" "browser-search-char-i")
(define-key "browser-search-input" "Char:j" "browser-search-char-j")
(define-key "browser-search-input" "Char:k" "browser-search-char-k")
(define-key "browser-search-input" "Char:l" "browser-search-char-l")
(define-key "browser-search-input" "Char:m" "browser-search-char-m")
(define-key "browser-search-input" "Char:n" "browser-search-char-n")
(define-key "browser-search-input" "Char:o" "browser-search-char-o")
(define-key "browser-search-input" "Char:p" "browser-search-char-p")
(define-key "browser-search-input" "Char:q" "browser-search-char-q")
(define-key "browser-search-input" "Char:r" "browser-search-char-r")
(define-key "browser-search-input" "Char:s" "browser-search-char-s")
(define-key "browser-search-input" "Char:t" "browser-search-char-t")
(define-key "browser-search-input" "Char:u" "browser-search-char-u")
(define-key "browser-search-input" "Char:v" "browser-search-char-v")
(define-key "browser-search-input" "Char:w" "browser-search-char-w")
(define-key "browser-search-input" "Char:x" "browser-search-char-x")
(define-key "browser-search-input" "Char:y" "browser-search-char-y")
(define-key "browser-search-input" "Char:z" "browser-search-char-z")

;; Per-character commands for uppercase A-Z
(define-command "browser-search-char-A" (lambda () (browser-search-append "A")))
(define-command "browser-search-char-B" (lambda () (browser-search-append "B")))
(define-command "browser-search-char-C" (lambda () (browser-search-append "C")))
(define-command "browser-search-char-D" (lambda () (browser-search-append "D")))
(define-command "browser-search-char-E" (lambda () (browser-search-append "E")))
(define-command "browser-search-char-F" (lambda () (browser-search-append "F")))
(define-command "browser-search-char-G" (lambda () (browser-search-append "G")))
(define-command "browser-search-char-H" (lambda () (browser-search-append "H")))
(define-command "browser-search-char-I" (lambda () (browser-search-append "I")))
(define-command "browser-search-char-J" (lambda () (browser-search-append "J")))
(define-command "browser-search-char-K" (lambda () (browser-search-append "K")))
(define-command "browser-search-char-L" (lambda () (browser-search-append "L")))
(define-command "browser-search-char-M" (lambda () (browser-search-append "M")))
(define-command "browser-search-char-N" (lambda () (browser-search-append "N")))
(define-command "browser-search-char-O" (lambda () (browser-search-append "O")))
(define-command "browser-search-char-P" (lambda () (browser-search-append "P")))
(define-command "browser-search-char-Q" (lambda () (browser-search-append "Q")))
(define-command "browser-search-char-R" (lambda () (browser-search-append "R")))
(define-command "browser-search-char-S" (lambda () (browser-search-append "S")))
(define-command "browser-search-char-T" (lambda () (browser-search-append "T")))
(define-command "browser-search-char-U" (lambda () (browser-search-append "U")))
(define-command "browser-search-char-V" (lambda () (browser-search-append "V")))
(define-command "browser-search-char-W" (lambda () (browser-search-append "W")))
(define-command "browser-search-char-X" (lambda () (browser-search-append "X")))
(define-command "browser-search-char-Y" (lambda () (browser-search-append "Y")))
(define-command "browser-search-char-Z" (lambda () (browser-search-append "Z")))

(define-key "browser-search-input" "Char:A" "browser-search-char-A")
(define-key "browser-search-input" "Char:B" "browser-search-char-B")
(define-key "browser-search-input" "Char:C" "browser-search-char-C")
(define-key "browser-search-input" "Char:D" "browser-search-char-D")
(define-key "browser-search-input" "Char:E" "browser-search-char-E")
(define-key "browser-search-input" "Char:F" "browser-search-char-F")
(define-key "browser-search-input" "Char:G" "browser-search-char-G")
(define-key "browser-search-input" "Char:H" "browser-search-char-H")
(define-key "browser-search-input" "Char:I" "browser-search-char-I")
(define-key "browser-search-input" "Char:J" "browser-search-char-J")
(define-key "browser-search-input" "Char:K" "browser-search-char-K")
(define-key "browser-search-input" "Char:L" "browser-search-char-L")
(define-key "browser-search-input" "Char:M" "browser-search-char-M")
(define-key "browser-search-input" "Char:N" "browser-search-char-N")
(define-key "browser-search-input" "Char:O" "browser-search-char-O")
(define-key "browser-search-input" "Char:P" "browser-search-char-P")
(define-key "browser-search-input" "Char:Q" "browser-search-char-Q")
(define-key "browser-search-input" "Char:R" "browser-search-char-R")
(define-key "browser-search-input" "Char:S" "browser-search-char-S")
(define-key "browser-search-input" "Char:T" "browser-search-char-T")
(define-key "browser-search-input" "Char:U" "browser-search-char-U")
(define-key "browser-search-input" "Char:V" "browser-search-char-V")
(define-key "browser-search-input" "Char:W" "browser-search-char-W")
(define-key "browser-search-input" "Char:X" "browser-search-char-X")
(define-key "browser-search-input" "Char:Y" "browser-search-char-Y")
(define-key "browser-search-input" "Char:Z" "browser-search-char-Z")

;; Per-character commands for digits 0-9
(define-command "browser-search-char-0" (lambda () (browser-search-append "0")))
(define-command "browser-search-char-1" (lambda () (browser-search-append "1")))
(define-command "browser-search-char-2" (lambda () (browser-search-append "2")))
(define-command "browser-search-char-3" (lambda () (browser-search-append "3")))
(define-command "browser-search-char-4" (lambda () (browser-search-append "4")))
(define-command "browser-search-char-5" (lambda () (browser-search-append "5")))
(define-command "browser-search-char-6" (lambda () (browser-search-append "6")))
(define-command "browser-search-char-7" (lambda () (browser-search-append "7")))
(define-command "browser-search-char-8" (lambda () (browser-search-append "8")))
(define-command "browser-search-char-9" (lambda () (browser-search-append "9")))

(define-key "browser-search-input" "Char:0" "browser-search-char-0")
(define-key "browser-search-input" "Char:1" "browser-search-char-1")
(define-key "browser-search-input" "Char:2" "browser-search-char-2")
(define-key "browser-search-input" "Char:3" "browser-search-char-3")
(define-key "browser-search-input" "Char:4" "browser-search-char-4")
(define-key "browser-search-input" "Char:5" "browser-search-char-5")
(define-key "browser-search-input" "Char:6" "browser-search-char-6")
(define-key "browser-search-input" "Char:7" "browser-search-char-7")
(define-key "browser-search-input" "Char:8" "browser-search-char-8")
(define-key "browser-search-input" "Char:9" "browser-search-char-9")

;; Per-character commands for common filename symbols
(define-command "browser-search-char-dash" (lambda () (browser-search-append "-")))
(define-command "browser-search-char-underscore" (lambda () (browser-search-append "_")))
(define-command "browser-search-char-dot" (lambda () (browser-search-append ".")))
(define-command "browser-search-char-slash" (lambda () (browser-search-append "/")))
(define-command "browser-search-char-space" (lambda () (browser-search-append " ")))

(define-key "browser-search-input" "Char:-" "browser-search-char-dash")
(define-key "browser-search-input" "Char:_" "browser-search-char-underscore")
(define-key "browser-search-input" "Char:." "browser-search-char-dot")
(define-key "browser-search-input" "Char:/" "browser-search-char-slash")
(define-key "browser-search-input" "Char: " "browser-search-char-space")

;; ---------------------------------------------------------------------------
;; Normal-mode binding for Ctrl-e toggle
;; ---------------------------------------------------------------------------

(define-key "normal-mode" "Ctrl:e" "toggle-sidebar")

;; ---------------------------------------------------------------------------
;; Activation: check CLI argument on load
;; ---------------------------------------------------------------------------

(define browser-cli-arg (cli-argument))

(if (= browser-cli-arg browser-empty-str)
  nil
  (if (is-dir? browser-cli-arg)
    ;; Directory argument: create panel, load entries, focus
    (begin
      (set browser-root-dir browser-cli-arg)
      (set browser-current-dir browser-cli-arg)
      (browser-show-panel)
      (browser-load-dir browser-cli-arg)
      (browser-focus-panel))
    ;; File argument: remember directory for later toggle
    (begin
      (set browser-root-dir (path-parent browser-cli-arg))
      (set browser-current-dir (path-parent browser-cli-arg))
      nil)))
