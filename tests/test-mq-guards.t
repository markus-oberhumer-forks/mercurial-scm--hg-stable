  $ echo "[extensions]" >> $HGRCPATH
  $ echo "mq=" >> $HGRCPATH

  $ hg init repo
  $ cd repo
  $ hg qinit

  $ echo x > x
  $ hg ci -Ama
  adding x

  $ hg qnew a.patch
  $ echo a > a
  $ hg add a
  $ hg qrefresh

  $ hg qnew b.patch
  $ echo b > b
  $ hg add b
  $ hg qrefresh

  $ hg qnew c.patch
  $ echo c > c
  $ hg add c
  $ hg qrefresh

  $ hg qpop -a
  popping c.patch
  popping b.patch
  popping a.patch
  patch queue now empty


should fail

  $ hg qguard does-not-exist.patch +bleh
  abort: no patch named does-not-exist.patch
  [255]


should fail

  $ hg qguard +fail
  abort: no patches applied
  [255]

  $ hg qpush
  applying a.patch
  now at: a.patch

should guard a.patch

  $ hg qguard +a

should print +a

  $ hg qguard
  a.patch: +a
  $ hg qpop
  popping a.patch
  patch queue now empty


should fail

  $ hg qpush a.patch
  cannot push 'a.patch' - guarded by '+a'
  [1]

  $ hg qguard a.patch
  a.patch: +a

should push b.patch

  $ hg qpush
  applying b.patch
  now at: b.patch

  $ hg qpop
  popping b.patch
  patch queue now empty

test selection of an empty guard

  $ hg qselect ""
  abort: guard cannot be an empty string
  [255]
  $ hg qselect a
  number of unguarded, unapplied patches has changed from 2 to 3

should push a.patch

  $ hg qpush
  applying a.patch
  now at: a.patch

  $ hg qguard -- c.patch -a

should print -a

  $ hg qguard c.patch
  c.patch: -a


should skip c.patch

  $ hg qpush -a
  applying b.patch
  skipping c.patch - guarded by '-a'
  now at: b.patch
  $ hg qnext
  all patches applied
  [1]

should display b.patch

  $ hg qtop
  b.patch

  $ hg qguard -n c.patch

should push c.patch

  $ hg qpush -a
  applying c.patch
  now at: c.patch

  $ hg qpop -a
  popping c.patch
  popping b.patch
  popping a.patch
  patch queue now empty
  $ hg qselect -n
  guards deactivated
  number of unguarded, unapplied patches has changed from 3 to 2

should push all

  $ hg qpush -a
  applying b.patch
  applying c.patch
  now at: c.patch

  $ hg qpop -a
  popping c.patch
  popping b.patch
  patch queue now empty
  $ hg qguard a.patch +1
  $ hg qguard b.patch +2
  $ hg qselect 1
  number of unguarded, unapplied patches has changed from 1 to 2

should push a.patch, not b.patch

  $ hg qpush
  applying a.patch
  now at: a.patch
  $ hg qpush
  applying c.patch
  now at: c.patch
  $ hg qpop -a
  popping c.patch
  popping a.patch
  patch queue now empty

  $ hg qselect 2

should push b.patch

  $ hg qpush
  applying b.patch
  now at: b.patch
  $ hg qpush -a
  applying c.patch
  now at: c.patch
  $ hg qprev
  b.patch

Used to be an issue with holes in the patch sequence
So, put one hole on the base and ask for topmost patch.

  $ hg qtop
  c.patch
  $ hg qpop -a
  popping c.patch
  popping b.patch
  patch queue now empty

  $ hg qselect 1 2
  number of unguarded, unapplied patches has changed from 2 to 3

should push a.patch, b.patch

  $ hg qpush
  applying a.patch
  now at: a.patch
  $ hg qpush
  applying b.patch
  now at: b.patch
  $ hg qpop -a
  popping b.patch
  popping a.patch
  patch queue now empty

  $ hg qguard -- a.patch +1 +2 -3
  $ hg qselect 1 2 3
  number of unguarded, unapplied patches has changed from 3 to 2


list patches and guards

  $ hg qguard -l
  a.patch: +1 +2 -3
  b.patch: +2
  c.patch: unguarded

have at least one patch applied to test coloring

  $ hg qpush
  applying b.patch
  now at: b.patch

list patches and guards with color

  $ hg --config extensions.color= qguard --config color.mode=ansi \
  >     -l --color=always
  \x1b[0;30;1ma.patch\x1b[0m: \x1b[0;33m+1\x1b[0m \x1b[0;33m+2\x1b[0m \x1b[0;31m-3\x1b[0m (esc)
  \x1b[0;34;1;4mb.patch\x1b[0m: \x1b[0;33m+2\x1b[0m (esc)
  \x1b[0;30;1mc.patch\x1b[0m: \x1b[0;32munguarded\x1b[0m (esc)

should pop b.patch

  $ hg qpop
  popping b.patch
  patch queue now empty

list series

  $ hg qseries -v
  0 G a.patch
  1 U b.patch
  2 U c.patch

list guards

  $ hg qselect
  1
  2
  3

should push b.patch

  $ hg qpush
  applying b.patch
  now at: b.patch

  $ hg qpush -a
  applying c.patch
  now at: c.patch
  $ hg qselect -n --reapply -v
  guards deactivated
  popping guarded patches
  popping c.patch
  popping b.patch
  patch queue now empty
  reapplying unguarded patches
  skipping a.patch - guarded by '+1' '+2'
  skipping b.patch - guarded by '+2'
  skipping a.patch - guarded by '+1' '+2'
  skipping b.patch - guarded by '+2'
  applying c.patch
  patching file c
  adding c
  committing files:
  c
  committing manifest
  committing changelog
  now at: c.patch

guards in series file: +1 +2 -3

  $ hg qselect -s
  +1
  +2
  -3

should show c.patch

  $ hg qapplied
  c.patch

  $ hg qrename a.patch new.patch

should show :


new.patch: +1 +2 -3


b.patch: +2


c.patch: unguarded

  $ hg qguard -l
  new.patch: +1 +2 -3
  b.patch: +2
  c.patch: unguarded

  $ hg qnew d.patch
  $ hg qpop
  popping d.patch
  now at: c.patch

should show new.patch and b.patch as Guarded, c.patch as Applied


and d.patch as Unapplied

  $ hg qseries -v
  0 G new.patch
  1 G b.patch
  2 A c.patch
  3 U d.patch

qseries again, but with color

  $ hg --config extensions.color= --config color.mode=ansi qseries -v --color=always
  0 G \x1b[0;30;1mnew.patch\x1b[0m (esc)
  1 G \x1b[0;30;1mb.patch\x1b[0m (esc)
  2 A \x1b[0;34;1;4mc.patch\x1b[0m (esc)
  3 U \x1b[0;30;1md.patch\x1b[0m (esc)

  $ hg qguard d.patch +2

new.patch, b.patch: Guarded. c.patch: Applied. d.patch: Guarded.

  $ hg qseries -v
  0 G new.patch
  1 G b.patch
  2 A c.patch
  3 G d.patch

  $ qappunappv()
  > {
  >     for command in qapplied "qapplied -v" qunapplied "qunapplied -v"; do
  >         echo % hg $command
  >         hg $command
  >     done
  > }

  $ hg qpop -a
  popping c.patch
  patch queue now empty
  $ hg qguard -l
  new.patch: +1 +2 -3
  b.patch: +2
  c.patch: unguarded
  d.patch: +2
  $ qappunappv
  % hg qapplied
  % hg qapplied -v
  % hg qunapplied
  c.patch
  % hg qunapplied -v
  0 G new.patch
  1 G b.patch
  2 U c.patch
  3 G d.patch
  $ hg qselect 1
  number of unguarded, unapplied patches has changed from 1 to 2
  $ qappunappv
  % hg qapplied
  % hg qapplied -v
  % hg qunapplied
  new.patch
  c.patch
  % hg qunapplied -v
  0 U new.patch
  1 G b.patch
  2 U c.patch
  3 G d.patch
  $ hg qpush -a
  applying new.patch
  skipping b.patch - guarded by '+2'
  applying c.patch
  skipping d.patch - guarded by '+2'
  now at: c.patch
  $ qappunappv
  % hg qapplied
  new.patch
  c.patch
  % hg qapplied -v
  0 A new.patch
  1 G b.patch
  2 A c.patch
  % hg qunapplied
  % hg qunapplied -v
  3 G d.patch
  $ hg qselect 2
  number of unguarded, unapplied patches has changed from 0 to 1
  $ qappunappv
  % hg qapplied
  new.patch
  c.patch
  % hg qapplied -v
  0 A new.patch
  1 U b.patch
  2 A c.patch
  % hg qunapplied
  d.patch
  % hg qunapplied -v
  3 U d.patch

  $ for patch in `hg qseries`; do
  >     echo % hg qapplied $patch
  >     hg qapplied $patch
  >     echo % hg qunapplied $patch
  >     hg qunapplied $patch
  > done
  % hg qapplied new.patch
  new.patch
  % hg qunapplied new.patch
  b.patch
  d.patch
  % hg qapplied b.patch
  new.patch
  % hg qunapplied b.patch
  d.patch
  % hg qapplied c.patch
  new.patch
  c.patch
  % hg qunapplied c.patch
  d.patch
  % hg qapplied d.patch
  new.patch
  c.patch
  % hg qunapplied d.patch


hg qseries -m: only b.patch should be shown
the guards file was not ignored in the past

  $ hg qdelete -k b.patch
  $ hg qseries -m
  b.patch

hg qseries -m with color

  $ hg --config extensions.color= --config color.mode=ansi qseries -m --color=always
  \x1b[0;31;1mb.patch\x1b[0m (esc)


excercise corner cases in "qselect --reapply"

  $ hg qpop -a
  popping c.patch
  popping new.patch
  patch queue now empty
  $ hg qguard -- new.patch -not-new
  $ hg qguard -- c.patch -not-c
  $ hg qguard -- d.patch -not-d
  $ hg qpush -a
  applying new.patch
  applying c.patch
  applying d.patch
  patch d.patch is empty
  now at: d.patch
  $ hg qguard -l
  new.patch: -not-new
  c.patch: -not-c
  d.patch: -not-d
  $ hg qselect --reapply not-d
  popping guarded patches
  popping d.patch
  now at: c.patch
  reapplying unguarded patches
  cannot push 'd.patch' - guarded by '-not-d'
  $ hg qser -v
  0 A new.patch
  1 A c.patch
  2 G d.patch
  $ hg qselect --reapply -n
  guards deactivated
  $ hg qpush
  applying d.patch
  patch d.patch is empty
  now at: d.patch
  $ hg qser -v
  0 A new.patch
  1 A c.patch
  2 A d.patch
  $ hg qselect --reapply not-c
  popping guarded patches
  popping d.patch
  popping c.patch
  now at: new.patch
  reapplying unguarded patches
  applying d.patch
  patch d.patch is empty
  now at: d.patch
  $ hg qser -v
  0 A new.patch
  1 G c.patch
  2 A d.patch
  $ hg qselect --reapply not-new
  popping guarded patches
  popping d.patch
  popping new.patch
  patch queue now empty
  reapplying unguarded patches
  applying c.patch
  applying d.patch
  patch d.patch is empty
  now at: d.patch
  $ hg qser -v
  0 G new.patch
  1 A c.patch
  2 A d.patch

test that qselect shows "number of guarded, applied patches" correctly

  $ hg qimport -q -e b.patch
  adding b.patch to series file
  $ hg qguard -- b.patch -not-b
  $ hg qpop -a -q
  patch queue now empty
  $ hg qunapplied -v
  0 G new.patch
  1 U c.patch
  2 U d.patch
  3 U b.patch
  $ hg qselect not-new not-c
  number of unguarded, unapplied patches has changed from 3 to 2
  $ hg qpush -q -a
  patch d.patch is empty
  now at: b.patch

  $ hg qapplied -v
  0 G new.patch
  1 G c.patch
  2 A d.patch
  3 A b.patch
  $ hg qselect --none
  guards deactivated
  $ hg qselect not-new not-c not-d
  number of guarded, applied patches has changed from 0 to 1

test that "qselect --reapply" reapplies patches successfully when the
already applied patch becomes unguarded and it follows the already
guarded (= not yet applied) one.

  $ hg qpop -q -a
  patch queue now empty
  $ hg qselect not-new not-c
  number of unguarded, unapplied patches has changed from 1 to 2
  $ hg qpush -q -a
  patch d.patch is empty
  now at: b.patch
  $ hg qapplied -v
  0 G new.patch
  1 G c.patch
  2 A d.patch
  3 A b.patch
  $ hg qselect -q --reapply not-c not-b
  now at: d.patch
  cannot push 'b.patch' - guarded by '-not-b'
  $ hg qseries -v
  0 U new.patch
  1 G c.patch
  2 A d.patch
  3 G b.patch

test that "qselect --reapply" checks applied patches correctly when no
applied patches becomes guarded but some of unapplied ones become
unguarded.

  $ hg qpop -q -a
  patch queue now empty
  $ hg qselect not-new not-c not-d
  number of unguarded, unapplied patches has changed from 2 to 1
  $ hg qpush -q -a
  now at: b.patch
  $ hg qapplied -v
  0 G new.patch
  1 G c.patch
  2 G d.patch
  3 A b.patch
  $ hg qselect -q --reapply not-new not-c
  $ hg qseries -v
  0 G new.patch
  1 G c.patch
  2 U d.patch
  3 A b.patch
