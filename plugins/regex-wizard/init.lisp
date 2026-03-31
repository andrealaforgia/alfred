;;; name: regex-wizard
;;; version: 0.1.0
;;; description: Regex wizard with bottom panel for live regex matching
;;; depends: default-theme

;; ---------------------------------------------------------------------------
;; Constants
;; ---------------------------------------------------------------------------

(define regex-wizard-panel-name "regex-wizard")
(define regex-wizard-panel-height 3)

;; ---------------------------------------------------------------------------
;; State variables
;; ---------------------------------------------------------------------------

(define regex-wizard-query (str-concat (list)))
(define regex-wizard-open nil)
(define regex-wizard-match-count 0)
(define regex-wizard-previous-mode (str-concat (list)))
(define regex-wizard-previous-keymap (str-concat (list)))

;; ---------------------------------------------------------------------------
;; Panel setup — created at load time, initially hidden (size 0)
;; ---------------------------------------------------------------------------

(define-panel regex-wizard-panel-name "bottom" 0)
(set-panel-style regex-wizard-panel-name theme-fg theme-surface)

;; ---------------------------------------------------------------------------
;; Helpers — pure utility functions
;; ---------------------------------------------------------------------------

;; Build the pattern display line (row 0): "Pattern: <query>"
(define regex-wizard-build-pattern-line
  (lambda ()
    (str-concat (list " Pattern: " regex-wizard-query))))

;; Build the match count display line (row 1): "N matches found"
(define regex-wizard-build-count-line
  (lambda ()
    (str-concat (list " " (to-string regex-wizard-match-count) " matches found"))))

;; Build the hint line (row 2)
(define regex-wizard-build-hint-line
  (lambda ()
    (str-concat (list " [Esc] close  |  Type regex pattern"))))

;; ---------------------------------------------------------------------------
;; Panel rendering — update all 3 rows with current state
;; ---------------------------------------------------------------------------

(define regex-wizard-render-panel
  (lambda ()
    (begin
      (clear-panel-lines regex-wizard-panel-name)
      (clear-panel-line-styles regex-wizard-panel-name)
      ;; Row 0: Pattern
      (set-panel-line regex-wizard-panel-name 0 (regex-wizard-build-pattern-line))
      (set-panel-line-style regex-wizard-panel-name 0 0
        (str-length (regex-wizard-build-pattern-line)) theme-prompt)
      ;; Row 1: Match count
      (set-panel-line regex-wizard-panel-name 1 (regex-wizard-build-count-line))
      (set-panel-line-style regex-wizard-panel-name 1 0
        (str-length (regex-wizard-build-count-line)) theme-accent)
      ;; Row 2: Hints
      (set-panel-line regex-wizard-panel-name 2 (regex-wizard-build-hint-line))
      (set-panel-line-style regex-wizard-panel-name 2 0
        (str-length (regex-wizard-build-hint-line)) theme-muted))))

;; ---------------------------------------------------------------------------
;; Search execution — validate and run regex
;; ---------------------------------------------------------------------------

(define regex-wizard-run-search
  (lambda ()
    (if (= (str-length regex-wizard-query) 0)
      (begin
        (clear-match-highlights)
        (set regex-wizard-match-count 0)
        (regex-wizard-render-panel))
      (if (regex-valid? regex-wizard-query)
        (begin
          (clear-match-highlights)
          (set regex-wizard-match-count (regex-find-all regex-wizard-query theme-accent))
          (regex-wizard-render-panel))
        (begin
          (clear-match-highlights)
          (set regex-wizard-match-count 0)
          (regex-wizard-render-panel))))))

;; ---------------------------------------------------------------------------
;; Wizard lifecycle — open and close
;; ---------------------------------------------------------------------------

;; Close wizard: clear highlights, hide panel, restore previous mode/keymap
(define regex-wizard-close
  (lambda ()
    (begin
      (clear-match-highlights)
      (set-panel-size regex-wizard-panel-name 0)
      (set regex-wizard-open nil)
      (set regex-wizard-query (str-concat (list)))
      (set regex-wizard-match-count 0)
      (set-mode regex-wizard-previous-mode)
      (set-active-keymap regex-wizard-previous-keymap))))

;; Open wizard: save current mode, show panel, switch to wizard keymap
(define regex-wizard-open-wizard
  (lambda ()
    (begin
      (set regex-wizard-previous-mode (current-mode))
      (set regex-wizard-previous-keymap "normal-mode")
      (set regex-wizard-query (str-concat (list)))
      (set regex-wizard-match-count 0)
      (set regex-wizard-open 1)
      (set-panel-size regex-wizard-panel-name regex-wizard-panel-height)
      (regex-wizard-render-panel)
      (set-active-keymap "regex-wizard-input"))))

;; ---------------------------------------------------------------------------
;; Commands
;; ---------------------------------------------------------------------------

;; Toggle open/close
(define-command "open-regex-wizard"
  (lambda ()
    (if regex-wizard-open
      (begin
        (clear-match-highlights)
        (set-panel-size regex-wizard-panel-name 0)
        (set regex-wizard-open nil)
        (set regex-wizard-query (str-concat (list)))
        (set regex-wizard-match-count 0)
        (set-mode "normal")
        (set-active-keymap "normal-mode"))
      (begin
        (set regex-wizard-query (str-concat (list)))
        (set regex-wizard-match-count 0)
        (set regex-wizard-open 1)
        (set-panel-size regex-wizard-panel-name regex-wizard-panel-height)
        (set-panel-line regex-wizard-panel-name 0
          (str-concat (list " Pattern: ")))
        (set-panel-line regex-wizard-panel-name 1
          (str-concat (list " 0 matches found")))
        (set-panel-line regex-wizard-panel-name 2
          (str-concat (list " [type pattern] [Esc: close]")))
        (set-active-keymap "regex-wizard-input")))))

;; Escape: close wizard
(define-command "regex-wizard-escape"
  (lambda ()
    (clear-match-highlights)
    (set-panel-size regex-wizard-panel-name 0)
    (set regex-wizard-open nil)
    (set regex-wizard-query (str-concat (list)))
    (set regex-wizard-match-count 0)
    (set-mode "normal")
    (set-active-keymap "normal-mode")))

;; Backspace: remove last char or close if empty
(define-command "regex-wizard-backspace"
  (lambda ()
    (if (= (str-length regex-wizard-query) 0)
      (regex-wizard-close)
      (begin
        (set regex-wizard-query
          (str-substring regex-wizard-query 0
            (- (str-length regex-wizard-query) 1)))
        (regex-wizard-run-search)))))

;; Append a character to the regex query and re-run search
(define regex-wizard-append
  (lambda (ch)
    (begin
      (set regex-wizard-query (str-concat (list regex-wizard-query ch)))
      (regex-wizard-run-search))))

;; ---------------------------------------------------------------------------
;; Keymap — per-character bindings (same pattern as overlay-search)
;; ---------------------------------------------------------------------------

(make-keymap "regex-wizard-input")
(define-key "regex-wizard-input" "Escape" "regex-wizard-escape")
(define-key "regex-wizard-input" "Backspace" "regex-wizard-backspace")
(define-key "regex-wizard-input" "Ctrl:r" "regex-wizard-escape")

;; Per-character commands for lowercase a-z
(define-command "regex-wizard-char-a" (lambda () (regex-wizard-append "a")))
(define-command "regex-wizard-char-b" (lambda () (regex-wizard-append "b")))
(define-command "regex-wizard-char-c" (lambda () (regex-wizard-append "c")))
(define-command "regex-wizard-char-d" (lambda () (regex-wizard-append "d")))
(define-command "regex-wizard-char-e" (lambda () (regex-wizard-append "e")))
(define-command "regex-wizard-char-f" (lambda () (regex-wizard-append "f")))
(define-command "regex-wizard-char-g" (lambda () (regex-wizard-append "g")))
(define-command "regex-wizard-char-h" (lambda () (regex-wizard-append "h")))
(define-command "regex-wizard-char-i" (lambda () (regex-wizard-append "i")))
(define-command "regex-wizard-char-j" (lambda () (regex-wizard-append "j")))
(define-command "regex-wizard-char-k" (lambda () (regex-wizard-append "k")))
(define-command "regex-wizard-char-l" (lambda () (regex-wizard-append "l")))
(define-command "regex-wizard-char-m" (lambda () (regex-wizard-append "m")))
(define-command "regex-wizard-char-n" (lambda () (regex-wizard-append "n")))
(define-command "regex-wizard-char-o" (lambda () (regex-wizard-append "o")))
(define-command "regex-wizard-char-p" (lambda () (regex-wizard-append "p")))
(define-command "regex-wizard-char-q" (lambda () (regex-wizard-append "q")))
(define-command "regex-wizard-char-r" (lambda () (regex-wizard-append "r")))
(define-command "regex-wizard-char-s" (lambda () (regex-wizard-append "s")))
(define-command "regex-wizard-char-t" (lambda () (regex-wizard-append "t")))
(define-command "regex-wizard-char-u" (lambda () (regex-wizard-append "u")))
(define-command "regex-wizard-char-v" (lambda () (regex-wizard-append "v")))
(define-command "regex-wizard-char-w" (lambda () (regex-wizard-append "w")))
(define-command "regex-wizard-char-x" (lambda () (regex-wizard-append "x")))
(define-command "regex-wizard-char-y" (lambda () (regex-wizard-append "y")))
(define-command "regex-wizard-char-z" (lambda () (regex-wizard-append "z")))

(define-key "regex-wizard-input" "Char:a" "regex-wizard-char-a")
(define-key "regex-wizard-input" "Char:b" "regex-wizard-char-b")
(define-key "regex-wizard-input" "Char:c" "regex-wizard-char-c")
(define-key "regex-wizard-input" "Char:d" "regex-wizard-char-d")
(define-key "regex-wizard-input" "Char:e" "regex-wizard-char-e")
(define-key "regex-wizard-input" "Char:f" "regex-wizard-char-f")
(define-key "regex-wizard-input" "Char:g" "regex-wizard-char-g")
(define-key "regex-wizard-input" "Char:h" "regex-wizard-char-h")
(define-key "regex-wizard-input" "Char:i" "regex-wizard-char-i")
(define-key "regex-wizard-input" "Char:j" "regex-wizard-char-j")
(define-key "regex-wizard-input" "Char:k" "regex-wizard-char-k")
(define-key "regex-wizard-input" "Char:l" "regex-wizard-char-l")
(define-key "regex-wizard-input" "Char:m" "regex-wizard-char-m")
(define-key "regex-wizard-input" "Char:n" "regex-wizard-char-n")
(define-key "regex-wizard-input" "Char:o" "regex-wizard-char-o")
(define-key "regex-wizard-input" "Char:p" "regex-wizard-char-p")
(define-key "regex-wizard-input" "Char:q" "regex-wizard-char-q")
(define-key "regex-wizard-input" "Char:r" "regex-wizard-char-r")
(define-key "regex-wizard-input" "Char:s" "regex-wizard-char-s")
(define-key "regex-wizard-input" "Char:t" "regex-wizard-char-t")
(define-key "regex-wizard-input" "Char:u" "regex-wizard-char-u")
(define-key "regex-wizard-input" "Char:v" "regex-wizard-char-v")
(define-key "regex-wizard-input" "Char:w" "regex-wizard-char-w")
(define-key "regex-wizard-input" "Char:x" "regex-wizard-char-x")
(define-key "regex-wizard-input" "Char:y" "regex-wizard-char-y")
(define-key "regex-wizard-input" "Char:z" "regex-wizard-char-z")

;; Per-character commands for uppercase A-Z
(define-command "regex-wizard-char-A" (lambda () (regex-wizard-append "A")))
(define-command "regex-wizard-char-B" (lambda () (regex-wizard-append "B")))
(define-command "regex-wizard-char-C" (lambda () (regex-wizard-append "C")))
(define-command "regex-wizard-char-D" (lambda () (regex-wizard-append "D")))
(define-command "regex-wizard-char-E" (lambda () (regex-wizard-append "E")))
(define-command "regex-wizard-char-F" (lambda () (regex-wizard-append "F")))
(define-command "regex-wizard-char-G" (lambda () (regex-wizard-append "G")))
(define-command "regex-wizard-char-H" (lambda () (regex-wizard-append "H")))
(define-command "regex-wizard-char-I" (lambda () (regex-wizard-append "I")))
(define-command "regex-wizard-char-J" (lambda () (regex-wizard-append "J")))
(define-command "regex-wizard-char-K" (lambda () (regex-wizard-append "K")))
(define-command "regex-wizard-char-L" (lambda () (regex-wizard-append "L")))
(define-command "regex-wizard-char-M" (lambda () (regex-wizard-append "M")))
(define-command "regex-wizard-char-N" (lambda () (regex-wizard-append "N")))
(define-command "regex-wizard-char-O" (lambda () (regex-wizard-append "O")))
(define-command "regex-wizard-char-P" (lambda () (regex-wizard-append "P")))
(define-command "regex-wizard-char-Q" (lambda () (regex-wizard-append "Q")))
(define-command "regex-wizard-char-R" (lambda () (regex-wizard-append "R")))
(define-command "regex-wizard-char-S" (lambda () (regex-wizard-append "S")))
(define-command "regex-wizard-char-T" (lambda () (regex-wizard-append "T")))
(define-command "regex-wizard-char-U" (lambda () (regex-wizard-append "U")))
(define-command "regex-wizard-char-V" (lambda () (regex-wizard-append "V")))
(define-command "regex-wizard-char-W" (lambda () (regex-wizard-append "W")))
(define-command "regex-wizard-char-X" (lambda () (regex-wizard-append "X")))
(define-command "regex-wizard-char-Y" (lambda () (regex-wizard-append "Y")))
(define-command "regex-wizard-char-Z" (lambda () (regex-wizard-append "Z")))

(define-key "regex-wizard-input" "Char:A" "regex-wizard-char-A")
(define-key "regex-wizard-input" "Char:B" "regex-wizard-char-B")
(define-key "regex-wizard-input" "Char:C" "regex-wizard-char-C")
(define-key "regex-wizard-input" "Char:D" "regex-wizard-char-D")
(define-key "regex-wizard-input" "Char:E" "regex-wizard-char-E")
(define-key "regex-wizard-input" "Char:F" "regex-wizard-char-F")
(define-key "regex-wizard-input" "Char:G" "regex-wizard-char-G")
(define-key "regex-wizard-input" "Char:H" "regex-wizard-char-H")
(define-key "regex-wizard-input" "Char:I" "regex-wizard-char-I")
(define-key "regex-wizard-input" "Char:J" "regex-wizard-char-J")
(define-key "regex-wizard-input" "Char:K" "regex-wizard-char-K")
(define-key "regex-wizard-input" "Char:L" "regex-wizard-char-L")
(define-key "regex-wizard-input" "Char:M" "regex-wizard-char-M")
(define-key "regex-wizard-input" "Char:N" "regex-wizard-char-N")
(define-key "regex-wizard-input" "Char:O" "regex-wizard-char-O")
(define-key "regex-wizard-input" "Char:P" "regex-wizard-char-P")
(define-key "regex-wizard-input" "Char:Q" "regex-wizard-char-Q")
(define-key "regex-wizard-input" "Char:R" "regex-wizard-char-R")
(define-key "regex-wizard-input" "Char:S" "regex-wizard-char-S")
(define-key "regex-wizard-input" "Char:T" "regex-wizard-char-T")
(define-key "regex-wizard-input" "Char:U" "regex-wizard-char-U")
(define-key "regex-wizard-input" "Char:V" "regex-wizard-char-V")
(define-key "regex-wizard-input" "Char:W" "regex-wizard-char-W")
(define-key "regex-wizard-input" "Char:X" "regex-wizard-char-X")
(define-key "regex-wizard-input" "Char:Y" "regex-wizard-char-Y")
(define-key "regex-wizard-input" "Char:Z" "regex-wizard-char-Z")

;; Per-character commands for digits 0-9
(define-command "regex-wizard-char-0" (lambda () (regex-wizard-append "0")))
(define-command "regex-wizard-char-1" (lambda () (regex-wizard-append "1")))
(define-command "regex-wizard-char-2" (lambda () (regex-wizard-append "2")))
(define-command "regex-wizard-char-3" (lambda () (regex-wizard-append "3")))
(define-command "regex-wizard-char-4" (lambda () (regex-wizard-append "4")))
(define-command "regex-wizard-char-5" (lambda () (regex-wizard-append "5")))
(define-command "regex-wizard-char-6" (lambda () (regex-wizard-append "6")))
(define-command "regex-wizard-char-7" (lambda () (regex-wizard-append "7")))
(define-command "regex-wizard-char-8" (lambda () (regex-wizard-append "8")))
(define-command "regex-wizard-char-9" (lambda () (regex-wizard-append "9")))

(define-key "regex-wizard-input" "Char:0" "regex-wizard-char-0")
(define-key "regex-wizard-input" "Char:1" "regex-wizard-char-1")
(define-key "regex-wizard-input" "Char:2" "regex-wizard-char-2")
(define-key "regex-wizard-input" "Char:3" "regex-wizard-char-3")
(define-key "regex-wizard-input" "Char:4" "regex-wizard-char-4")
(define-key "regex-wizard-input" "Char:5" "regex-wizard-char-5")
(define-key "regex-wizard-input" "Char:6" "regex-wizard-char-6")
(define-key "regex-wizard-input" "Char:7" "regex-wizard-char-7")
(define-key "regex-wizard-input" "Char:8" "regex-wizard-char-8")
(define-key "regex-wizard-input" "Char:9" "regex-wizard-char-9")

;; Per-character commands for regex-relevant symbols
(define-command "regex-wizard-char-dot" (lambda () (regex-wizard-append ".")))
(define-command "regex-wizard-char-star" (lambda () (regex-wizard-append "*")))
(define-command "regex-wizard-char-plus" (lambda () (regex-wizard-append "+")))
(define-command "regex-wizard-char-question" (lambda () (regex-wizard-append "?")))
(define-command "regex-wizard-char-pipe" (lambda () (regex-wizard-append "|")))
(define-command "regex-wizard-char-caret" (lambda () (regex-wizard-append "^")))
(define-command "regex-wizard-char-dollar" (lambda () (regex-wizard-append "$")))
(define-command "regex-wizard-char-backslash" (lambda () (regex-wizard-append backslash-char)))
(define-command "regex-wizard-char-lbracket" (lambda () (regex-wizard-append "[")))
(define-command "regex-wizard-char-rbracket" (lambda () (regex-wizard-append "]")))
(define-command "regex-wizard-char-lparen" (lambda () (regex-wizard-append "(")))
(define-command "regex-wizard-char-rparen" (lambda () (regex-wizard-append ")")))
(define-command "regex-wizard-char-lbrace" (lambda () (regex-wizard-append "{")))
(define-command "regex-wizard-char-rbrace" (lambda () (regex-wizard-append "}")))
(define-command "regex-wizard-char-dash" (lambda () (regex-wizard-append "-")))
(define-command "regex-wizard-char-underscore" (lambda () (regex-wizard-append "_")))
(define-command "regex-wizard-char-space" (lambda () (regex-wizard-append " ")))
(define-command "regex-wizard-char-colon" (lambda () (regex-wizard-append ":")))
(define-command "regex-wizard-char-semicolon" (lambda () (regex-wizard-append ";")))
(define-command "regex-wizard-char-comma" (lambda () (regex-wizard-append ",")))
(define-command "regex-wizard-char-at" (lambda () (regex-wizard-append "@")))
(define-command "regex-wizard-char-hash" (lambda () (regex-wizard-append "#")))
(define-command "regex-wizard-char-ampersand" (lambda () (regex-wizard-append "&")))
(define-command "regex-wizard-char-equals" (lambda () (regex-wizard-append "=")))
(define-command "regex-wizard-char-exclaim" (lambda () (regex-wizard-append "!")))
(define-command "regex-wizard-char-tilde" (lambda () (regex-wizard-append "~")))
(define-command "regex-wizard-char-slash" (lambda () (regex-wizard-append "/")))
(define-command "regex-wizard-char-percent" (lambda () (regex-wizard-append "%")))
(define-command "regex-wizard-char-singlequote" (lambda () (regex-wizard-append "'")))
(define-command "regex-wizard-char-doublequote" (lambda () (regex-wizard-append double-quote)))
(define-command "regex-wizard-char-backtick" (lambda () (regex-wizard-append "`")))
(define-command "regex-wizard-char-lt" (lambda () (regex-wizard-append "<")))
(define-command "regex-wizard-char-gt" (lambda () (regex-wizard-append ">")))

(define-key "regex-wizard-input" "Char:." "regex-wizard-char-dot")
(define-key "regex-wizard-input" "Char:*" "regex-wizard-char-star")
(define-key "regex-wizard-input" "Char:+" "regex-wizard-char-plus")
(define-key "regex-wizard-input" "Char:?" "regex-wizard-char-question")
(define-key "regex-wizard-input" "Char:|" "regex-wizard-char-pipe")
(define-key "regex-wizard-input" "Char:^" "regex-wizard-char-caret")
(define-key "regex-wizard-input" "Char:$" "regex-wizard-char-dollar")
(define-key "regex-wizard-input" (str-concat (list "Char:" backslash-char)) "regex-wizard-char-backslash")
(define-key "regex-wizard-input" "Char:[" "regex-wizard-char-lbracket")
(define-key "regex-wizard-input" "Char:]" "regex-wizard-char-rbracket")
(define-key "regex-wizard-input" "Char:(" "regex-wizard-char-lparen")
(define-key "regex-wizard-input" "Char:)" "regex-wizard-char-rparen")
(define-key "regex-wizard-input" "Char:{" "regex-wizard-char-lbrace")
(define-key "regex-wizard-input" "Char:}" "regex-wizard-char-rbrace")
(define-key "regex-wizard-input" "Char:-" "regex-wizard-char-dash")
(define-key "regex-wizard-input" "Char:_" "regex-wizard-char-underscore")
(define-key "regex-wizard-input" "Char: " "regex-wizard-char-space")
(define-key "regex-wizard-input" "Char::" "regex-wizard-char-colon")
(define-key "regex-wizard-input" "Char:;" "regex-wizard-char-semicolon")
(define-key "regex-wizard-input" "Char:," "regex-wizard-char-comma")
(define-key "regex-wizard-input" "Char:@" "regex-wizard-char-at")
(define-key "regex-wizard-input" "Char:#" "regex-wizard-char-hash")
(define-key "regex-wizard-input" "Char:&" "regex-wizard-char-ampersand")
(define-key "regex-wizard-input" "Char:=" "regex-wizard-char-equals")
(define-key "regex-wizard-input" "Char:!" "regex-wizard-char-exclaim")
(define-key "regex-wizard-input" "Char:~" "regex-wizard-char-tilde")
(define-key "regex-wizard-input" "Char:/" "regex-wizard-char-slash")
(define-key "regex-wizard-input" "Char:%" "regex-wizard-char-percent")
(define-key "regex-wizard-input" "SingleQuote" "regex-wizard-char-singlequote")
(define-key "regex-wizard-input" "DoubleQuote" "regex-wizard-char-doublequote")
(define-key "regex-wizard-input" "Char:`" "regex-wizard-char-backtick")
(define-key "regex-wizard-input" "Char:<" "regex-wizard-char-lt")
(define-key "regex-wizard-input" "Char:>" "regex-wizard-char-gt")

;; Alias: :regex opens the wizard (inline logic — define-command
;; callbacks have limited access to Lisp-defined functions)
(define-command "regex"
  (lambda ()
    (if regex-wizard-open
      (begin
        (clear-match-highlights)
        (set-panel-size regex-wizard-panel-name 0)
        (set regex-wizard-open nil)
        (set regex-wizard-query (str-concat (list)))
        (set regex-wizard-match-count 0)
        (set-mode "normal")
        (set-active-keymap "normal-mode"))
      (begin
        (set regex-wizard-query (str-concat (list)))
        (set regex-wizard-match-count 0)
        (set regex-wizard-open 1)
        (set-panel-size regex-wizard-panel-name regex-wizard-panel-height)
        (set-panel-line regex-wizard-panel-name 0
          (str-concat (list " Pattern: ")))
        (set-panel-line regex-wizard-panel-name 1
          (str-concat (list " 0 matches found")))
        (set-panel-line regex-wizard-panel-name 2
          (str-concat (list " [type pattern] [Esc: close]")))
        (set-active-keymap "regex-wizard-input")))))
