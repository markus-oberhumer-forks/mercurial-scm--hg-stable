  $ hg init repo
  $ cd repo
  $ echo a > a
  $ hg commit -A -ma
  adding a

  $ echo b >> a
  $ hg commit -mb

  $ echo c >> a
  $ hg commit -mc

  $ hg up 1
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved
  $ echo d >> a
  $ hg commit -md
  created new head

  $ hg up 1
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved
  $ echo e >> a
  $ hg commit -me
  created new head

  $ hg up 1
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved

Should fail because not at a head:

  $ hg merge
  abort: working directory not at a head revision
  (use 'hg update' or merge with an explicit revision)
  [255]

  $ hg up
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved
  updated to "f25cbe84d8b3: e"
  2 other heads for branch "default"

Should fail because > 2 heads:

  $ HGMERGE=internal:other; export HGMERGE
  $ hg merge
  abort: branch 'default' has 3 heads - please merge with an explicit rev
  (run 'hg heads .' to see heads, specify rev with -r)
  [255]

Should succeed (we're specifying commands.merge.require-rev=True just to test
that it allows merge to succeed if we specify a revision):

  $ hg merge 2 --config commands.merge.require-rev=True
  0 files updated, 1 files merged, 0 files removed, 0 files unresolved
  (branch merge, don't forget to commit)
  $ hg id -Tjson
  [
   {
    "bookmarks": [],
    "branch": "default",
    "dirty": "+",
    "id": "f25cbe84d8b320e298e7703f18a25a3959518c23+2d95304fed5d89bc9d70b2a0d02f0d567469c3ab+",
    "node": "ffffffffffffffffffffffffffffffffffffffff",
    "parents": ["f25cbe84d8b320e298e7703f18a25a3959518c23", "2d95304fed5d89bc9d70b2a0d02f0d567469c3ab"],
    "tags": ["tip"]
   }
  ]
  $ hg commit -mm1

Should fail because we didn't specify a revision (even though it would have
succeeded without this):

  $ hg merge --config commands.merge.require-rev=True
  abort: configuration requires specifying revision to merge with
  [10]

Should succeed - 2 heads:

  $ hg merge -P
  changeset:   3:ea9ff125ff88
  parent:      1:1846eede8b68
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     d
  
  $ hg merge
  0 files updated, 1 files merged, 0 files removed, 0 files unresolved
  (branch merge, don't forget to commit)
  $ hg commit -mm2

  $ hg id -r 1 -Tjson
  [
   {
    "bookmarks": [],
    "branch": "default",
    "id": "1846eede8b6886d8cc8a88c96a687b7fe8f3b9d1",
    "node": "1846eede8b6886d8cc8a88c96a687b7fe8f3b9d1",
    "tags": []
   }
  ]

Should fail because we didn't specify a revision (even though it would have
failed without this due to being on tip, but this check comes first):

  $ hg merge --config commands.merge.require-rev=True
  abort: configuration requires specifying revision to merge with
  [10]

Should fail because at tip:

  $ hg merge
  abort: nothing to merge
  [255]

  $ hg up 0
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved

Should fail because there is only one head:

  $ hg merge
  abort: nothing to merge
  (use 'hg update' instead)
  [255]

  $ hg up 3
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved

  $ echo f >> a
  $ hg branch foobranch
  marked working directory as branch foobranch
  (branches are permanent and global, did you want a bookmark?)
  $ hg commit -mf

Should fail because merge with other branch:

  $ hg merge
  abort: branch 'foobranch' has one head - please merge with an explicit rev
  (run 'hg heads' to see all heads, specify rev with -r)
  [255]


Test for issue2043: ensure that 'merge -P' shows ancestors of 6 that
are not ancestors of 7, regardless of where their common ancestors are.

Merge preview not affected by common ancestor:

  $ hg up -q 7
  $ hg merge -q -P 6
  2:2d95304fed5d
  4:f25cbe84d8b3
  5:a431fabd6039
  6:e88e33f3bf62

Test experimental destination revset

  $ hg log -r '_destmerge()'
  abort: branch 'foobranch' has one head - please merge with an explicit rev
  (run 'hg heads' to see all heads, specify rev with -r)
  [255]

(on a branch with a two heads)

  $ hg up 5
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved
  $ echo f >> a
  $ hg commit -mf
  created new head
  $ hg log -r '_destmerge()'
  changeset:   6:e88e33f3bf62
  parent:      5:a431fabd6039
  parent:      3:ea9ff125ff88
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     m2
  

(from the other head)

  $ hg log -r '_destmerge(e88e33f3bf62)'
  changeset:   8:b613918999e2
  tag:         tip
  parent:      5:a431fabd6039
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     f
  

(from unrelated branch)

  $ hg log -r '_destmerge(foobranch)'
  abort: branch 'foobranch' has one head - please merge with an explicit rev
  (run 'hg heads' to see all heads, specify rev with -r)
  [255]
