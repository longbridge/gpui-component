((substitution_regexp
  (replacement) @injection.content
  (substitution_regexp_modifiers) @_modifiers)
  (#match? @_modifiers "e")
  (#not-match? @_modifiers "e.*e")
  (#set! injection.language "perl"))
