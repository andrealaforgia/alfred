;;; name: basic-keybindings
;;; version: 0.1.0
;;; description: Default keybindings for arrow navigation, text insertion, backspace, and command mode

;; Create the global keymap
(make-keymap "global")

;; Arrow keys -> cursor movement commands
(define-key "global" "Up" "cursor-up")
(define-key "global" "Down" "cursor-down")
(define-key "global" "Left" "cursor-left")
(define-key "global" "Right" "cursor-right")

;; Colon -> enter command mode
(define-key "global" "Char::" "enter-command-mode")

;; Backspace -> delete character before cursor
(define-key "global" "Backspace" "delete-backward")

;; Activate the global keymap
;; Unbound printable characters are auto-inserted by the event loop.
(set-active-keymap "global")
