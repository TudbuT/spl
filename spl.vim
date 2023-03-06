
if exists("b:current_syntax")
  finish
endif

syn match Comment /".*?";/
syn match Number /\<[0-9._]*\>/
syn match Function /\<func[ \n]\+[^ ]\+[ \n]\+{ .*[ ]*|\|{ .*[ ]*|\|{\|}/
syn keyword Keyword while if exit eq lt gt neg or and not + - * ++ -- % / with namespace
syn match Keyword /;/
syn keyword Type pop dup swap
syn match Type /=[a-zA-Z0-9_\-]\+\|\<_[a-zA-Z0-9_\-]\+\>/
syn match Identifier /[a-zA-Z0-9_\-]\+:\|\<this\>/
syn match String /"[^"]*"/
syn match Typedef /\<def[ \n]\+[^ ]*\|construct/
