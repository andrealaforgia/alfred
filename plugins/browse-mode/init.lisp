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

;; ---------------------------------------------------------------------------
;; Helpers — no local (define) inside lambdas, use args or inline
;; ---------------------------------------------------------------------------

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
;; Render: rebuild buffer text from current state
;; ---------------------------------------------------------------------------

(define browser-render
  (lambda ()
    (buffer-set-content
      (str-concat
        (list
          " " browser-current-dir newline
          newline
          (if (= (length browser-entries) 0)
            "   (empty directory)"
            (browser-build-lines browser-entries 0)))))))

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
    (if (< browser-cursor (- (length browser-entries) 1))
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
    (set browser-cursor (- (length browser-entries) 1))
    (browser-render)))

(define-command "browser-enter"
  (lambda ()
    (if (= (nth 1 (nth browser-cursor browser-entries)) "dir")
      (if (= (first (nth browser-cursor browser-entries)) "..")
        (browser-do-parent)
        (browser-do-enter-dir (first (nth browser-cursor browser-entries))))
      (open-file
        (path-join browser-current-dir
          (first (nth browser-cursor browser-entries)))))))

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
;; File tree sidebar (read-only left panel)
;; ---------------------------------------------------------------------------

(define browser-sidebar-visible nil)
(define browser-sidebar-width 30)

(define browser-build-sidebar
  (lambda (entries idx)
    (if (= (length entries) 0)
      nil
      (begin
        (set-panel-line "filetree" idx
          (str-concat (list
            " "
            (first (nth idx entries))
            (if (= (nth 1 (nth idx entries)) "dir") "/" browser-empty-str))))
        (if (< (+ idx 1) (length entries))
          (browser-build-sidebar entries (+ idx 1))
          nil)))))

(define-command "toggle-sidebar"
  (lambda ()
    (if (= browser-root-dir browser-empty-str)
      (message "No browse directory set")
      (if browser-sidebar-visible
        (begin
          (set browser-sidebar-visible nil)
          (set-panel-size "filetree" 0))
        (begin
          (set browser-sidebar-visible 1)
          (define-panel "filetree" "left" browser-sidebar-width)
          (set-panel-style "filetree" "#6c7086" "default")
          (set-panel-size "filetree" browser-sidebar-width)
          (define sidebar-entries (list-dir browser-root-dir))
          (browser-build-sidebar sidebar-entries 0))))))

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
      (browser-load-dir browser-cli-arg)
      (set-mode "browse")
      (set-active-keymap "browse-mode"))
    nil))
