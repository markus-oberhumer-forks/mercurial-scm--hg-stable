Test issue2761

  $ hg init repo
  $ cd repo

  $ touch to-be-deleted
  $ hg add
  adding to-be-deleted
  $ hg ci -m first
  $ echo a > to-be-deleted
  $ hg ci -m second
  $ rm to-be-deleted
  $ hg diff -r 0

Same issue, different code path

  $ hg up -C
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved
  $ touch does-not-exist-in-1
  $ hg add
  adding does-not-exist-in-1
  $ hg ci -m third
  $ rm does-not-exist-in-1
  $ hg diff -r 1
