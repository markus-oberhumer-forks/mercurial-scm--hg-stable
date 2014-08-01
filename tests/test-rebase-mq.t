  $ cat >> $HGRCPATH <<EOF
  > [extensions]
  > rebase=
  > mq=
  > 
  > [mq]
  > plain=true
  > 
  > [alias]
  > tglog = log -G --template "{rev}: '{desc}' tags: {tags}\n"
  > EOF


  $ hg init a
  $ cd a
  $ hg qinit -c

  $ echo c1 > f
  $ hg add f
  $ hg ci -m C1

  $ echo r1 > f
  $ hg ci -m R1

  $ hg up -q 0

  $ hg qnew f.patch
  $ echo mq1 > f
  $ hg qref -m P0

  $ hg qnew f2.patch
  $ echo mq2 > f
  $ hg qref -m P1

  $ hg tglog
  @  3: 'P1' tags: f2.patch qtip tip
  |
  o  2: 'P0' tags: f.patch qbase
  |
  | o  1: 'R1' tags:
  |/
  o  0: 'C1' tags: qparent
  

Rebase - try to rebase on an applied mq patch:

  $ hg rebase -s 1 -d 3
  abort: cannot rebase onto an applied mq patch
  [255]

Rebase - same thing, but mq patch is default dest:

  $ hg up -q 1
  $ hg rebase
  abort: cannot rebase onto an applied mq patch
  [255]
  $ hg up -q qtip

Rebase - generate a conflict:

  $ hg rebase -s 2 -d 1
  merging f
  warning: conflicts during merge.
  merging f incomplete! (edit conflicts, then use 'hg resolve --mark')
  unresolved conflicts (see hg resolve, then hg rebase --continue)
  [1]

Fix the 1st conflict:

  $ echo mq1r1 > f
  $ hg resolve -m f
  (no more unresolved files)
  $ hg rebase -c
  merging f
  warning: conflicts during merge.
  merging f incomplete! (edit conflicts, then use 'hg resolve --mark')
  unresolved conflicts (see hg resolve, then hg rebase --continue)
  [1]

Fix the 2nd conflict:

  $ echo mq1r1mq2 > f
  $ hg resolve -m f
  (no more unresolved files)
  $ hg rebase -c
  saved backup bundle to $TESTTMP/a/.hg/strip-backup/*-backup.hg (glob)

  $ hg tglog
  @  3: 'P1' tags: f2.patch qtip tip
  |
  o  2: 'P0' tags: f.patch qbase
  |
  o  1: 'R1' tags: qparent
  |
  o  0: 'C1' tags:
  
  $ hg up -q qbase

  $ cat f
  mq1r1

  $ cat .hg/patches/f.patch
  # HG changeset patch
  # User test
  # Date ?????????? ? (glob)
  #      * (glob)
  # Node ID ???????????????????????????????????????? (glob)
  # Parent  bac9ed9960d8992bcad75864a879fa76cadaf1b0
  P0
  
  diff -r bac9ed9960d8 -r ???????????? f (glob)
  --- a/f	Thu Jan 01 00:00:00 1970 +0000
  +++ b/f	??? ??? ?? ??:??:?? ???? ????? (glob)
  @@ -1,1 +1,1 @@
  -r1
  +mq1r1

Update to qtip:

  $ hg up -q qtip

  $ cat f
  mq1r1mq2

  $ cat .hg/patches/f2.patch
  # HG changeset patch
  # User test
  # Date ?????????? ? (glob)
  #      * (glob)
  # Node ID ???????????????????????????????????????? (glob)
  # Parent  ???????????????????????????????????????? (glob)
  P1
  
  diff -r ???????????? -r ???????????? f (glob)
  --- a/f	??? ??? ?? ??:??:?? ???? ????? (glob)
  +++ b/f	??? ??? ?? ??:??:?? ???? ????? (glob)
  @@ -1,1 +1,1 @@
  -mq1r1
  +mq1r1mq2

Adding one git-style patch and one normal:

  $ hg qpop -a
  popping f2.patch
  popping f.patch
  patch queue now empty

  $ rm -fr .hg/patches
  $ hg qinit -c

  $ hg up -q 0

  $ hg qnew --git f_git.patch
  $ echo mq1 > p
  $ hg add p
  $ hg qref --git -m 'P0 (git)'

  $ hg qnew f.patch
  $ echo mq2 > p
  $ hg qref -m P1
  $ hg qci -m 'save patch state'

  $ hg qseries -s
  f_git.patch: P0 (git)
  f.patch: P1

  $ hg -R .hg/patches manifest
  .hgignore
  f.patch
  f_git.patch
  series

  $ cat .hg/patches/f_git.patch
  P0 (git)
  
  diff --git a/p b/p
  new file mode 100644
  --- /dev/null
  +++ b/p
  @@ -0,0 +1,1 @@
  +mq1

  $ cat .hg/patches/f.patch
  P1
  
  diff -r ???????????? p (glob)
  --- a/p	??? ??? ?? ??:??:?? ???? ????? (glob)
  +++ b/p	??? ??? ?? ??:??:?? ???? ????? (glob)
  @@ -1,1 +1,1 @@
  -mq1
  +mq2


Rebase the applied mq patches:

  $ hg rebase -s 2 -d 1
  saved backup bundle to $TESTTMP/a/.hg/strip-backup/*-backup.hg (glob)

  $ hg qci -m 'save patch state'

  $ hg qseries -s
  f_git.patch: P0 (git)
  f.patch: P1

  $ hg -R .hg/patches manifest
  .hgignore
  f.patch
  f_git.patch
  series

  $ cat .hg/patches/f_git.patch
  # HG changeset patch
  # User test
  # Date ?????????? ? (glob)
  #      * (glob)
  # Node ID ???????????????????????????????????????? (glob)
  # Parent  bac9ed9960d8992bcad75864a879fa76cadaf1b0
  P0 (git)
  
  diff --git a/p b/p
  new file mode 100644
  --- /dev/null
  +++ b/p
  @@ -0,0 +1,1 @@
  +mq1

  $ cat .hg/patches/f.patch
  # HG changeset patch
  # User test
  # Date ?????????? ? (glob)
  #      * (glob)
  # Node ID ???????????????????????????????????????? (glob)
  # Parent  ???????????????????????????????????????? (glob)
  P1
  
  diff -r ???????????? -r ???????????? p (glob)
  --- a/p	??? ??? ?? ??:??:?? ???? ????? (glob)
  +++ b/p	??? ??? ?? ??:??:?? ???? ????? (glob)
  @@ -1,1 +1,1 @@
  -mq1
  +mq2

  $ cd ..

Rebase with guards

  $ hg init foo
  $ cd foo
  $ echo a > a
  $ hg ci -Am a
  adding a

Create mq repo with guarded patches foo and bar and empty patch:

  $ hg qinit
  $ echo guarded > guarded
  $ hg add guarded
  $ hg qnew guarded
  $ hg qnew empty-important -m 'important commit message'
  $ echo bar > bar
  $ hg add bar
  $ hg qnew bar
  $ echo foo > foo
  $ hg add foo
  $ hg qnew foo
  $ hg qpop -a
  popping foo
  popping bar
  popping empty-important
  popping guarded
  patch queue now empty
  $ hg qguard guarded +guarded
  $ hg qguard bar +baz
  $ hg qguard foo +baz
  $ hg qselect baz
  number of unguarded, unapplied patches has changed from 1 to 3
  $ hg qpush bar
  applying empty-important
  patch empty-important is empty
  applying bar
  now at: bar

  $ hg qguard -l
  guarded: +guarded
  empty-important: unguarded
  bar: +baz
  foo: +baz

  $ hg tglog
  @  2: 'imported patch bar' tags: bar qtip tip
  |
  o  1: 'important commit message' tags: empty-important qbase
  |
  o  0: 'a' tags: qparent
  
Create new head to rebase bar onto:

  $ hg up -C 0
  0 files updated, 0 files merged, 1 files removed, 0 files unresolved
  $ echo b > b
  $ hg add b
  $ hg ci -m b
  created new head
  $ hg up -C 2
  1 files updated, 0 files merged, 1 files removed, 0 files unresolved
  $ echo a >> a
  $ hg qref

  $ hg tglog
  @  3: '[mq]: bar' tags: bar qtip tip
  |
  | o  2: 'b' tags:
  | |
  o |  1: 'important commit message' tags: empty-important qbase
  |/
  o  0: 'a' tags: qparent
  

Rebase bar (make sure series order is preserved and empty-important also is
removed from the series):

  $ hg qseries
  guarded
  empty-important
  bar
  foo
  $ [ -f .hg/patches/empty-important ]
  $ hg -q rebase -d 2
  $ hg qseries
  guarded
  bar
  foo
  $ [ -f .hg/patches/empty-important ]
  [1]

  $ hg qguard -l
  guarded: +guarded
  bar: +baz
  foo: +baz

  $ hg tglog
  @  2:* '[mq]: bar' tags: bar qbase qtip tip (glob)
  |
  o  1:* 'b' tags: qparent (glob)
  |
  o  0:* 'a' tags: (glob)
  
  $ cd ..
