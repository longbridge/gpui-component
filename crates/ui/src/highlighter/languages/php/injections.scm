; PHP injection rules
; Based on tree-sitter-php injections.scm with added HTML support for text nodes

((comment) @injection.content
  (#set! injection.language "phpdoc"))

(heredoc
  (heredoc_body) @injection.content
  (heredoc_end) @injection.language)

(nowdoc
  (nowdoc_body) @injection.content
  (heredoc_end) @injection.language)

; HTML in text nodes (content outside <?php ?> tags)
((text) @injection.content
  (#set! injection.language "html"))
