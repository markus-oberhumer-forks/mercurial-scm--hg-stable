  > from mercurial import commands, context, pycompat, registrar
  > @command(b'eval', [], b'hg eval CMD')
  >     cmd = b" ".join(cmds)
  >     res = pycompat.bytestr(eval(cmd, globals(), locals()))
  >     ui.warn(b"%s" % res)
  $ hg eval "context.arbitraryfilectx(b'A', repo).cmp(repo[None][b'real_A'])"
  $ hg eval "not filecmp.cmp(b'A', b'real_A')"
  $ hg eval "context.arbitraryfilectx(b'A', repo).cmp(repo[None][b'A'])"
  $ hg eval "context.arbitraryfilectx(b'A', repo).cmp(repo[None][b'B'])"
  $ hg eval "not filecmp.cmp(b'A', b'B')"
  $ hg eval "context.arbitraryfilectx(b'real_A', repo).cmp(repo[None][b'sym_A'])"
  $ hg eval "not filecmp.cmp(b'real_A', b'sym_A')"
  $ hg eval "not filecmp.cmp(b'real_A', b'sym_A')"