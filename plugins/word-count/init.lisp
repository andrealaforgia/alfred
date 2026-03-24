;;; name: word-count
;;; version: 1.0.0
;;; description: Displays word count, line count, and character count

;; Count words by splitting buffer content on spaces and newlines
(define count-words
  (lambda ()
    (length
      (filter
        (lambda (w) (> (str-length w) 0))
        (str-split (buffer-content) " ")))))

;; Main command: show stats in the message bar
(define-command "word-count"
  (lambda ()
    (message
      (str-join
        (list
          "Lines: " (to-string (buffer-line-count))
          " | Words: " (to-string (count-words))
          " | Chars: " (to-string (str-length (buffer-content))))
        ""))))
