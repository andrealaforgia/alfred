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

(define browser-current-dir "")
(define browser-root-dir "")
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
        (if (= (nth 1 entry) "dir") "/" "")))))

;; Recursive line builder
(define browser-build-lines
  (lambda (entries idx)
    (if (= (length entries) 0)
      ""
      (if (= (length entries) 1)
        (browser-format-entry (first entries) idx)
        (str-concat
          (list
            (browser-format-entry (first entries) idx)
            "\n"
            (browser-build-lines (rest entries) (+ idx 1))))))))

;; ---------------------------------------------------------------------------
;; Render: rebuild buffer text from current state
;; ---------------------------------------------------------------------------

(define browser-render
  (lambda ()
    (buffer-set-content
      (str-concat
        (list
          " " browser-current-dir "\n"
          "\n"
          (if (= (length browser-entries) 0)
            "   (empty directory)"
            (browser-build-lines browser-entries 0)))))))

;; ---------------------------------------------------------------------------
;; Load entries for a directory
;; ---------------------------------------------------------------------------

(define browser-add-parent-entry
  (lambda (dir entries)
    (if (= (path-parent dir) dir)
      entries
      (cons (list ".." "dir") entries))))

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
      #f
      (browser-do-go-parent))))

(define browser-do-go-parent
  (lambda ()
    (if (> (length browser-history) 0)
      (set browser-cursor (nth 1 (first browser-history)))
      #f)
    (if (> (length browser-history) 0)
      (set browser-history (rest browser-history))
      #f)
    (set browser-current-dir (path-parent browser-current-dir))
    (set browser-entries
      (browser-add-parent-entry browser-current-dir (list-dir browser-current-dir)))
    (if (> browser-cursor (- (length browser-entries) 1))
      (set browser-cursor (- (length browser-entries) 1))
      #f)
    (browser-render)))

(define-command "browser-quit"
  (lambda () (quit)))

;; ---------------------------------------------------------------------------
;; Activation: check CLI argument on load
;; ---------------------------------------------------------------------------

(define browser-cli-arg (cli-argument))

(if (= browser-cli-arg "")
  #f
  (if (is-dir? browser-cli-arg)
    (begin
      (set browser-root-dir browser-cli-arg)
      (browser-load-dir browser-cli-arg)
      (set-mode "browse")
      (set-active-keymap "browse-mode"))
    #f))
