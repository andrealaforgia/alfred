;;; name: vim-keybindings
;;; version: 0.1.0
;;; description: Vim-style modal keybindings

;; Create normal-mode keymap with hjkl navigation and editing commands
(make-keymap "normal-mode")
(define-key "normal-mode" "Char:h" "cursor-left")
(define-key "normal-mode" "Char:j" "cursor-down")
(define-key "normal-mode" "Char:k" "cursor-up")
(define-key "normal-mode" "Char:l" "cursor-right")
(define-key "normal-mode" "Char:i" "enter-insert-mode")
(define-key "normal-mode" "Char:x" "delete-char-at-cursor")
(define-key "normal-mode" "Char:d" "delete-line")
(define-key "normal-mode" "Char::" "enter-command-mode")

;; Create insert-mode keymap with Escape to return to normal mode
(make-keymap "insert-mode")
(define-key "insert-mode" "Escape" "enter-normal-mode")

;; Define mode-switching commands
(define-command "enter-insert-mode" (lambda () (set-mode "insert")))
(define-command "enter-normal-mode" (lambda () (set-mode "normal")))

;; Start in normal mode
(set-active-keymap "normal-mode")
(set-mode "normal")
