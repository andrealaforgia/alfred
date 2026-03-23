;;; name: status-bar
;;; version: 0.1.0
;;; description: Displays a status bar with filename, cursor position, modified indicator, and mode

;; Register a callback for the render-status hook.
;; The callback's presence signals that the status bar should be displayed.
;; The actual formatting is done in Rust (compute_status_content) --
;; this hook just activates the status bar feature.
(add-hook "render-status" (lambda () "status-bar-active"))
