;;; name: rainbow-csv
;;; version: 2.0.0
;;; description: Colorizes CSV columns with rainbow colors — pure Alfred Lisp

;; 8 pastel colors cycling per column
(define csv-colors
  '("#ff6b6b" "#4ecdc4" "#45b7d1" "#96ceb4"
    "#ffeaa7" "#dda0dd" "#98d8c8" "#f7dc6f"))

;; Get color for a column index (modulo 8)
(define get-csv-color
  (lambda (idx)
    (nth (- idx (* (/ idx 8) 8)) csv-colors)))

;; Build list of (start end color) for fields in a line
(define build-field-styles
  (lambda (fields col field-idx)
    (if (nil? fields)
      '()
      (cons
        (list col
              (+ col (str-length (first fields)))
              (get-csv-color field-idx))
        (build-field-styles
          (rest fields)
          (+ col (str-length (first fields)) 1)
          (+ field-idx 1))))))

;; Colorize one line
(define colorize-csv-line
  (lambda (line-num)
    (for-each
      (lambda (seg)
        (set-line-style line-num (nth 0 seg) (nth 1 seg) (nth 2 seg)))
      (build-field-styles
        (str-split (buffer-get-line line-num) ",")
        0
        0))))

;; Main command: colorize entire buffer
(define-command "rainbow-csv"
  (lambda ()
    (clear-line-styles)
    (for-each colorize-csv-line (range 0 (buffer-line-count)))
    (message "Rainbow CSV applied")))

;; Clear command
(define-command "clear-csv-colors"
  (lambda ()
    (clear-line-styles)
    (message "CSV colors cleared")))
