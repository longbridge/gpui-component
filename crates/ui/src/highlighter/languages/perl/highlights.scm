; Shebang line
((source_file . (comment) @preproc)
  (#lua-match? @preproc "^#!/"))

; Keywords — control flow (mapped from @conditional / @repeat / @exception)
[ "if" "elsif" "unless" "else" ] @keyword
[ "while" "until" "for" "foreach" ] @keyword
("continue" @keyword (block))
[ "try" "catch" "finally" ] @keyword
(yadayada) @keyword

; Keywords — modules / namespaces (mapped from @include)
[ "use" "no" "require" "package" "class" "role" ] @keyword

; Keywords — declarations and control
"return" @keyword
[ "sub" "method" "async" "extended" ] @keyword
[
  "defer"
  "do" "eval"
  "my" "our" "local" "dynamically" "state" "field"
  "last" "next" "redo" "goto"
  "undef" "await"
] @keyword

; Phaser blocks (BEGIN, END, INIT …)
(phaser_statement phase: _ @keyword)
(class_phaser_statement phase: _ @keyword)

; Operators
(_ operator: _ @operator)
"\\" @operator
[
  "or" "xor" "and"
  "eq" "ne" "cmp" "lt" "le" "ge" "gt"
  "isa"
] @operator

; Special markers
(eof_marker) @preproc
(data_section) @comment

; POD documentation
(pod) @comment

; Numbers
[
  (number)
  (version)
] @number

; Strings
[
  (string_literal)
  (interpolated_string_literal)
  (quoted_word_list)
  (command_string)
  (heredoc_content)
  (replacement)
  (transliteration_content)
] @string

; Heredoc tokens styled as labels
[
  (heredoc_token)
  (command_heredoc_token)
  (heredoc_end)
] @label

; Escape sequences
[(escape_sequence) (escaped_delimiter)] @string.escape

; Regex modifiers
(_ modifiers: _ @operator)

; Regular expressions
[
  (quoted_regexp)
  (match_regexp)
  (regexp_content)
] @string.regex

; Autoquoted barewords (e.g. hash keys)
(autoquoted_bareword) @string.special

; Types / packages
(use_statement (package) @type)
(package_statement (package) @type)
(class_statement (package) @type)
(require_expression (bareword) @type)

; Functions / methods
(subroutine_declaration_statement name: (bareword) @function)
(method_declaration_statement name: (bareword) @function)
(attribute_name) @attribute
(attribute_value) @string

; Labels
(label) @label
(statement_label label: _ @label)

(relational_expression operator: "isa" right: (bareword) @type)

(function) @function
(function_call_expression (function) @function)
(method_call_expression (method) @function)
(method_call_expression invocant: (bareword) @type)

; Built-in list operators
[ "map" "grep" "sort" ] @function
(func0op_call_expression function: _ @function)
(func1op_call_expression function: _ @function)

([(function)(expression_statement (bareword))] @function
 (#match? @function
   "^(accept|atan2|bind|binmode|bless|crypt|chmod|chown|connect|die|dbmopen|exec|fcntl|flock|getpriority|getprotobynumber|gethostbyaddr|getnetbyaddr|getservbyname|getservbyport|getsockopt|glob|index|ioctl|join|kill|link|listen|mkdir|msgctl|msgget|msgrcv|msgsend|opendir|print|printf|push|pack|pipe|return|rename|rindex|read|recv|reverse|say|select|seek|semctl|semget|semop|send|setpgrp|setpriority|seekdir|setsockopt|shmctl|shmread|shmwrite|shutdown|socket|socketpair|split|sprintf|splice|substr|system|symlink|syscall|sysopen|sysseek|sysread|syswrite|tie|truncate|unlink|unpack|utime|unshift|vec|warn|waitpid|formline|open|sort)$"
))

; Parse errors
(ERROR) @error

; Built-in variables ($_, $!, $@, @ARGV, …)
(
  [(varname) (filehandle)] @variable.special
  (#match? @variable.special "^((ENV|ARGV|INC|ARGVOUT|SIG|STDIN|STDOUT|STDERR)|[_ab]|\\W|\\d+|\\^.*)$")
)

; Variable sigils
[(array) (arraylen)] @variable
(glob) @variable.special
(scalar) @variable
(hash) @variable

; Dereferencing operators
(amper_deref_expression [ "&" "*" ] @function)
(glob_deref_expression "*" @variable.special)
(glob_slot_expression "*" @variable.special)
(scalar_deref_expression [ "$" "*"] @variable)

; Generic array/hash in context
(_
  [
   array: (_) @variable
   hash: (_) @variable
  ])
(postfix_deref ["@" "$#" ] @variable "*" @variable)
(postfix_deref "%" @variable "*" @variable)
(slices hashref:_ [ "@" "%" ] @variable)
(slices arrayref:_ [ "@" "%" ] @variable)

; Comments
(comment) @comment

; Punctuation
([ "=>" "," ";" "->" ] @punctuation.delimiter)
([ "[" "]" "{" "}" "(" ")" ] @punctuation.bracket)

(_
  "{" @punctuation.special
  (varname)
  "}" @punctuation.special)

(varname
  (block
    "{" @punctuation.special
    "}" @punctuation.special))
