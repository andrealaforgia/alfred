;;; name: browse-mode
;;; version: 2.0.0
;;; description: Pure Lisp folder browser — renders directory listing as buffer text

;; ---------------------------------------------------------------------------
;; Keymap
;; ---------------------------------------------------------------------------

(make-keymap "browse-mode")

;; Navigation
(define-key "browse-mode" "Char:j" "browser-cursor-down")
(define-key "browse-mode" "Char:k" "browser-cursor-up")
(define-key "browse-mode" "Down" "browser-cursor-down")
(define-key "browse-mode" "Up" "browser-cursor-up")
(define-key "browse-mode" "Char:g" "browser-jump-first")
(define-key "browse-mode" "Char:G" "browser-jump-last")

;; Actions
(define-key "browse-mode" "Enter" "browser-enter")
(define-key "browse-mode" "Char:l" "browser-enter")
(define-key "browse-mode" "Char:h" "browser-parent")
(define-key "browse-mode" "Backspace" "browser-parent")

;; Quit
(define-key "browse-mode" "Char:q" "browser-quit")

;; Cursor shape
(set-cursor-shape "browse" "block")

;; ---------------------------------------------------------------------------
;; Browser state (module-level variables, mutated via `set`)
;; ---------------------------------------------------------------------------

(define browser-current-dir "")
(define browser-root-dir "")
(define browser-entries '())
(define browser-cursor 0)
(define browser-history '())

;; ---------------------------------------------------------------------------
;; Helpers
;; ---------------------------------------------------------------------------

;; Count elements in a list
(define browser-count
  (lambda (lst)
    (length lst)))

;; Return the entry at index n (0-based) from browser-entries
(define browser-entry-at
  (lambda (n)
    (nth n browser-entries)))

;; Format a single entry for display: "   name/" or "   name"
(define browser-format-entry
  (lambda (entry idx)
    (define name (first entry))
    (define type (nth 1 entry))
    (define prefix (if (= idx browser-cursor) " > " "   "))
    (define suffix (if (= type "dir") "/" ""))
    (str-concat (list prefix name suffix))))

;; Build a display line for each entry, joined with newlines
(define browser-build-lines
  (lambda (entries idx)
    (if (= (length entries) 0)
      ""
      (define entry (first entries))
      (define line (browser-format-entry entry idx))
      (define rest-lines (browser-build-lines (rest entries) (+ idx 1)))
      (if (= rest-lines "")
        line
        (str-concat (list line "\n" rest-lines))))))

;; ---------------------------------------------------------------------------
;; Render: rebuild buffer text from current state
;; ---------------------------------------------------------------------------

(define browser-render
  (lambda ()
    (define header (str-concat (list " " browser-current-dir)))
    (define separator "")
    (define body
      (if (= (browser-count browser-entries) 0)
        "   (empty directory)"
        (browser-build-lines browser-entries 0)))
    (define content (str-concat (list header "\n" separator "\n" body)))
    (buffer-set-content content)))

;; ---------------------------------------------------------------------------
;; Load entries for a directory
;; ---------------------------------------------------------------------------

(define browser-load-dir
  (lambda (dir)
    (set browser-current-dir dir)
    (define raw-entries (list-dir dir))
    ;; Prepend ../ entry if directory has a parent
    (define parent (path-parent dir))
    (define with-parent
      (if (= parent dir)
        raw-entries
        (cons (list ".." "dir") raw-entries)))
    (set browser-entries with-parent)
    (set browser-cursor 0)
    (browser-render)))

;; ---------------------------------------------------------------------------
;; Commands
;; ---------------------------------------------------------------------------

(define-command "browser-cursor-down"
  (lambda ()
    (define max-idx (- (browser-count browser-entries) 1))
    (if (< browser-cursor max-idx)
      (set browser-cursor (+ browser-cursor 1))
      #f)
    (browser-render)))

(define-command "browser-cursor-up"
  (lambda ()
    (if (> browser-cursor 0)
      (set browser-cursor (- browser-cursor 1))
      #f)
    (browser-render)))

(define-command "browser-jump-first"
  (lambda ()
    (set browser-cursor 0)
    (browser-render)))

(define-command "browser-jump-last"
  (lambda ()
    (set browser-cursor (- (browser-count browser-entries) 1))
    (browser-render)))

(define-command "browser-enter"
  (lambda ()
    (define entry (browser-entry-at browser-cursor))
    (define name (first entry))
    (define type (nth 1 entry))
    (if (= type "dir")
      ;; Enter directory
      (begin
        (define target
          (if (= name "..")
            (path-parent browser-current-dir)
            (path-join browser-current-dir name)))
        ;; Push current state onto history
        (set browser-history
          (cons (list browser-current-dir browser-cursor) browser-history))
        (browser-load-dir target))
      ;; Open file
      (begin
        (define file-path (path-join browser-current-dir name))
        (open-file file-path)))))

(define-command "browser-parent"
  (lambda ()
    (define parent (path-parent browser-current-dir))
    (if (= parent browser-current-dir)
      #f  ;; already at root
      (begin
        ;; Restore cursor from history if available
        (define saved-cursor 0)
        (if (> (length browser-history) 0)
          (begin
            (define top (first browser-history))
            (set saved-cursor (nth 1 top))
            (set browser-history (rest browser-history)))
          #f)
        (set browser-current-dir parent)
        (define raw-entries (list-dir parent))
        (define grand-parent (path-parent parent))
        (define with-parent
          (if (= grand-parent parent)
            raw-entries
            (cons (list ".." "dir") raw-entries)))
        (set browser-entries with-parent)
        (set browser-cursor saved-cursor)
        ;; Clamp cursor to valid range
        (define max-idx (- (browser-count browser-entries) 1))
        (if (> browser-cursor max-idx)
          (set browser-cursor max-idx)
          #f)
        (browser-render)))))

(define-command "browser-quit"
  (lambda ()
    (quit)))

;; ---------------------------------------------------------------------------
;; Activation: check CLI argument on load
;; ---------------------------------------------------------------------------

(define browser-cli-arg (cli-argument))

(if (= browser-cli-arg "")
  #f  ;; No argument, do nothing
  (if (is-dir? browser-cli-arg)
    ;; Directory argument: activate browse mode
    (begin
      (set browser-root-dir browser-cli-arg)
      (browser-load-dir browser-cli-arg)
      (set-mode "browse")
      (set-active-keymap "browse-mode"))
    #f))  ;; File argument, handled elsewhere
