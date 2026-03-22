;;; name: line-numbers
;;; version: 0.1.0
;;; description: Displays line numbers in the gutter

;; Register a callback for the render-gutter hook.
;; The callback receives start_line, end_line, and total_lines as string args.
;; The actual line number formatting is done in Rust (compute_gutter_content)
;; -- this hook's presence signals that line numbers should be displayed.
(add-hook "render-gutter" (lambda (start end total) start))
