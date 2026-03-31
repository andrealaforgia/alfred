;;; name: overlay-search
;;; version: 0.1.0
;;; description: Overlay file search triggered by Ctrl-p
;;; depends: browse-mode

;; ---------------------------------------------------------------------------
;; Constants
;; ---------------------------------------------------------------------------

(define overlay-search-width 65)
(define overlay-search-max-items 15)
(define overlay-search-empty-str (str-concat (list)))

;; ---------------------------------------------------------------------------
;; State variables
;; ---------------------------------------------------------------------------

(define overlay-search-query (str-concat (list)))
(define overlay-search-all-files (list))
(define overlay-search-open nil)

;; ---------------------------------------------------------------------------
;; Helpers -- pure utility functions
;; ---------------------------------------------------------------------------

;; Check if an entry is a file (not a directory)
(define overlay-search-is-file
  (lambda (entry)
    (= (nth 1 entry) "file")))

;; Extract file paths from recursive directory listing
(define overlay-search-extract-files
  (lambda (entries)
    (map first (filter overlay-search-is-file entries))))

;; Check if a file path contains the current query (case-insensitive).
;; Reads overlay-search-query directly to avoid nested lambda closures
;; which trigger a rust_lisp bug with captured local variables.
(define overlay-search-matches-query
  (lambda (filepath)
    (str-contains (str-lower filepath) (str-lower overlay-search-query))))

;; Filter file list by current query substring match
(define overlay-search-filter
  (lambda (files)
    (if (= (str-length overlay-search-query) 0)
      files
      (filter overlay-search-matches-query files))))

;; ---------------------------------------------------------------------------
;; Overlay lifecycle
;; ---------------------------------------------------------------------------

;; Close overlay and restore browser panel mode
(define overlay-search-close
  (lambda ()
    (close-overlay)
    (set overlay-search-open nil)
    (set-mode "panel-browse")
    (set-active-keymap "browser-panel-mode")))

;; Update overlay items based on current query
(define overlay-search-update-results
  (lambda ()
    (overlay-set-items
      (overlay-search-filter overlay-search-all-files))
    (overlay-set-input overlay-search-query)))

;; ---------------------------------------------------------------------------
;; Commands
;; ---------------------------------------------------------------------------

;; Open the overlay file search
(define-command "overlay-file-search"
  (lambda ()
    (if overlay-search-open
      (overlay-search-close)
      (begin
        (set overlay-search-open 1)
        (set overlay-search-query (str-concat (list)))
        (set overlay-search-all-files
          (overlay-search-extract-files (list-dir-recursive browser-root-dir)))
        (open-overlay overlay-search-width overlay-search-max-items)
        (overlay-set-items overlay-search-all-files)
        (overlay-set-input overlay-search-query)
        (set-active-keymap "overlay-search-input")))))

;; Enter: open selected file and close overlay
(define-command "overlay-search-enter"
  (lambda ()
    (if (= (overlay-get-selected) overlay-search-empty-str)
      (overlay-search-close)
      (begin
        (open-file (path-join browser-root-dir (overlay-get-selected)))
        (close-overlay)
        (set overlay-search-open nil)))))

;; Escape: close overlay without action
(define-command "overlay-search-escape"
  (lambda ()
    (overlay-search-close)))

;; Backspace: remove last query character or close if empty
(define-command "overlay-search-backspace"
  (lambda ()
    (if (= (str-length overlay-search-query) 0)
      (overlay-search-close)
      (begin
        (set overlay-search-query
          (str-substring overlay-search-query 0
            (- (str-length overlay-search-query) 1)))
        (overlay-search-update-results)))))

;; Append a character to the search query and re-filter
(define overlay-search-append
  (lambda (ch)
    (set overlay-search-query (str-concat (list overlay-search-query ch)))
    (overlay-search-update-results)))

;; ---------------------------------------------------------------------------
;; Keymap
;; ---------------------------------------------------------------------------

;; Cursor navigation commands
(define-command "overlay-search-cursor-down"
  (lambda () (overlay-cursor-down)))
(define-command "overlay-search-cursor-up"
  (lambda () (overlay-cursor-up)))

(make-keymap "overlay-search-input")
(define-key "overlay-search-input" "Escape" "overlay-search-escape")
(define-key "overlay-search-input" "Enter" "overlay-search-enter")
(define-key "overlay-search-input" "Backspace" "overlay-search-backspace")
(define-key "overlay-search-input" "Ctrl:p" "overlay-search-escape")
(define-key "overlay-search-input" "Down" "overlay-search-cursor-down")
(define-key "overlay-search-input" "Up" "overlay-search-cursor-up")

;; Per-character commands for lowercase a-z
(define-command "overlay-search-char-a" (lambda () (overlay-search-append "a")))
(define-command "overlay-search-char-b" (lambda () (overlay-search-append "b")))
(define-command "overlay-search-char-c" (lambda () (overlay-search-append "c")))
(define-command "overlay-search-char-d" (lambda () (overlay-search-append "d")))
(define-command "overlay-search-char-e" (lambda () (overlay-search-append "e")))
(define-command "overlay-search-char-f" (lambda () (overlay-search-append "f")))
(define-command "overlay-search-char-g" (lambda () (overlay-search-append "g")))
(define-command "overlay-search-char-h" (lambda () (overlay-search-append "h")))
(define-command "overlay-search-char-i" (lambda () (overlay-search-append "i")))
(define-command "overlay-search-char-j" (lambda () (overlay-search-append "j")))
(define-command "overlay-search-char-k" (lambda () (overlay-search-append "k")))
(define-command "overlay-search-char-l" (lambda () (overlay-search-append "l")))
(define-command "overlay-search-char-m" (lambda () (overlay-search-append "m")))
(define-command "overlay-search-char-n" (lambda () (overlay-search-append "n")))
(define-command "overlay-search-char-o" (lambda () (overlay-search-append "o")))
(define-command "overlay-search-char-p" (lambda () (overlay-search-append "p")))
(define-command "overlay-search-char-q" (lambda () (overlay-search-append "q")))
(define-command "overlay-search-char-r" (lambda () (overlay-search-append "r")))
(define-command "overlay-search-char-s" (lambda () (overlay-search-append "s")))
(define-command "overlay-search-char-t" (lambda () (overlay-search-append "t")))
(define-command "overlay-search-char-u" (lambda () (overlay-search-append "u")))
(define-command "overlay-search-char-v" (lambda () (overlay-search-append "v")))
(define-command "overlay-search-char-w" (lambda () (overlay-search-append "w")))
(define-command "overlay-search-char-x" (lambda () (overlay-search-append "x")))
(define-command "overlay-search-char-y" (lambda () (overlay-search-append "y")))
(define-command "overlay-search-char-z" (lambda () (overlay-search-append "z")))

(define-key "overlay-search-input" "Char:a" "overlay-search-char-a")
(define-key "overlay-search-input" "Char:b" "overlay-search-char-b")
(define-key "overlay-search-input" "Char:c" "overlay-search-char-c")
(define-key "overlay-search-input" "Char:d" "overlay-search-char-d")
(define-key "overlay-search-input" "Char:e" "overlay-search-char-e")
(define-key "overlay-search-input" "Char:f" "overlay-search-char-f")
(define-key "overlay-search-input" "Char:g" "overlay-search-char-g")
(define-key "overlay-search-input" "Char:h" "overlay-search-char-h")
(define-key "overlay-search-input" "Char:i" "overlay-search-char-i")
(define-key "overlay-search-input" "Char:j" "overlay-search-char-j")
(define-key "overlay-search-input" "Char:k" "overlay-search-char-k")
(define-key "overlay-search-input" "Char:l" "overlay-search-char-l")
(define-key "overlay-search-input" "Char:m" "overlay-search-char-m")
(define-key "overlay-search-input" "Char:n" "overlay-search-char-n")
(define-key "overlay-search-input" "Char:o" "overlay-search-char-o")
(define-key "overlay-search-input" "Char:p" "overlay-search-char-p")
(define-key "overlay-search-input" "Char:q" "overlay-search-char-q")
(define-key "overlay-search-input" "Char:r" "overlay-search-char-r")
(define-key "overlay-search-input" "Char:s" "overlay-search-char-s")
(define-key "overlay-search-input" "Char:t" "overlay-search-char-t")
(define-key "overlay-search-input" "Char:u" "overlay-search-char-u")
(define-key "overlay-search-input" "Char:v" "overlay-search-char-v")
(define-key "overlay-search-input" "Char:w" "overlay-search-char-w")
(define-key "overlay-search-input" "Char:x" "overlay-search-char-x")
(define-key "overlay-search-input" "Char:y" "overlay-search-char-y")
(define-key "overlay-search-input" "Char:z" "overlay-search-char-z")

;; Per-character commands for uppercase A-Z
(define-command "overlay-search-char-A" (lambda () (overlay-search-append "A")))
(define-command "overlay-search-char-B" (lambda () (overlay-search-append "B")))
(define-command "overlay-search-char-C" (lambda () (overlay-search-append "C")))
(define-command "overlay-search-char-D" (lambda () (overlay-search-append "D")))
(define-command "overlay-search-char-E" (lambda () (overlay-search-append "E")))
(define-command "overlay-search-char-F" (lambda () (overlay-search-append "F")))
(define-command "overlay-search-char-G" (lambda () (overlay-search-append "G")))
(define-command "overlay-search-char-H" (lambda () (overlay-search-append "H")))
(define-command "overlay-search-char-I" (lambda () (overlay-search-append "I")))
(define-command "overlay-search-char-J" (lambda () (overlay-search-append "J")))
(define-command "overlay-search-char-K" (lambda () (overlay-search-append "K")))
(define-command "overlay-search-char-L" (lambda () (overlay-search-append "L")))
(define-command "overlay-search-char-M" (lambda () (overlay-search-append "M")))
(define-command "overlay-search-char-N" (lambda () (overlay-search-append "N")))
(define-command "overlay-search-char-O" (lambda () (overlay-search-append "O")))
(define-command "overlay-search-char-P" (lambda () (overlay-search-append "P")))
(define-command "overlay-search-char-Q" (lambda () (overlay-search-append "Q")))
(define-command "overlay-search-char-R" (lambda () (overlay-search-append "R")))
(define-command "overlay-search-char-S" (lambda () (overlay-search-append "S")))
(define-command "overlay-search-char-T" (lambda () (overlay-search-append "T")))
(define-command "overlay-search-char-U" (lambda () (overlay-search-append "U")))
(define-command "overlay-search-char-V" (lambda () (overlay-search-append "V")))
(define-command "overlay-search-char-W" (lambda () (overlay-search-append "W")))
(define-command "overlay-search-char-X" (lambda () (overlay-search-append "X")))
(define-command "overlay-search-char-Y" (lambda () (overlay-search-append "Y")))
(define-command "overlay-search-char-Z" (lambda () (overlay-search-append "Z")))

(define-key "overlay-search-input" "Char:A" "overlay-search-char-A")
(define-key "overlay-search-input" "Char:B" "overlay-search-char-B")
(define-key "overlay-search-input" "Char:C" "overlay-search-char-C")
(define-key "overlay-search-input" "Char:D" "overlay-search-char-D")
(define-key "overlay-search-input" "Char:E" "overlay-search-char-E")
(define-key "overlay-search-input" "Char:F" "overlay-search-char-F")
(define-key "overlay-search-input" "Char:G" "overlay-search-char-G")
(define-key "overlay-search-input" "Char:H" "overlay-search-char-H")
(define-key "overlay-search-input" "Char:I" "overlay-search-char-I")
(define-key "overlay-search-input" "Char:J" "overlay-search-char-J")
(define-key "overlay-search-input" "Char:K" "overlay-search-char-K")
(define-key "overlay-search-input" "Char:L" "overlay-search-char-L")
(define-key "overlay-search-input" "Char:M" "overlay-search-char-M")
(define-key "overlay-search-input" "Char:N" "overlay-search-char-N")
(define-key "overlay-search-input" "Char:O" "overlay-search-char-O")
(define-key "overlay-search-input" "Char:P" "overlay-search-char-P")
(define-key "overlay-search-input" "Char:Q" "overlay-search-char-Q")
(define-key "overlay-search-input" "Char:R" "overlay-search-char-R")
(define-key "overlay-search-input" "Char:S" "overlay-search-char-S")
(define-key "overlay-search-input" "Char:T" "overlay-search-char-T")
(define-key "overlay-search-input" "Char:U" "overlay-search-char-U")
(define-key "overlay-search-input" "Char:V" "overlay-search-char-V")
(define-key "overlay-search-input" "Char:W" "overlay-search-char-W")
(define-key "overlay-search-input" "Char:X" "overlay-search-char-X")
(define-key "overlay-search-input" "Char:Y" "overlay-search-char-Y")
(define-key "overlay-search-input" "Char:Z" "overlay-search-char-Z")

;; Per-character commands for digits 0-9
(define-command "overlay-search-char-0" (lambda () (overlay-search-append "0")))
(define-command "overlay-search-char-1" (lambda () (overlay-search-append "1")))
(define-command "overlay-search-char-2" (lambda () (overlay-search-append "2")))
(define-command "overlay-search-char-3" (lambda () (overlay-search-append "3")))
(define-command "overlay-search-char-4" (lambda () (overlay-search-append "4")))
(define-command "overlay-search-char-5" (lambda () (overlay-search-append "5")))
(define-command "overlay-search-char-6" (lambda () (overlay-search-append "6")))
(define-command "overlay-search-char-7" (lambda () (overlay-search-append "7")))
(define-command "overlay-search-char-8" (lambda () (overlay-search-append "8")))
(define-command "overlay-search-char-9" (lambda () (overlay-search-append "9")))

(define-key "overlay-search-input" "Char:0" "overlay-search-char-0")
(define-key "overlay-search-input" "Char:1" "overlay-search-char-1")
(define-key "overlay-search-input" "Char:2" "overlay-search-char-2")
(define-key "overlay-search-input" "Char:3" "overlay-search-char-3")
(define-key "overlay-search-input" "Char:4" "overlay-search-char-4")
(define-key "overlay-search-input" "Char:5" "overlay-search-char-5")
(define-key "overlay-search-input" "Char:6" "overlay-search-char-6")
(define-key "overlay-search-input" "Char:7" "overlay-search-char-7")
(define-key "overlay-search-input" "Char:8" "overlay-search-char-8")
(define-key "overlay-search-input" "Char:9" "overlay-search-char-9")

;; Per-character commands for common filename symbols
(define-command "overlay-search-char-dash" (lambda () (overlay-search-append "-")))
(define-command "overlay-search-char-underscore" (lambda () (overlay-search-append "_")))
(define-command "overlay-search-char-dot" (lambda () (overlay-search-append ".")))
(define-command "overlay-search-char-slash" (lambda () (overlay-search-append "/")))
(define-command "overlay-search-char-space" (lambda () (overlay-search-append " ")))

(define-key "overlay-search-input" "Char:-" "overlay-search-char-dash")
(define-key "overlay-search-input" "Char:_" "overlay-search-char-underscore")
(define-key "overlay-search-input" "Char:." "overlay-search-char-dot")
(define-key "overlay-search-input" "Char:/" "overlay-search-char-slash")
(define-key "overlay-search-input" "Char: " "overlay-search-char-space")

;; Ctrl-p from normal mode (editor focused, no browser panel)
(define-key "normal-mode" "Ctrl:p" "overlay-file-search")
