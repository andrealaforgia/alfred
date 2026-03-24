;;; name: browse-mode
;;; version: 0.1.0
;;; description: Folder browser keymap and cursor configuration

;; Create the browse-mode keymap
(make-keymap "browse-mode")

;; Navigation
(define-key "browse-mode" "j" "browser-cursor-down")
(define-key "browse-mode" "k" "browser-cursor-up")
(define-key "browse-mode" "Down" "browser-cursor-down")
(define-key "browse-mode" "Up" "browser-cursor-up")
(define-key "browse-mode" "g" "browser-jump-first")
(define-key "browse-mode" "G" "browser-jump-last")

;; Actions
(define-key "browse-mode" "Enter" "browser-enter")
(define-key "browse-mode" "l" "browser-enter")
(define-key "browse-mode" "h" "browser-parent")
(define-key "browse-mode" "Backspace" "browser-parent")

;; Quit
(define-key "browse-mode" "q" "browser-quit")

;; Cursor shape for browse mode
(set-cursor-shape "browse" "block")
