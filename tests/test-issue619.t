https://bz.mercurial-scm.org/619

  $ hg init repo
  $ cd repo
  $ echo a > a
  $ hg ci -Ama
  adding a

  $ echo b > b
  $ hg branch b
  marked working directory as branch b
  (branches are permanent and global, did you want a bookmark?)
  $ hg ci -Amb
  adding b

  $ hg co -C 0
  0 files updated, 0 files merged, 1 files removed, 0 files unresolved

Fast-forward:

  $ hg merge b
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved
  (branch merge, don't forget to commit)
  $ hg ci -Ammerge

Bogus fast-forward should fail:

  $ hg merge b
  abort: merging with a working directory ancestor has no effect
  [255]

Even with strange revset (issue4465)

  $ hg merge ::.
  abort: merging with a working directory ancestor has no effect
  [255]

