Test changesets filtering during exchanges (some tests are still in
test-obsolete.t)

  $ cat >> $HGRCPATH << EOF
  > [experimental]
  > evolution.createmarkers=True
  > EOF

Push does not corrupt remote
----------------------------

Create a DAG where a changeset reuses a revision from a file first used in an
extinct changeset.

  $ hg init local
  $ cd local
  $ echo 'base' > base
  $ hg commit -Am base
  adding base
  $ echo 'A' > A
  $ hg commit -Am A
  adding A
  $ hg up 0
  0 files updated, 0 files merged, 1 files removed, 0 files unresolved
  $ hg revert -ar 1
  adding A
  $ hg commit -Am "A'"
  created new head
  $ hg log -G --template='{desc} {node}'
  @  A' f89bcc95eba5174b1ccc3e33a82e84c96e8338ee
  |
  | o  A 9d73aac1b2ed7d53835eaeec212ed41ea47da53a
  |/
  o  base d20a80d4def38df63a4b330b7fb688f3d4cae1e3
  
  $ hg debugobsolete 9d73aac1b2ed7d53835eaeec212ed41ea47da53a f89bcc95eba5174b1ccc3e33a82e84c96e8338ee
  1 new obsolescence markers
  obsoleted 1 changesets

Push it. The bundle should not refer to the extinct changeset.

  $ hg init ../other
  $ hg push ../other
  pushing to ../other
  searching for changes
  adding changesets
  adding manifests
  adding file changes
  added 2 changesets with 2 changes to 2 files
  $ hg -R ../other verify -q

Adding a changeset going extinct locally
------------------------------------------

Pull a changeset that will immediatly goes extinct (because you already have a
marker to obsolete him)
(test resolution of issue3788)

  $ hg phase --draft --force f89bcc95eba5
  $ hg phase -R ../other --draft --force f89bcc95eba5
  $ hg commit --amend -m "A''"
  $ hg --hidden --config extensions.mq= strip  --no-backup f89bcc95eba5
  $ hg pull ../other
  pulling from ../other
  searching for changes
  adding changesets
  adding manifests
  adding file changes
  added 1 changesets with 0 changes to 1 files (+1 heads)
  1 new phase-divergent changesets
  new changesets f89bcc95eba5
  (run 'hg heads' to see heads, 'hg merge' to merge)

check that bundle is not affected

  $ hg bundle --hidden --rev f89bcc95eba5 --base "f89bcc95eba5^" ../f89bcc95eba5.hg
  1 changesets found
  $ hg --hidden --config extensions.mq= strip --no-backup f89bcc95eba5
  $ hg unbundle ../f89bcc95eba5.hg
  adding changesets
  adding manifests
  adding file changes
  added 1 changesets with 0 changes to 1 files (+1 heads)
  (1 other changesets obsolete on arrival)
  (run 'hg heads' to see heads)

check-that bundle can contain markers:

  $ hg bundle --hidden --rev f89bcc95eba5 --base "f89bcc95eba5^" ../f89bcc95eba5-obs.hg --config experimental.evolution.bundle-obsmarker=1
  1 changesets found
  $ hg debugbundle ../f89bcc95eba5.hg
  Stream params: {Compression: BZ}
  changegroup -- {nbchanges: 1, version: 02} (mandatory: True)
      f89bcc95eba5174b1ccc3e33a82e84c96e8338ee
  cache:rev-branch-cache -- {} (mandatory: False)
  $ hg debugbundle ../f89bcc95eba5-obs.hg
  Stream params: {Compression: BZ}
  changegroup -- {nbchanges: 1, version: 02} (mandatory: True)
      f89bcc95eba5174b1ccc3e33a82e84c96e8338ee
  cache:rev-branch-cache -- {} (mandatory: False)
  obsmarkers -- {} (mandatory: True)
      version: 1 (70 bytes)
      9d73aac1b2ed7d53835eaeec212ed41ea47da53a f89bcc95eba5174b1ccc3e33a82e84c96e8338ee 0 (Thu Jan 01 00:00:00 1970 +0000) {'user': 'test'}

  $ cd ..

pull does not fetch excessive changesets when common node is hidden (issue4982)
-------------------------------------------------------------------------------

initial repo with server and client matching

  $ hg init pull-hidden-common
  $ cd pull-hidden-common
  $ touch foo
  $ hg -q commit -A -m initial
  $ echo 1 > foo
  $ hg commit -m 1
  $ echo 2a > foo
  $ hg commit -m 2a
  $ cd ..
  $ hg clone --pull pull-hidden-common pull-hidden-common-client
  requesting all changes
  adding changesets
  adding manifests
  adding file changes
  added 3 changesets with 3 changes to 1 files
  new changesets 96ee1d7354c4:6a29ed9c68de
  updating to branch default
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved

server obsoletes the old head

  $ cd pull-hidden-common
  $ hg -q up -r 1
  $ echo 2b > foo
  $ hg -q commit -m 2b
  $ hg debugobsolete 6a29ed9c68defff1a139e5c6fa9696fb1a75783d bec0734cd68e84477ba7fc1d13e6cff53ab70129
  1 new obsolescence markers
  obsoleted 1 changesets
  $ cd ..

client only pulls down 1 changeset

  $ cd pull-hidden-common-client
  $ hg pull --debug
  pulling from $TESTTMP/pull-hidden-common
  query 1; heads
  searching for changes
  taking quick initial sample
  query 2; still undecided: 2, sample size is: 2
  2 total queries in *.????s (glob)
  1 changesets found
  list of changesets:
  bec0734cd68e84477ba7fc1d13e6cff53ab70129
  listing keys for "bookmarks"
  bundle2-output-bundle: "HG20", 3 parts total
  bundle2-output-part: "changegroup" (params: 1 mandatory 1 advisory) streamed payload
  bundle2-output-part: "listkeys" (params: 1 mandatory) empty payload
  bundle2-output-part: "phase-heads" 24 bytes payload
  bundle2-input-bundle: with-transaction
  bundle2-input-part: "changegroup" (params: 1 mandatory 1 advisory) supported
  adding changesets
  add changeset bec0734cd68e
  adding manifests
  adding file changes
  adding foo revisions
  bundle2-input-part: total payload size 486
  bundle2-input-part: "listkeys" (params: 1 mandatory) supported
  bundle2-input-part: "phase-heads" supported
  bundle2-input-part: total payload size 24
  bundle2-input-bundle: 3 parts total
  checking for updated bookmarks
  updating the branch cache
  added 1 changesets with 1 changes to 1 files (+1 heads)
  new changesets bec0734cd68e
  (run 'hg heads' to see heads, 'hg merge' to merge)
