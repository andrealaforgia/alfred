;;; name: rainbow-csv
;;; version: 0.1.0
;;; description: Colorizes CSV columns with rainbow colors for easier reading

;; Register the :rainbow-csv command.
;; When invoked, it parses the current buffer as comma-separated values
;; and applies a rotating color palette to each column.
(define-command "rainbow-csv" (lambda () (rainbow-csv-colorize)))

;; Register the :clear-csv-colors command to remove colorization.
(define-command "clear-csv-colors" (lambda () (clear-line-styles)))
