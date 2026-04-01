;;; name: regex-wizard
;;; version: 0.3.0
;;; description: Regex wizard with component picker and live regex matching
;;; depends: default-theme

;; ---------------------------------------------------------------------------
;; Constants
;; ---------------------------------------------------------------------------

(define regex-wizard-panel-name "regex-wizard")
(define regex-wizard-panel-height 10)
(define regex-wizard-tab-count 4)

;; ---------------------------------------------------------------------------
;; Component data -- parallel lists per tab
;; ---------------------------------------------------------------------------

;; Tab names
(define regex-wizard-tab-names
  (list "Character Classes" "Quantifiers" "Assertions" "Groups"))

;; Tab 0: Character Classes -- labels
(define regex-wizard-tab0-labels
  (list "Any character (.)"
        "Digit (\\d)"
        "Non-digit (\\D)"
        "Word character (\\w)"
        "Non-word (\\W)"
        "Whitespace (\\s)"
        "Non-whitespace (\\S)"))

;; Tab 0: Character Classes -- patterns
(define regex-wizard-tab0-patterns
  (list "."
        (str-concat (list backslash-char "d"))
        (str-concat (list backslash-char "D"))
        (str-concat (list backslash-char "w"))
        (str-concat (list backslash-char "W"))
        (str-concat (list backslash-char "s"))
        (str-concat (list backslash-char "S"))))

;; Tab 1: Quantifiers -- labels
(define regex-wizard-tab1-labels
  (list "Zero or more (*)"
        "One or more (+)"
        "Zero or one (?)"))

;; Tab 1: Quantifiers -- patterns
(define regex-wizard-tab1-patterns
  (list "*" "+" "?"))

;; Tab 2: Assertions -- labels
(define regex-wizard-tab2-labels
  (list "Start of line (^)"
        "End of line ($)"
        "Word boundary (\\b)"
        "Non-word boundary (\\B)"))

;; Tab 2: Assertions -- patterns
(define regex-wizard-tab2-patterns
  (list "^"
        "$"
        (str-concat (list backslash-char "b"))
        (str-concat (list backslash-char "B"))))

;; Tab 3: Groups -- labels
(define regex-wizard-tab3-labels
  (list "Capturing group ()"
        "Non-capturing (?:)"
        "Alternation (|)"))

;; Tab 3: Groups -- patterns
(define regex-wizard-tab3-patterns
  (list "()" "(?:)" "|"))

;; ---------------------------------------------------------------------------
;; State variables
;; ---------------------------------------------------------------------------

(define regex-wizard-query (str-concat (list)))
(define regex-wizard-open nil)
(define regex-wizard-match-count 0)
(define regex-wizard-previous-mode (str-concat (list)))
(define regex-wizard-previous-keymap (str-concat (list)))
(define regex-wizard-tab-index 0)
(define regex-wizard-cursor 0)

;; Temporary variables used by inline panel rendering inside define-command
(define regex-wizard-tmp-labels (list))
(define regex-wizard-tmp-size 0)
(define regex-wizard-tmp-line (str-concat (list)))

;; ---------------------------------------------------------------------------
;; Panel setup -- created at load time, initially hidden (size 0)
;; ---------------------------------------------------------------------------

(define-panel regex-wizard-panel-name "bottom" 0)
(set-panel-style regex-wizard-panel-name theme-fg theme-surface)

;; ---------------------------------------------------------------------------
;; Helpers -- get labels/patterns for current tab
;; (kept for reference but NOT called from define-command callbacks)
;; ---------------------------------------------------------------------------

(define regex-wizard-current-labels
  (lambda ()
    (if (= regex-wizard-tab-index 0) regex-wizard-tab0-labels
      (if (= regex-wizard-tab-index 1) regex-wizard-tab1-labels
        (if (= regex-wizard-tab-index 2) regex-wizard-tab2-labels
          regex-wizard-tab3-labels)))))

(define regex-wizard-current-patterns
  (lambda ()
    (if (= regex-wizard-tab-index 0) regex-wizard-tab0-patterns
      (if (= regex-wizard-tab-index 1) regex-wizard-tab1-patterns
        (if (= regex-wizard-tab-index 2) regex-wizard-tab2-patterns
          regex-wizard-tab3-patterns)))))

(define regex-wizard-current-tab-size
  (lambda ()
    (length (regex-wizard-current-labels))))

;; ---------------------------------------------------------------------------
;; Rendering helpers -- kept for reference only
;; ---------------------------------------------------------------------------

(define regex-wizard-build-pattern-line
  (lambda ()
    (str-concat (list " Pattern: " regex-wizard-query " | " (to-string regex-wizard-match-count) " matches"))))

(define regex-wizard-build-tab-bar
  (lambda ()
    (str-concat
      (list " "
        (if (= regex-wizard-tab-index 0) "[>Character<]" " [Character] ")
        (if (= regex-wizard-tab-index 1) "[>Quantifier<]" " [Quantifier] ")
        (if (= regex-wizard-tab-index 2) "[>Assertion<]" " [Assertion] ")
        (if (= regex-wizard-tab-index 3) "[>Group<]" " [Group] ")))))

(define regex-wizard-build-hint-line
  (lambda ()
    (str-concat (list " " (to-string regex-wizard-match-count)
      " matches | [Enter] add [BS] remove [Tab] next [Esc] close"))))

(define regex-wizard-render-item-line
  (lambda (index label)
    (if (= index regex-wizard-cursor)
      (str-concat (list " > " label))
      (str-concat (list "   " label)))))

(define regex-wizard-render-panel
  (lambda ()
    (begin
      (clear-panel-lines regex-wizard-panel-name)
      (clear-panel-line-styles regex-wizard-panel-name)

      (set-panel-line regex-wizard-panel-name 0 (regex-wizard-build-pattern-line))
      (set-panel-line-style regex-wizard-panel-name 0 0
        (str-length (regex-wizard-build-pattern-line)) theme-prompt)

      (set-panel-line regex-wizard-panel-name 1 (regex-wizard-build-tab-bar))
      (set-panel-line-style regex-wizard-panel-name 1 0
        (str-length (regex-wizard-build-tab-bar)) theme-accent)

      (set regex-wizard-render-idx 0)
      (set regex-wizard-render-labels (regex-wizard-current-labels))
      (regex-wizard-render-items)

      (set-panel-line regex-wizard-panel-name 9 (regex-wizard-build-hint-line))
      (set-panel-line-style regex-wizard-panel-name 9 0
        (str-length (regex-wizard-build-hint-line)) theme-muted))))

(define regex-wizard-render-idx 0)
(define regex-wizard-render-labels (list))

(define regex-wizard-render-items
  (lambda ()
    (if (>= regex-wizard-render-idx (length regex-wizard-render-labels))
      nil
      (begin
        (set-panel-line regex-wizard-panel-name
          (+ 2 regex-wizard-render-idx)
          (regex-wizard-render-item-line
            regex-wizard-render-idx
            (nth regex-wizard-render-idx regex-wizard-render-labels)))
        (if (= regex-wizard-render-idx regex-wizard-cursor)
          (set-panel-line-style regex-wizard-panel-name
            (+ 2 regex-wizard-render-idx)
            0
            (str-length
              (regex-wizard-render-item-line
                regex-wizard-render-idx
                (nth regex-wizard-render-idx regex-wizard-render-labels)))
            theme-highlight-bg)
          nil)
        (set regex-wizard-render-idx (+ regex-wizard-render-idx 1))
        (regex-wizard-render-items)))))

;; ---------------------------------------------------------------------------
;; Lifecycle helpers -- kept for reference only
;; ---------------------------------------------------------------------------

(define regex-wizard-close
  (lambda ()
    (begin
      (clear-match-highlights)
      (set-panel-size regex-wizard-panel-name 0)
      (set regex-wizard-open nil)
      (set regex-wizard-query (str-concat (list)))
      (set regex-wizard-match-count 0)
      (set regex-wizard-tab-index 0)
      (set regex-wizard-cursor 0)
      (set-mode regex-wizard-previous-mode)
      (set-active-keymap regex-wizard-previous-keymap))))

(define regex-wizard-open-wizard
  (lambda ()
    (begin
      (set regex-wizard-previous-mode (current-mode))
      (set regex-wizard-previous-keymap "normal-mode")
      (set regex-wizard-query (str-concat (list)))
      (set regex-wizard-match-count 0)
      (set regex-wizard-tab-index 0)
      (set regex-wizard-cursor 0)
      (set regex-wizard-open 1)
      (set-panel-size regex-wizard-panel-name regex-wizard-panel-height)
      (regex-wizard-render-panel)
      (set-active-keymap "regex-wizard-input"))))

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

;; ===========================================================================
;; COMMANDS -- All logic inlined using only bridge primitives and globals.
;;
;; define-command callbacks CANNOT call Lisp-defined functions.
;; They CAN: read/write global variables, call bridge primitives,
;; use if/begin/str-concat/set/nth/length.
;; ===========================================================================

;; ---------------------------------------------------------------------------
;; Inline full panel render macro (used in open, tab switch, cursor move)
;;
;; Steps:
;;   1. Clear all panel lines and styles
;;   2. Row 0: pattern line
;;   3. Row 1: tab bar
;;   4. Rows 2-8: component items (unrolled loop, max 7 items)
;;   5. Row 9: hint line
;; ---------------------------------------------------------------------------

;; Toggle open/close
(define-command "open-regex-wizard"
  (lambda ()
    (if regex-wizard-open
      ;; Inline close
      (begin
        (clear-match-highlights)
        (set-panel-size regex-wizard-panel-name 0)
        (set regex-wizard-open nil)
        (set regex-wizard-query (str-concat (list)))
        (set regex-wizard-match-count 0)
        (set regex-wizard-tab-index 0)
        (set regex-wizard-cursor 0)
        (set-mode regex-wizard-previous-mode)
        (set-active-keymap regex-wizard-previous-keymap))
      ;; Inline open
      (begin
        (set regex-wizard-previous-mode (current-mode))
        (set regex-wizard-previous-keymap "normal-mode")
        (set regex-wizard-query (str-concat (list)))
        (set regex-wizard-match-count 0)
        (set regex-wizard-tab-index 0)
        (set regex-wizard-cursor 0)
        (set regex-wizard-open 1)
        (set-panel-size regex-wizard-panel-name regex-wizard-panel-height)
        ;; Inline full panel render
        (clear-panel-lines regex-wizard-panel-name)
        (clear-panel-line-styles regex-wizard-panel-name)
        ;; Row 0: pattern
        (set regex-wizard-tmp-line
          (str-concat (list " Pattern: " regex-wizard-query " | " (to-string regex-wizard-match-count) " matches")))
        (set-panel-line regex-wizard-panel-name 0 regex-wizard-tmp-line)
        (set-panel-line-style regex-wizard-panel-name 0 0
          (str-length regex-wizard-tmp-line) theme-prompt)
        ;; Row 1: tab bar
        (set regex-wizard-tmp-line
          (str-concat
            (list " "
              (if (= regex-wizard-tab-index 0) "[>Character<]" " [Character] ")
              (if (= regex-wizard-tab-index 1) "[>Quantifier<]" " [Quantifier] ")
              (if (= regex-wizard-tab-index 2) "[>Assertion<]" " [Assertion] ")
              (if (= regex-wizard-tab-index 3) "[>Group<]" " [Group] "))))
        (set-panel-line regex-wizard-panel-name 1 regex-wizard-tmp-line)
        (set-panel-line-style regex-wizard-panel-name 1 0
          (str-length regex-wizard-tmp-line) theme-accent)
        ;; Rows 2-8: component items (unrolled)
        (set regex-wizard-tmp-labels
          (if (= regex-wizard-tab-index 0) regex-wizard-tab0-labels
            (if (= regex-wizard-tab-index 1) regex-wizard-tab1-labels
              (if (= regex-wizard-tab-index 2) regex-wizard-tab2-labels
                regex-wizard-tab3-labels))))
        (set regex-wizard-tmp-size (length regex-wizard-tmp-labels))
        ;; Item 0
        (if (> regex-wizard-tmp-size 0)
          (begin
            (set regex-wizard-tmp-line
              (if (= regex-wizard-cursor 0)
                (str-concat (list " > " (nth 0 regex-wizard-tmp-labels)))
                (str-concat (list "   " (nth 0 regex-wizard-tmp-labels)))))
            (set-panel-line regex-wizard-panel-name 2 regex-wizard-tmp-line)
            (if (= regex-wizard-cursor 0)
              (set-panel-line-style regex-wizard-panel-name 2 0
                (str-length regex-wizard-tmp-line) theme-highlight-bg)
              nil))
          nil)
        ;; Item 1
        (if (> regex-wizard-tmp-size 1)
          (begin
            (set regex-wizard-tmp-line
              (if (= regex-wizard-cursor 1)
                (str-concat (list " > " (nth 1 regex-wizard-tmp-labels)))
                (str-concat (list "   " (nth 1 regex-wizard-tmp-labels)))))
            (set-panel-line regex-wizard-panel-name 3 regex-wizard-tmp-line)
            (if (= regex-wizard-cursor 1)
              (set-panel-line-style regex-wizard-panel-name 3 0
                (str-length regex-wizard-tmp-line) theme-highlight-bg)
              nil))
          nil)
        ;; Item 2
        (if (> regex-wizard-tmp-size 2)
          (begin
            (set regex-wizard-tmp-line
              (if (= regex-wizard-cursor 2)
                (str-concat (list " > " (nth 2 regex-wizard-tmp-labels)))
                (str-concat (list "   " (nth 2 regex-wizard-tmp-labels)))))
            (set-panel-line regex-wizard-panel-name 4 regex-wizard-tmp-line)
            (if (= regex-wizard-cursor 2)
              (set-panel-line-style regex-wizard-panel-name 4 0
                (str-length regex-wizard-tmp-line) theme-highlight-bg)
              nil))
          nil)
        ;; Item 3
        (if (> regex-wizard-tmp-size 3)
          (begin
            (set regex-wizard-tmp-line
              (if (= regex-wizard-cursor 3)
                (str-concat (list " > " (nth 3 regex-wizard-tmp-labels)))
                (str-concat (list "   " (nth 3 regex-wizard-tmp-labels)))))
            (set-panel-line regex-wizard-panel-name 5 regex-wizard-tmp-line)
            (if (= regex-wizard-cursor 3)
              (set-panel-line-style regex-wizard-panel-name 5 0
                (str-length regex-wizard-tmp-line) theme-highlight-bg)
              nil))
          nil)
        ;; Item 4
        (if (> regex-wizard-tmp-size 4)
          (begin
            (set regex-wizard-tmp-line
              (if (= regex-wizard-cursor 4)
                (str-concat (list " > " (nth 4 regex-wizard-tmp-labels)))
                (str-concat (list "   " (nth 4 regex-wizard-tmp-labels)))))
            (set-panel-line regex-wizard-panel-name 6 regex-wizard-tmp-line)
            (if (= regex-wizard-cursor 4)
              (set-panel-line-style regex-wizard-panel-name 6 0
                (str-length regex-wizard-tmp-line) theme-highlight-bg)
              nil))
          nil)
        ;; Item 5
        (if (> regex-wizard-tmp-size 5)
          (begin
            (set regex-wizard-tmp-line
              (if (= regex-wizard-cursor 5)
                (str-concat (list " > " (nth 5 regex-wizard-tmp-labels)))
                (str-concat (list "   " (nth 5 regex-wizard-tmp-labels)))))
            (set-panel-line regex-wizard-panel-name 7 regex-wizard-tmp-line)
            (if (= regex-wizard-cursor 5)
              (set-panel-line-style regex-wizard-panel-name 7 0
                (str-length regex-wizard-tmp-line) theme-highlight-bg)
              nil))
          nil)
        ;; Item 6
        (if (> regex-wizard-tmp-size 6)
          (begin
            (set regex-wizard-tmp-line
              (if (= regex-wizard-cursor 6)
                (str-concat (list " > " (nth 6 regex-wizard-tmp-labels)))
                (str-concat (list "   " (nth 6 regex-wizard-tmp-labels)))))
            (set-panel-line regex-wizard-panel-name 8 regex-wizard-tmp-line)
            (if (= regex-wizard-cursor 6)
              (set-panel-line-style regex-wizard-panel-name 8 0
                (str-length regex-wizard-tmp-line) theme-highlight-bg)
              nil))
          nil)
        ;; Row 9: hint line
        (set regex-wizard-tmp-line
          (str-concat (list " " (to-string regex-wizard-match-count)
            " matches | [Enter] add [BS] remove [Tab] next [Esc] close")))
        (set-panel-line regex-wizard-panel-name 9 regex-wizard-tmp-line)
        (set-panel-line-style regex-wizard-panel-name 9 0
          (str-length regex-wizard-tmp-line) theme-muted)
        ;; Activate keymap
        (set-active-keymap "regex-wizard-input")))))

;; Escape: close wizard (already fully inline)
(define-command "regex-wizard-escape"
  (lambda ()
    (begin
      (clear-match-highlights)
      (set-panel-size regex-wizard-panel-name 0)
      (set regex-wizard-open nil)
      (set regex-wizard-query (str-concat (list)))
      (set regex-wizard-match-count 0)
      (set regex-wizard-tab-index 0)
      (set regex-wizard-cursor 0)
      (set-mode "normal")
      (set-active-keymap "normal-mode"))))

;; Backspace: remove last char from pattern, inline search + partial panel update
(define-command "regex-wizard-backspace"
  (lambda ()
    (if (= (str-length regex-wizard-query) 0)
      ;; Inline close
      (begin
        (clear-match-highlights)
        (set-panel-size regex-wizard-panel-name 0)
        (set regex-wizard-open nil)
        (set regex-wizard-query (str-concat (list)))
        (set regex-wizard-match-count 0)
        (set regex-wizard-tab-index 0)
        (set regex-wizard-cursor 0)
        (set-mode regex-wizard-previous-mode)
        (set-active-keymap regex-wizard-previous-keymap))
      (begin
        (set regex-wizard-query
          (str-substring regex-wizard-query 0
            (- (str-length regex-wizard-query) 1)))
        ;; Inline search
        (if (= (str-length regex-wizard-query) 0)
          (begin (clear-match-highlights) (set regex-wizard-match-count 0))
          (if (regex-valid? regex-wizard-query)
            (begin
              (clear-match-highlights)
              (set regex-wizard-match-count (regex-find-all regex-wizard-query theme-accent)))
            (begin (clear-match-highlights) (set regex-wizard-match-count 0))))
        ;; Inline full panel render
        (clear-panel-lines regex-wizard-panel-name)
        (clear-panel-line-styles regex-wizard-panel-name)
        ;; Row 0: pattern
        (set regex-wizard-tmp-line
          (str-concat (list " Pattern: " regex-wizard-query " | " (to-string regex-wizard-match-count) " matches")))
        (set-panel-line regex-wizard-panel-name 0 regex-wizard-tmp-line)
        (set-panel-line-style regex-wizard-panel-name 0 0
          (str-length regex-wizard-tmp-line) theme-prompt)
        ;; Row 1: tab bar
        (set regex-wizard-tmp-line
          (str-concat
            (list " "
              (if (= regex-wizard-tab-index 0) "[>Character<]" " [Character] ")
              (if (= regex-wizard-tab-index 1) "[>Quantifier<]" " [Quantifier] ")
              (if (= regex-wizard-tab-index 2) "[>Assertion<]" " [Assertion] ")
              (if (= regex-wizard-tab-index 3) "[>Group<]" " [Group] "))))
        (set-panel-line regex-wizard-panel-name 1 regex-wizard-tmp-line)
        (set-panel-line-style regex-wizard-panel-name 1 0
          (str-length regex-wizard-tmp-line) theme-accent)
        ;; Rows 2-8: component items
        (set regex-wizard-tmp-labels
          (if (= regex-wizard-tab-index 0) regex-wizard-tab0-labels
            (if (= regex-wizard-tab-index 1) regex-wizard-tab1-labels
              (if (= regex-wizard-tab-index 2) regex-wizard-tab2-labels
                regex-wizard-tab3-labels))))
        (set regex-wizard-tmp-size (length regex-wizard-tmp-labels))
        ;; Item 0
        (if (> regex-wizard-tmp-size 0)
          (begin
            (set regex-wizard-tmp-line
              (if (= regex-wizard-cursor 0)
                (str-concat (list " > " (nth 0 regex-wizard-tmp-labels)))
                (str-concat (list "   " (nth 0 regex-wizard-tmp-labels)))))
            (set-panel-line regex-wizard-panel-name 2 regex-wizard-tmp-line)
            (if (= regex-wizard-cursor 0)
              (set-panel-line-style regex-wizard-panel-name 2 0
                (str-length regex-wizard-tmp-line) theme-highlight-bg)
              nil))
          nil)
        ;; Item 1
        (if (> regex-wizard-tmp-size 1)
          (begin
            (set regex-wizard-tmp-line
              (if (= regex-wizard-cursor 1)
                (str-concat (list " > " (nth 1 regex-wizard-tmp-labels)))
                (str-concat (list "   " (nth 1 regex-wizard-tmp-labels)))))
            (set-panel-line regex-wizard-panel-name 3 regex-wizard-tmp-line)
            (if (= regex-wizard-cursor 1)
              (set-panel-line-style regex-wizard-panel-name 3 0
                (str-length regex-wizard-tmp-line) theme-highlight-bg)
              nil))
          nil)
        ;; Item 2
        (if (> regex-wizard-tmp-size 2)
          (begin
            (set regex-wizard-tmp-line
              (if (= regex-wizard-cursor 2)
                (str-concat (list " > " (nth 2 regex-wizard-tmp-labels)))
                (str-concat (list "   " (nth 2 regex-wizard-tmp-labels)))))
            (set-panel-line regex-wizard-panel-name 4 regex-wizard-tmp-line)
            (if (= regex-wizard-cursor 2)
              (set-panel-line-style regex-wizard-panel-name 4 0
                (str-length regex-wizard-tmp-line) theme-highlight-bg)
              nil))
          nil)
        ;; Item 3
        (if (> regex-wizard-tmp-size 3)
          (begin
            (set regex-wizard-tmp-line
              (if (= regex-wizard-cursor 3)
                (str-concat (list " > " (nth 3 regex-wizard-tmp-labels)))
                (str-concat (list "   " (nth 3 regex-wizard-tmp-labels)))))
            (set-panel-line regex-wizard-panel-name 5 regex-wizard-tmp-line)
            (if (= regex-wizard-cursor 3)
              (set-panel-line-style regex-wizard-panel-name 5 0
                (str-length regex-wizard-tmp-line) theme-highlight-bg)
              nil))
          nil)
        ;; Item 4
        (if (> regex-wizard-tmp-size 4)
          (begin
            (set regex-wizard-tmp-line
              (if (= regex-wizard-cursor 4)
                (str-concat (list " > " (nth 4 regex-wizard-tmp-labels)))
                (str-concat (list "   " (nth 4 regex-wizard-tmp-labels)))))
            (set-panel-line regex-wizard-panel-name 6 regex-wizard-tmp-line)
            (if (= regex-wizard-cursor 4)
              (set-panel-line-style regex-wizard-panel-name 6 0
                (str-length regex-wizard-tmp-line) theme-highlight-bg)
              nil))
          nil)
        ;; Item 5
        (if (> regex-wizard-tmp-size 5)
          (begin
            (set regex-wizard-tmp-line
              (if (= regex-wizard-cursor 5)
                (str-concat (list " > " (nth 5 regex-wizard-tmp-labels)))
                (str-concat (list "   " (nth 5 regex-wizard-tmp-labels)))))
            (set-panel-line regex-wizard-panel-name 7 regex-wizard-tmp-line)
            (if (= regex-wizard-cursor 5)
              (set-panel-line-style regex-wizard-panel-name 7 0
                (str-length regex-wizard-tmp-line) theme-highlight-bg)
              nil))
          nil)
        ;; Item 6
        (if (> regex-wizard-tmp-size 6)
          (begin
            (set regex-wizard-tmp-line
              (if (= regex-wizard-cursor 6)
                (str-concat (list " > " (nth 6 regex-wizard-tmp-labels)))
                (str-concat (list "   " (nth 6 regex-wizard-tmp-labels)))))
            (set-panel-line regex-wizard-panel-name 8 regex-wizard-tmp-line)
            (if (= regex-wizard-cursor 6)
              (set-panel-line-style regex-wizard-panel-name 8 0
                (str-length regex-wizard-tmp-line) theme-highlight-bg)
              nil))
          nil)
        ;; Row 9: hint line
        (set regex-wizard-tmp-line
          (str-concat (list " " (to-string regex-wizard-match-count)
            " matches | [Enter] add [BS] remove [Tab] next [Esc] close")))
        (set-panel-line regex-wizard-panel-name 9 regex-wizard-tmp-line)
        (set-panel-line-style regex-wizard-panel-name 9 0
          (str-length regex-wizard-tmp-line) theme-muted)))))

;; Tab: switch to next category tab, inline full panel render
(define-command "regex-wizard-next-tab"
  (lambda ()
    (begin
      (set regex-wizard-tab-index
        (if (= regex-wizard-tab-index 3) 0
          (+ regex-wizard-tab-index 1)))
      (set regex-wizard-cursor 0)
      ;; Inline full panel render
      (clear-panel-lines regex-wizard-panel-name)
      (clear-panel-line-styles regex-wizard-panel-name)
      ;; Row 0: pattern
      (set regex-wizard-tmp-line
        (str-concat (list " Pattern: " regex-wizard-query " | " (to-string regex-wizard-match-count) " matches")))
      (set-panel-line regex-wizard-panel-name 0 regex-wizard-tmp-line)
      (set-panel-line-style regex-wizard-panel-name 0 0
        (str-length regex-wizard-tmp-line) theme-prompt)
      ;; Row 1: tab bar
      (set regex-wizard-tmp-line
        (str-concat
          (list " "
            (if (= regex-wizard-tab-index 0) "[>Character<]" " [Character] ")
            (if (= regex-wizard-tab-index 1) "[>Quantifier<]" " [Quantifier] ")
            (if (= regex-wizard-tab-index 2) "[>Assertion<]" " [Assertion] ")
            (if (= regex-wizard-tab-index 3) "[>Group<]" " [Group] "))))
      (set-panel-line regex-wizard-panel-name 1 regex-wizard-tmp-line)
      (set-panel-line-style regex-wizard-panel-name 1 0
        (str-length regex-wizard-tmp-line) theme-accent)
      ;; Rows 2-8: component items
      (set regex-wizard-tmp-labels
        (if (= regex-wizard-tab-index 0) regex-wizard-tab0-labels
          (if (= regex-wizard-tab-index 1) regex-wizard-tab1-labels
            (if (= regex-wizard-tab-index 2) regex-wizard-tab2-labels
              regex-wizard-tab3-labels))))
      (set regex-wizard-tmp-size (length regex-wizard-tmp-labels))
      ;; Item 0
      (if (> regex-wizard-tmp-size 0)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 0)
              (str-concat (list " > " (nth 0 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 0 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 2 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 0)
            (set-panel-line-style regex-wizard-panel-name 2 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 1
      (if (> regex-wizard-tmp-size 1)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 1)
              (str-concat (list " > " (nth 1 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 1 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 3 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 1)
            (set-panel-line-style regex-wizard-panel-name 3 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 2
      (if (> regex-wizard-tmp-size 2)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 2)
              (str-concat (list " > " (nth 2 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 2 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 4 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 2)
            (set-panel-line-style regex-wizard-panel-name 4 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 3
      (if (> regex-wizard-tmp-size 3)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 3)
              (str-concat (list " > " (nth 3 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 3 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 5 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 3)
            (set-panel-line-style regex-wizard-panel-name 5 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 4
      (if (> regex-wizard-tmp-size 4)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 4)
              (str-concat (list " > " (nth 4 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 4 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 6 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 4)
            (set-panel-line-style regex-wizard-panel-name 6 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 5
      (if (> regex-wizard-tmp-size 5)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 5)
              (str-concat (list " > " (nth 5 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 5 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 7 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 5)
            (set-panel-line-style regex-wizard-panel-name 7 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 6
      (if (> regex-wizard-tmp-size 6)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 6)
              (str-concat (list " > " (nth 6 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 6 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 8 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 6)
            (set-panel-line-style regex-wizard-panel-name 8 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Row 9: hint line
      (set regex-wizard-tmp-line
        (str-concat (list " " (to-string regex-wizard-match-count)
          " matches | [Enter] add [BS] remove [Tab] next [Esc] close")))
      (set-panel-line regex-wizard-panel-name 9 regex-wizard-tmp-line)
      (set-panel-line-style regex-wizard-panel-name 9 0
        (str-length regex-wizard-tmp-line) theme-muted))))

;; Cursor down: move to next component in active tab
(define-command "regex-wizard-cursor-down"
  (lambda ()
    (begin
      ;; Inline tab size
      (set regex-wizard-tmp-size
        (length
          (if (= regex-wizard-tab-index 0) regex-wizard-tab0-labels
            (if (= regex-wizard-tab-index 1) regex-wizard-tab1-labels
              (if (= regex-wizard-tab-index 2) regex-wizard-tab2-labels
                regex-wizard-tab3-labels)))))
      (set regex-wizard-cursor
        (if (>= (+ regex-wizard-cursor 1) regex-wizard-tmp-size)
          0
          (+ regex-wizard-cursor 1)))
      ;; Inline full panel render
      (clear-panel-lines regex-wizard-panel-name)
      (clear-panel-line-styles regex-wizard-panel-name)
      ;; Row 0: pattern
      (set regex-wizard-tmp-line
        (str-concat (list " Pattern: " regex-wizard-query " | " (to-string regex-wizard-match-count) " matches")))
      (set-panel-line regex-wizard-panel-name 0 regex-wizard-tmp-line)
      (set-panel-line-style regex-wizard-panel-name 0 0
        (str-length regex-wizard-tmp-line) theme-prompt)
      ;; Row 1: tab bar
      (set regex-wizard-tmp-line
        (str-concat
          (list " "
            (if (= regex-wizard-tab-index 0) "[>Character<]" " [Character] ")
            (if (= regex-wizard-tab-index 1) "[>Quantifier<]" " [Quantifier] ")
            (if (= regex-wizard-tab-index 2) "[>Assertion<]" " [Assertion] ")
            (if (= regex-wizard-tab-index 3) "[>Group<]" " [Group] "))))
      (set-panel-line regex-wizard-panel-name 1 regex-wizard-tmp-line)
      (set-panel-line-style regex-wizard-panel-name 1 0
        (str-length regex-wizard-tmp-line) theme-accent)
      ;; Rows 2-8: component items
      (set regex-wizard-tmp-labels
        (if (= regex-wizard-tab-index 0) regex-wizard-tab0-labels
          (if (= regex-wizard-tab-index 1) regex-wizard-tab1-labels
            (if (= regex-wizard-tab-index 2) regex-wizard-tab2-labels
              regex-wizard-tab3-labels))))
      ;; Item 0
      (if (> regex-wizard-tmp-size 0)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 0)
              (str-concat (list " > " (nth 0 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 0 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 2 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 0)
            (set-panel-line-style regex-wizard-panel-name 2 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 1
      (if (> regex-wizard-tmp-size 1)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 1)
              (str-concat (list " > " (nth 1 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 1 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 3 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 1)
            (set-panel-line-style regex-wizard-panel-name 3 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 2
      (if (> regex-wizard-tmp-size 2)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 2)
              (str-concat (list " > " (nth 2 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 2 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 4 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 2)
            (set-panel-line-style regex-wizard-panel-name 4 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 3
      (if (> regex-wizard-tmp-size 3)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 3)
              (str-concat (list " > " (nth 3 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 3 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 5 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 3)
            (set-panel-line-style regex-wizard-panel-name 5 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 4
      (if (> regex-wizard-tmp-size 4)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 4)
              (str-concat (list " > " (nth 4 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 4 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 6 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 4)
            (set-panel-line-style regex-wizard-panel-name 6 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 5
      (if (> regex-wizard-tmp-size 5)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 5)
              (str-concat (list " > " (nth 5 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 5 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 7 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 5)
            (set-panel-line-style regex-wizard-panel-name 7 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 6
      (if (> regex-wizard-tmp-size 6)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 6)
              (str-concat (list " > " (nth 6 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 6 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 8 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 6)
            (set-panel-line-style regex-wizard-panel-name 8 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Row 9: hint line
      (set regex-wizard-tmp-line
        (str-concat (list " " (to-string regex-wizard-match-count)
          " matches | [Enter] add [BS] remove [Tab] next [Esc] close")))
      (set-panel-line regex-wizard-panel-name 9 regex-wizard-tmp-line)
      (set-panel-line-style regex-wizard-panel-name 9 0
        (str-length regex-wizard-tmp-line) theme-muted))))

;; Cursor up: move to previous component in active tab
(define-command "regex-wizard-cursor-up"
  (lambda ()
    (begin
      ;; Inline tab size
      (set regex-wizard-tmp-size
        (length
          (if (= regex-wizard-tab-index 0) regex-wizard-tab0-labels
            (if (= regex-wizard-tab-index 1) regex-wizard-tab1-labels
              (if (= regex-wizard-tab-index 2) regex-wizard-tab2-labels
                regex-wizard-tab3-labels)))))
      (set regex-wizard-cursor
        (if (= regex-wizard-cursor 0)
          (- regex-wizard-tmp-size 1)
          (- regex-wizard-cursor 1)))
      ;; Inline full panel render
      (clear-panel-lines regex-wizard-panel-name)
      (clear-panel-line-styles regex-wizard-panel-name)
      ;; Row 0: pattern
      (set regex-wizard-tmp-line
        (str-concat (list " Pattern: " regex-wizard-query " | " (to-string regex-wizard-match-count) " matches")))
      (set-panel-line regex-wizard-panel-name 0 regex-wizard-tmp-line)
      (set-panel-line-style regex-wizard-panel-name 0 0
        (str-length regex-wizard-tmp-line) theme-prompt)
      ;; Row 1: tab bar
      (set regex-wizard-tmp-line
        (str-concat
          (list " "
            (if (= regex-wizard-tab-index 0) "[>Character<]" " [Character] ")
            (if (= regex-wizard-tab-index 1) "[>Quantifier<]" " [Quantifier] ")
            (if (= regex-wizard-tab-index 2) "[>Assertion<]" " [Assertion] ")
            (if (= regex-wizard-tab-index 3) "[>Group<]" " [Group] "))))
      (set-panel-line regex-wizard-panel-name 1 regex-wizard-tmp-line)
      (set-panel-line-style regex-wizard-panel-name 1 0
        (str-length regex-wizard-tmp-line) theme-accent)
      ;; Rows 2-8: component items
      (set regex-wizard-tmp-labels
        (if (= regex-wizard-tab-index 0) regex-wizard-tab0-labels
          (if (= regex-wizard-tab-index 1) regex-wizard-tab1-labels
            (if (= regex-wizard-tab-index 2) regex-wizard-tab2-labels
              regex-wizard-tab3-labels))))
      ;; Item 0
      (if (> regex-wizard-tmp-size 0)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 0)
              (str-concat (list " > " (nth 0 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 0 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 2 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 0)
            (set-panel-line-style regex-wizard-panel-name 2 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 1
      (if (> regex-wizard-tmp-size 1)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 1)
              (str-concat (list " > " (nth 1 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 1 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 3 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 1)
            (set-panel-line-style regex-wizard-panel-name 3 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 2
      (if (> regex-wizard-tmp-size 2)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 2)
              (str-concat (list " > " (nth 2 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 2 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 4 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 2)
            (set-panel-line-style regex-wizard-panel-name 4 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 3
      (if (> regex-wizard-tmp-size 3)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 3)
              (str-concat (list " > " (nth 3 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 3 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 5 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 3)
            (set-panel-line-style regex-wizard-panel-name 5 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 4
      (if (> regex-wizard-tmp-size 4)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 4)
              (str-concat (list " > " (nth 4 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 4 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 6 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 4)
            (set-panel-line-style regex-wizard-panel-name 6 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 5
      (if (> regex-wizard-tmp-size 5)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 5)
              (str-concat (list " > " (nth 5 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 5 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 7 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 5)
            (set-panel-line-style regex-wizard-panel-name 7 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 6
      (if (> regex-wizard-tmp-size 6)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 6)
              (str-concat (list " > " (nth 6 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 6 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 8 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 6)
            (set-panel-line-style regex-wizard-panel-name 8 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Row 9: hint line
      (set regex-wizard-tmp-line
        (str-concat (list " " (to-string regex-wizard-match-count)
          " matches | [Enter] add [BS] remove [Tab] next [Esc] close")))
      (set-panel-line regex-wizard-panel-name 9 regex-wizard-tmp-line)
      (set-panel-line-style regex-wizard-panel-name 9 0
        (str-length regex-wizard-tmp-line) theme-muted))))

;; Enter: add selected component pattern to the query
(define-command "regex-wizard-select"
  (lambda ()
    (begin
      ;; Inline current patterns lookup
      (set regex-wizard-query
        (str-concat (list regex-wizard-query
          (nth regex-wizard-cursor
            (if (= regex-wizard-tab-index 0) regex-wizard-tab0-patterns
              (if (= regex-wizard-tab-index 1) regex-wizard-tab1-patterns
                (if (= regex-wizard-tab-index 2) regex-wizard-tab2-patterns
                  regex-wizard-tab3-patterns)))))))
      ;; Inline search
      (if (= (str-length regex-wizard-query) 0)
        (begin (clear-match-highlights) (set regex-wizard-match-count 0))
        (if (regex-valid? regex-wizard-query)
          (begin
            (clear-match-highlights)
            (set regex-wizard-match-count (regex-find-all regex-wizard-query theme-accent)))
          (begin (clear-match-highlights) (set regex-wizard-match-count 0))))
      ;; Force full redraw by toggling panel size (resets ratatui cell cache)
      (set-panel-size regex-wizard-panel-name 0)
      (set-panel-size regex-wizard-panel-name regex-wizard-panel-height)
      ;; Inline full panel render
      (clear-panel-lines regex-wizard-panel-name)
      (clear-panel-line-styles regex-wizard-panel-name)
      ;; Row 0: pattern
      (set regex-wizard-tmp-line
        (str-concat (list " Pattern: " regex-wizard-query " | " (to-string regex-wizard-match-count) " matches")))
      (set-panel-line regex-wizard-panel-name 0 regex-wizard-tmp-line)
      (set-panel-line-style regex-wizard-panel-name 0 0
        (str-length regex-wizard-tmp-line) theme-prompt)
      ;; Row 1: tab bar
      (set regex-wizard-tmp-line
        (str-concat
          (list " "
            (if (= regex-wizard-tab-index 0) "[>Character<]" " [Character] ")
            (if (= regex-wizard-tab-index 1) "[>Quantifier<]" " [Quantifier] ")
            (if (= regex-wizard-tab-index 2) "[>Assertion<]" " [Assertion] ")
            (if (= regex-wizard-tab-index 3) "[>Group<]" " [Group] "))))
      (set-panel-line regex-wizard-panel-name 1 regex-wizard-tmp-line)
      (set-panel-line-style regex-wizard-panel-name 1 0
        (str-length regex-wizard-tmp-line) theme-accent)
      ;; Rows 2-8: component items
      (set regex-wizard-tmp-labels
        (if (= regex-wizard-tab-index 0) regex-wizard-tab0-labels
          (if (= regex-wizard-tab-index 1) regex-wizard-tab1-labels
            (if (= regex-wizard-tab-index 2) regex-wizard-tab2-labels
              regex-wizard-tab3-labels))))
      (set regex-wizard-tmp-size (length regex-wizard-tmp-labels))
      ;; Item 0
      (if (> regex-wizard-tmp-size 0)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 0)
              (str-concat (list " > " (nth 0 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 0 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 2 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 0)
            (set-panel-line-style regex-wizard-panel-name 2 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 1
      (if (> regex-wizard-tmp-size 1)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 1)
              (str-concat (list " > " (nth 1 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 1 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 3 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 1)
            (set-panel-line-style regex-wizard-panel-name 3 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 2
      (if (> regex-wizard-tmp-size 2)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 2)
              (str-concat (list " > " (nth 2 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 2 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 4 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 2)
            (set-panel-line-style regex-wizard-panel-name 4 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 3
      (if (> regex-wizard-tmp-size 3)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 3)
              (str-concat (list " > " (nth 3 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 3 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 5 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 3)
            (set-panel-line-style regex-wizard-panel-name 5 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 4
      (if (> regex-wizard-tmp-size 4)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 4)
              (str-concat (list " > " (nth 4 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 4 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 6 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 4)
            (set-panel-line-style regex-wizard-panel-name 6 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 5
      (if (> regex-wizard-tmp-size 5)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 5)
              (str-concat (list " > " (nth 5 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 5 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 7 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 5)
            (set-panel-line-style regex-wizard-panel-name 7 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 6
      (if (> regex-wizard-tmp-size 6)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 6)
              (str-concat (list " > " (nth 6 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 6 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 8 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 6)
            (set-panel-line-style regex-wizard-panel-name 8 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Row 9: hint line
      (set regex-wizard-tmp-line
        (str-concat (list " " (to-string regex-wizard-match-count)
          " matches | [Enter] add [BS] remove [Tab] next [Esc] close")))
      (set-panel-line regex-wizard-panel-name 9 regex-wizard-tmp-line)
      (set-panel-line-style regex-wizard-panel-name 9 0
        (str-length regex-wizard-tmp-line) theme-muted))))

;; Direct tab selection: keys 1-4
(define-command "regex-wizard-tab-1"
  (lambda ()
    (begin
      (set regex-wizard-tab-index 0)
      (set regex-wizard-cursor 0)
      ;; Inline full panel render
      (clear-panel-lines regex-wizard-panel-name)
      (clear-panel-line-styles regex-wizard-panel-name)
      (set regex-wizard-tmp-line
        (str-concat (list " Pattern: " regex-wizard-query " | " (to-string regex-wizard-match-count) " matches")))
      (set-panel-line regex-wizard-panel-name 0 regex-wizard-tmp-line)
      (set-panel-line-style regex-wizard-panel-name 0 0
        (str-length regex-wizard-tmp-line) theme-prompt)
      (set regex-wizard-tmp-line
        (str-concat
          (list " " "[>Character<]" " [Quantifier] " " [Assertion] " " [Group] ")))
      (set-panel-line regex-wizard-panel-name 1 regex-wizard-tmp-line)
      (set-panel-line-style regex-wizard-panel-name 1 0
        (str-length regex-wizard-tmp-line) theme-accent)
      (set regex-wizard-tmp-labels regex-wizard-tab0-labels)
      (set regex-wizard-tmp-size (length regex-wizard-tmp-labels))
      ;; Item 0
      (if (> regex-wizard-tmp-size 0)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 0)
              (str-concat (list " > " (nth 0 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 0 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 2 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 0)
            (set-panel-line-style regex-wizard-panel-name 2 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 1
      (if (> regex-wizard-tmp-size 1)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 1)
              (str-concat (list " > " (nth 1 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 1 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 3 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 1)
            (set-panel-line-style regex-wizard-panel-name 3 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 2
      (if (> regex-wizard-tmp-size 2)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 2)
              (str-concat (list " > " (nth 2 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 2 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 4 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 2)
            (set-panel-line-style regex-wizard-panel-name 4 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 3
      (if (> regex-wizard-tmp-size 3)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 3)
              (str-concat (list " > " (nth 3 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 3 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 5 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 3)
            (set-panel-line-style regex-wizard-panel-name 5 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 4
      (if (> regex-wizard-tmp-size 4)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 4)
              (str-concat (list " > " (nth 4 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 4 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 6 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 4)
            (set-panel-line-style regex-wizard-panel-name 6 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 5
      (if (> regex-wizard-tmp-size 5)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 5)
              (str-concat (list " > " (nth 5 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 5 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 7 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 5)
            (set-panel-line-style regex-wizard-panel-name 7 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 6
      (if (> regex-wizard-tmp-size 6)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 6)
              (str-concat (list " > " (nth 6 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 6 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 8 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 6)
            (set-panel-line-style regex-wizard-panel-name 8 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Row 9: hint line
      (set regex-wizard-tmp-line
        (str-concat (list " " (to-string regex-wizard-match-count)
          " matches | [Enter] add [BS] remove [Tab] next [Esc] close")))
      (set-panel-line regex-wizard-panel-name 9 regex-wizard-tmp-line)
      (set-panel-line-style regex-wizard-panel-name 9 0
        (str-length regex-wizard-tmp-line) theme-muted))))

(define-command "regex-wizard-tab-2"
  (lambda ()
    (begin
      (set regex-wizard-tab-index 1)
      (set regex-wizard-cursor 0)
      ;; Inline full panel render
      (clear-panel-lines regex-wizard-panel-name)
      (clear-panel-line-styles regex-wizard-panel-name)
      (set regex-wizard-tmp-line
        (str-concat (list " Pattern: " regex-wizard-query " | " (to-string regex-wizard-match-count) " matches")))
      (set-panel-line regex-wizard-panel-name 0 regex-wizard-tmp-line)
      (set-panel-line-style regex-wizard-panel-name 0 0
        (str-length regex-wizard-tmp-line) theme-prompt)
      (set regex-wizard-tmp-line
        (str-concat
          (list " " " [Character] " "[>Quantifier<]" " [Assertion] " " [Group] ")))
      (set-panel-line regex-wizard-panel-name 1 regex-wizard-tmp-line)
      (set-panel-line-style regex-wizard-panel-name 1 0
        (str-length regex-wizard-tmp-line) theme-accent)
      (set regex-wizard-tmp-labels regex-wizard-tab1-labels)
      (set regex-wizard-tmp-size (length regex-wizard-tmp-labels))
      ;; Item 0
      (if (> regex-wizard-tmp-size 0)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 0)
              (str-concat (list " > " (nth 0 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 0 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 2 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 0)
            (set-panel-line-style regex-wizard-panel-name 2 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 1
      (if (> regex-wizard-tmp-size 1)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 1)
              (str-concat (list " > " (nth 1 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 1 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 3 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 1)
            (set-panel-line-style regex-wizard-panel-name 3 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 2
      (if (> regex-wizard-tmp-size 2)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 2)
              (str-concat (list " > " (nth 2 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 2 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 4 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 2)
            (set-panel-line-style regex-wizard-panel-name 4 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Row 9: hint line
      (set regex-wizard-tmp-line
        (str-concat (list " " (to-string regex-wizard-match-count)
          " matches | [Enter] add [BS] remove [Tab] next [Esc] close")))
      (set-panel-line regex-wizard-panel-name 9 regex-wizard-tmp-line)
      (set-panel-line-style regex-wizard-panel-name 9 0
        (str-length regex-wizard-tmp-line) theme-muted))))

(define-command "regex-wizard-tab-3"
  (lambda ()
    (begin
      (set regex-wizard-tab-index 2)
      (set regex-wizard-cursor 0)
      ;; Inline full panel render
      (clear-panel-lines regex-wizard-panel-name)
      (clear-panel-line-styles regex-wizard-panel-name)
      (set regex-wizard-tmp-line
        (str-concat (list " Pattern: " regex-wizard-query " | " (to-string regex-wizard-match-count) " matches")))
      (set-panel-line regex-wizard-panel-name 0 regex-wizard-tmp-line)
      (set-panel-line-style regex-wizard-panel-name 0 0
        (str-length regex-wizard-tmp-line) theme-prompt)
      (set regex-wizard-tmp-line
        (str-concat
          (list " " " [Character] " " [Quantifier] " "[>Assertion<]" " [Group] ")))
      (set-panel-line regex-wizard-panel-name 1 regex-wizard-tmp-line)
      (set-panel-line-style regex-wizard-panel-name 1 0
        (str-length regex-wizard-tmp-line) theme-accent)
      (set regex-wizard-tmp-labels regex-wizard-tab2-labels)
      (set regex-wizard-tmp-size (length regex-wizard-tmp-labels))
      ;; Item 0
      (if (> regex-wizard-tmp-size 0)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 0)
              (str-concat (list " > " (nth 0 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 0 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 2 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 0)
            (set-panel-line-style regex-wizard-panel-name 2 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 1
      (if (> regex-wizard-tmp-size 1)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 1)
              (str-concat (list " > " (nth 1 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 1 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 3 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 1)
            (set-panel-line-style regex-wizard-panel-name 3 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 2
      (if (> regex-wizard-tmp-size 2)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 2)
              (str-concat (list " > " (nth 2 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 2 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 4 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 2)
            (set-panel-line-style regex-wizard-panel-name 4 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 3
      (if (> regex-wizard-tmp-size 3)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 3)
              (str-concat (list " > " (nth 3 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 3 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 5 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 3)
            (set-panel-line-style regex-wizard-panel-name 5 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Row 9: hint line
      (set regex-wizard-tmp-line
        (str-concat (list " " (to-string regex-wizard-match-count)
          " matches | [Enter] add [BS] remove [Tab] next [Esc] close")))
      (set-panel-line regex-wizard-panel-name 9 regex-wizard-tmp-line)
      (set-panel-line-style regex-wizard-panel-name 9 0
        (str-length regex-wizard-tmp-line) theme-muted))))

(define-command "regex-wizard-tab-4"
  (lambda ()
    (begin
      (set regex-wizard-tab-index 3)
      (set regex-wizard-cursor 0)
      ;; Inline full panel render
      (clear-panel-lines regex-wizard-panel-name)
      (clear-panel-line-styles regex-wizard-panel-name)
      (set regex-wizard-tmp-line
        (str-concat (list " Pattern: " regex-wizard-query " | " (to-string regex-wizard-match-count) " matches")))
      (set-panel-line regex-wizard-panel-name 0 regex-wizard-tmp-line)
      (set-panel-line-style regex-wizard-panel-name 0 0
        (str-length regex-wizard-tmp-line) theme-prompt)
      (set regex-wizard-tmp-line
        (str-concat
          (list " " " [Character] " " [Quantifier] " " [Assertion] " "[>Group<]")))
      (set-panel-line regex-wizard-panel-name 1 regex-wizard-tmp-line)
      (set-panel-line-style regex-wizard-panel-name 1 0
        (str-length regex-wizard-tmp-line) theme-accent)
      (set regex-wizard-tmp-labels regex-wizard-tab3-labels)
      (set regex-wizard-tmp-size (length regex-wizard-tmp-labels))
      ;; Item 0
      (if (> regex-wizard-tmp-size 0)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 0)
              (str-concat (list " > " (nth 0 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 0 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 2 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 0)
            (set-panel-line-style regex-wizard-panel-name 2 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 1
      (if (> regex-wizard-tmp-size 1)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 1)
              (str-concat (list " > " (nth 1 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 1 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 3 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 1)
            (set-panel-line-style regex-wizard-panel-name 3 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Item 2
      (if (> regex-wizard-tmp-size 2)
        (begin
          (set regex-wizard-tmp-line
            (if (= regex-wizard-cursor 2)
              (str-concat (list " > " (nth 2 regex-wizard-tmp-labels)))
              (str-concat (list "   " (nth 2 regex-wizard-tmp-labels)))))
          (set-panel-line regex-wizard-panel-name 4 regex-wizard-tmp-line)
          (if (= regex-wizard-cursor 2)
            (set-panel-line-style regex-wizard-panel-name 4 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg)
            nil))
        nil)
      ;; Row 9: hint line
      (set regex-wizard-tmp-line
        (str-concat (list " " (to-string regex-wizard-match-count)
          " matches | [Enter] add [BS] remove [Tab] next [Esc] close")))
      (set-panel-line regex-wizard-panel-name 9 regex-wizard-tmp-line)
      (set-panel-line-style regex-wizard-panel-name 9 0
        (str-length regex-wizard-tmp-line) theme-muted))))

;; ---------------------------------------------------------------------------
;; Keymap -- navigation bindings for component picker
;; ---------------------------------------------------------------------------

(make-keymap "regex-wizard-input")
(define-key "regex-wizard-input" "Escape" "regex-wizard-escape")
(define-key "regex-wizard-input" "Backspace" "regex-wizard-backspace")
(define-key "regex-wizard-input" "Tab" "regex-wizard-next-tab")
(define-key "regex-wizard-input" "Enter" "regex-wizard-select")
(define-key "regex-wizard-input" "Down" "regex-wizard-cursor-down")
(define-key "regex-wizard-input" "Up" "regex-wizard-cursor-up")
(define-key "regex-wizard-input" "Char:j" "regex-wizard-cursor-down")
(define-key "regex-wizard-input" "Char:k" "regex-wizard-cursor-up")
(define-key "regex-wizard-input" "Char:1" "regex-wizard-tab-1")
(define-key "regex-wizard-input" "Char:2" "regex-wizard-tab-2")
(define-key "regex-wizard-input" "Char:3" "regex-wizard-tab-3")
(define-key "regex-wizard-input" "Char:4" "regex-wizard-tab-4")

;; ---------------------------------------------------------------------------
;; Alias: :regex opens the wizard (fully inlined)
;; ---------------------------------------------------------------------------

(define-command "regex"
  (lambda ()
    (if regex-wizard-open
      ;; Inline close
      (begin
        (clear-match-highlights)
        (set-panel-size regex-wizard-panel-name 0)
        (set regex-wizard-open nil)
        (set regex-wizard-query (str-concat (list)))
        (set regex-wizard-match-count 0)
        (set regex-wizard-tab-index 0)
        (set regex-wizard-cursor 0)
        (set-mode "normal")
        (set-active-keymap "normal-mode"))
      ;; Inline open with full panel render
      (begin
        (set regex-wizard-query (str-concat (list)))
        (set regex-wizard-match-count 0)
        (set regex-wizard-tab-index 0)
        (set regex-wizard-cursor 0)
        (set regex-wizard-open 1)
        (set-panel-size regex-wizard-panel-name regex-wizard-panel-height)
        ;; Inline full panel render
        (clear-panel-lines regex-wizard-panel-name)
        (clear-panel-line-styles regex-wizard-panel-name)
        ;; Row 0: pattern
        (set regex-wizard-tmp-line
          (str-concat (list " Pattern: " regex-wizard-query " | " (to-string regex-wizard-match-count) " matches")))
        (set-panel-line regex-wizard-panel-name 0 regex-wizard-tmp-line)
        (set-panel-line-style regex-wizard-panel-name 0 0
          (str-length regex-wizard-tmp-line) theme-prompt)
        ;; Row 1: tab bar (tab 0 active on open)
        (set regex-wizard-tmp-line
          (str-concat
            (list " " "[>Character<]" " [Quantifier] " " [Assertion] " " [Group] ")))
        (set-panel-line regex-wizard-panel-name 1 regex-wizard-tmp-line)
        (set-panel-line-style regex-wizard-panel-name 1 0
          (str-length regex-wizard-tmp-line) theme-accent)
        ;; Rows 2-8: tab0 items (cursor=0 on open)
        (set regex-wizard-tmp-labels regex-wizard-tab0-labels)
        (set regex-wizard-tmp-size (length regex-wizard-tmp-labels))
        ;; Item 0 (cursor)
        (if (> regex-wizard-tmp-size 0)
          (begin
            (set regex-wizard-tmp-line
              (str-concat (list " > " (nth 0 regex-wizard-tmp-labels))))
            (set-panel-line regex-wizard-panel-name 2 regex-wizard-tmp-line)
            (set-panel-line-style regex-wizard-panel-name 2 0
              (str-length regex-wizard-tmp-line) theme-highlight-bg))
          nil)
        ;; Item 1
        (if (> regex-wizard-tmp-size 1)
          (begin
            (set regex-wizard-tmp-line
              (str-concat (list "   " (nth 1 regex-wizard-tmp-labels))))
            (set-panel-line regex-wizard-panel-name 3 regex-wizard-tmp-line))
          nil)
        ;; Item 2
        (if (> regex-wizard-tmp-size 2)
          (begin
            (set regex-wizard-tmp-line
              (str-concat (list "   " (nth 2 regex-wizard-tmp-labels))))
            (set-panel-line regex-wizard-panel-name 4 regex-wizard-tmp-line))
          nil)
        ;; Item 3
        (if (> regex-wizard-tmp-size 3)
          (begin
            (set regex-wizard-tmp-line
              (str-concat (list "   " (nth 3 regex-wizard-tmp-labels))))
            (set-panel-line regex-wizard-panel-name 5 regex-wizard-tmp-line))
          nil)
        ;; Item 4
        (if (> regex-wizard-tmp-size 4)
          (begin
            (set regex-wizard-tmp-line
              (str-concat (list "   " (nth 4 regex-wizard-tmp-labels))))
            (set-panel-line regex-wizard-panel-name 6 regex-wizard-tmp-line))
          nil)
        ;; Item 5
        (if (> regex-wizard-tmp-size 5)
          (begin
            (set regex-wizard-tmp-line
              (str-concat (list "   " (nth 5 regex-wizard-tmp-labels))))
            (set-panel-line regex-wizard-panel-name 7 regex-wizard-tmp-line))
          nil)
        ;; Item 6
        (if (> regex-wizard-tmp-size 6)
          (begin
            (set regex-wizard-tmp-line
              (str-concat (list "   " (nth 6 regex-wizard-tmp-labels))))
            (set-panel-line regex-wizard-panel-name 8 regex-wizard-tmp-line))
          nil)
        ;; Row 9: hint line
        (set regex-wizard-tmp-line
          (str-concat (list " " (to-string regex-wizard-match-count)
            " matches | [Enter] add [BS] remove [Tab] next [Esc] close")))
        (set-panel-line regex-wizard-panel-name 9 regex-wizard-tmp-line)
        (set-panel-line-style regex-wizard-panel-name 9 0
          (str-length regex-wizard-tmp-line) theme-muted)
        (set-active-keymap "regex-wizard-input")))))
