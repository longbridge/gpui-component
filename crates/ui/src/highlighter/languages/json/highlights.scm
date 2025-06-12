(comment) @comment

(string) @string
(escape_sequence) @string.escape

(pair key: (string) @property)

(number) @number

[
  (true)
  (false)
] @boolean

(null) @constant.builtin

[
  ","
  ":"
  "{"
  "}"
  "["
  "]"
] @punctuation
