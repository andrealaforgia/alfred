;;; name: line-numbers
;;; version: 3.0.0
;;; description: Line numbers in the gutter — pure Lisp via panel system

;; Compute gutter width based on total line count
(define compute-gutter-width
  (lambda ()
    (+ (str-length (to-string (buffer-line-count))) 1)))

;; Create a left panel for line numbers
(define-panel "gutter" "left" 4)
(set-panel-style "gutter" "#6c7086" "default")

;; Update gutter content for visible lines
(define update-gutter
  (lambda ()
    (set-panel-size "gutter" (compute-gutter-width))
    (for-each
      (lambda (i)
        (if (< (+ (viewport-top-line) i) (buffer-line-count))
          (set-panel-line "gutter" i
            (to-string (+ (viewport-top-line) i 1)))
          (set-panel-line "gutter" i "~")))
      (range 0 (viewport-height)))))

(add-hook "cursor-moved" update-gutter)
(add-hook "buffer-changed" update-gutter)

;; Initial render
(update-gutter)
