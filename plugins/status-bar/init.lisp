;;; name: status-bar
;;; version: 3.0.0
;;; description: Status bar showing filename, position, mode — pure Lisp via panel system

;; Create a bottom panel for the status bar
(define-panel "status" "bottom" 1)
(set-panel-style "status" "#cdd6f4" "#313244")

;; Build status text from editor state
(define build-status
  (lambda ()
    (str-concat
      (list
        " "
        (buffer-filename)
        "  Ln "
        (to-string (+ (nth 0 (cursor-position)) 1))
        ", Col "
        (to-string (+ (nth 1 (cursor-position)) 1))
        (if (buffer-modified?) "  [+]" " ")
        " "
        (str-upper (current-mode))
        " "))))

;; Update on every change
(define update-status (lambda () (set-panel-content "status" (build-status))))

(add-hook "cursor-moved" update-status)
(add-hook "buffer-changed" update-status)
(add-hook "mode-changed" update-status)

;; Initial render
(update-status)
