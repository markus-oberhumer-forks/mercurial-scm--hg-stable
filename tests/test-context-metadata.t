Tests about metadataonlyctx

  $ hg init
  $ echo A > A
  $ hg commit -A A -m 'Add A'
  $ echo B > B
  $ hg commit -A B -m 'Add B'
  $ hg rm A
  $ echo C > C
  $ echo B2 > B
  $ hg add C -q
  $ hg commit -m 'Remove A'

  $ cat > metaedit.py <<EOF
  > from mercurial import context, pycompat, registrar
  > cmdtable = {}
  > command = registrar.command(cmdtable)
  > @command(b'metaedit')
  > def metaedit(ui, repo, arg):
  >     # Modify commit message to "FOO"
  >     with repo.wlock(), repo.lock(), repo.transaction(b'metaedit'):
  >         old = repo[b'.']
  >         kwargs = dict(s.split(b'=', 1) for s in arg.split(b';'))
  >         if b'parents' in kwargs:
  >             kwargs[b'parents'] = map(int, kwargs[b'parents'].split(b','))
  >         new = context.metadataonlyctx(repo, old,
  >                                       **pycompat.strkwargs(kwargs))
  >         new.commit()
  > EOF
  $ hg --config extensions.metaedit=$TESTTMP/metaedit.py metaedit 'text=Changed'
  $ hg log -r tip
  changeset:   3:ad83e9e00ec9
  tag:         tip
  parent:      1:3afb7afe6632
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     Changed
  
  $ hg --config extensions.metaedit=$TESTTMP/metaedit.py metaedit 'parents=0' 2>&1 | grep -E '^RuntimeError'
  RuntimeError: can't reuse the manifest: its p1 doesn't match the new ctx p1

  $ hg --config extensions.metaedit=$TESTTMP/metaedit.py metaedit 'user=foo <foo@example.com>'
  $ hg log -r tip
  changeset:   4:1f86eaeca92b
  tag:         tip
  parent:      1:3afb7afe6632
  user:        foo <foo@example.com>
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     Remove A
  
