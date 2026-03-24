;;; name: status-bar
;;; version: 3.0.0
;;; description: Status bar showing filename, position, mode — pure Lisp via panel system

;; Create a bottom panel for the status bar
(define-panel "status" "bottom" 1)
(set-panel-style "status" "#cdd6f4" "#313244")

;; Build status text from editor state
(define build-status
  (lambda ()
    (let* ((pos (cursor-position))
           (line (+ (nth 0 pos) 1))
           (col (+ (nth 1 pos) 1))
           (fname (buffer-filename))
           (mod-flag (if (buffer-modified?) "  [+]" " "))
           (mode (str-upper (current-mode))))
      (str-join
        (list " " fname "  Ln " (to-string line) ", Col " (to-string col) mod-flag " " mode " ")
        ""))))

;; Update on every change
(define update-status (lambda () (set-panel-content "status" (build-status))))

(add-hook "cursor-moved" update-status)
(add-hook "buffer-changed" update-status)
(add-hook "mode-changed" update-status)

;; Initial render
(update-status)
