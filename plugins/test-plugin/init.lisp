;;; name: test-plugin
;;; version: 0.1.0
;;; description: A test plugin that registers a hello command

(define-command "hello" (lambda () (message "Hello from test-plugin!")))
