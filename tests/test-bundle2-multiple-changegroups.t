Create an extension to test bundle2 with multiple changegroups

  $ cat > bundle2.py <<EOF
  > """
  > """
  > from mercurial import changegroup, discovery, exchange
  > 
  > def _getbundlechangegrouppart(bundler, repo, source, bundlecaps=None,
  >                               b2caps=None, heads=None, common=None,
  >                               **kwargs):
  >     # Create two changegroups given the common changesets and heads for the
  >     # changegroup part we are being requested. Use the parent of each head
  >     # in 'heads' as intermediate heads for the first changegroup.
  >     intermediates = [repo[r].p1().node() for r in heads]
  >     outgoing = discovery.outgoing(repo, common, intermediates)
  >     cg = changegroup.makechangegroup(repo, outgoing, b'01',
  >                                      source, bundlecaps=bundlecaps)
  >     bundler.newpart(b'output', data=b'changegroup1')
  >     bundler.newpart(b'changegroup', data=cg.getchunks())
  >     outgoing = discovery.outgoing(repo, common + intermediates, heads)
  >     cg = changegroup.makechangegroup(repo, outgoing, b'01',
  >                                      source, bundlecaps=bundlecaps)
  >     bundler.newpart(b'output', data=b'changegroup2')
  >     bundler.newpart(b'changegroup', data=cg.getchunks())
  > 
  > def _pull(repo, *args, **kwargs):
  >   pullop = _orig_pull(repo, *args, **kwargs)
  >   repo.ui.write(b'pullop.cgresult is %d\n' % pullop.cgresult)
  >   return pullop
  > 
  > _orig_pull = exchange.pull
  > exchange.pull = _pull
  > exchange.getbundle2partsmapping[b'changegroup'] = _getbundlechangegrouppart
  > EOF

  $ cat >> $HGRCPATH << EOF
  > [command-templates]
  > log={rev}:{node|short} {phase} {author} {bookmarks} {desc|firstline}
  > EOF

Start with a simple repository with a single commit

  $ hg init repo
  $ cd repo
  $ cat > .hg/hgrc << EOF
  > [extensions]
  > bundle2=$TESTTMP/bundle2.py
  > EOF

  $ echo A > A
  $ hg commit -A -m A -q
  $ cd ..

Clone

  $ hg clone -q repo clone

Add two linear commits

  $ cd repo
  $ echo B > B
  $ hg commit -A -m B -q
  $ echo C > C
  $ hg commit -A -m C -q

  $ cd ../clone
  $ cat >> .hg/hgrc <<EOF
  > [hooks]
  > pretxnchangegroup = sh -c "printenv.py --line pretxnchangegroup"
  > changegroup = sh -c "printenv.py --line changegroup"
  > incoming = sh -c "printenv.py --line incoming"
  > EOF

Pull the new commits in the clone

  $ hg pull
  pulling from $TESTTMP/repo
  searching for changes
  remote: changegroup1
  adding changesets
  adding manifests
  adding file changes
  pretxnchangegroup hook: HG_HOOKNAME=pretxnchangegroup
  HG_HOOKTYPE=pretxnchangegroup
  HG_NODE=27547f69f25460a52fff66ad004e58da7ad3fb56
  HG_NODE_LAST=27547f69f25460a52fff66ad004e58da7ad3fb56
  HG_PENDING=$TESTTMP/clone
  HG_SOURCE=pull
  HG_TXNID=TXN:$ID$
  HG_TXNNAME=pull
  file:/*/$TESTTMP/repo (glob)
  HG_URL=file:$TESTTMP/repo
  
  remote: changegroup2
  adding changesets
  adding manifests
  adding file changes
  pretxnchangegroup hook: HG_HOOKNAME=pretxnchangegroup
  HG_HOOKTYPE=pretxnchangegroup
  HG_NODE=f838bfaca5c7226600ebcfd84f3c3c13a28d3757
  HG_NODE_LAST=f838bfaca5c7226600ebcfd84f3c3c13a28d3757
  HG_PENDING=$TESTTMP/clone
  HG_PHASES_MOVED=1
  HG_SOURCE=pull
  HG_TXNID=TXN:$ID$
  HG_TXNNAME=pull
  file:/*/$TESTTMP/repo (glob)
  HG_URL=file:$TESTTMP/repo
  
  added 2 changesets with 2 changes to 2 files
  new changesets 27547f69f254:f838bfaca5c7
  changegroup hook: HG_HOOKNAME=changegroup
  HG_HOOKTYPE=changegroup
  HG_NODE=27547f69f25460a52fff66ad004e58da7ad3fb56
  HG_NODE_LAST=27547f69f25460a52fff66ad004e58da7ad3fb56
  HG_SOURCE=pull
  HG_TXNID=TXN:$ID$
  HG_TXNNAME=pull
  file:/*/$TESTTMP/repo (glob)
  HG_URL=file:$TESTTMP/repo
  
  incoming hook: HG_HOOKNAME=incoming
  HG_HOOKTYPE=incoming
  HG_NODE=27547f69f25460a52fff66ad004e58da7ad3fb56
  HG_SOURCE=pull
  HG_TXNID=TXN:$ID$
  HG_TXNNAME=pull
  file:/*/$TESTTMP/repo (glob)
  HG_URL=file:$TESTTMP/repo
  
  changegroup hook: HG_HOOKNAME=changegroup
  HG_HOOKTYPE=changegroup
  HG_NODE=f838bfaca5c7226600ebcfd84f3c3c13a28d3757
  HG_NODE_LAST=f838bfaca5c7226600ebcfd84f3c3c13a28d3757
  HG_PHASES_MOVED=1
  HG_SOURCE=pull
  HG_TXNID=TXN:$ID$
  HG_TXNNAME=pull
  file:/*/$TESTTMP/repo (glob)
  HG_URL=file:$TESTTMP/repo
  
  incoming hook: HG_HOOKNAME=incoming
  HG_HOOKTYPE=incoming
  HG_NODE=f838bfaca5c7226600ebcfd84f3c3c13a28d3757
  HG_PHASES_MOVED=1
  HG_SOURCE=pull
  HG_TXNID=TXN:$ID$
  HG_TXNNAME=pull
  file:/*/$TESTTMP/repo (glob)
  HG_URL=file:$TESTTMP/repo
  
  pullop.cgresult is 1
  (run 'hg update' to get a working copy)
  $ hg update
  2 files updated, 0 files merged, 0 files removed, 0 files unresolved
  $ hg log -G
  @  2:f838bfaca5c7 public test  C
  |
  o  1:27547f69f254 public test  B
  |
  o  0:4a2df7238c3b public test  A
  
Add more changesets with multiple heads to the original repository

  $ cd ../repo
  $ echo D > D
  $ hg commit -A -m D -q
  $ hg up -r 1
  0 files updated, 0 files merged, 2 files removed, 0 files unresolved
  $ echo E > E
  $ hg commit -A -m E -q
  $ echo F > F
  $ hg commit -A -m F -q
  $ hg up -r 1
  0 files updated, 0 files merged, 2 files removed, 0 files unresolved
  $ echo G > G
  $ hg commit -A -m G -q
  $ hg up -r 3
  2 files updated, 0 files merged, 1 files removed, 0 files unresolved
  $ echo H > H
  $ hg commit -A -m H -q
  $ hg log -G
  @  7:5cd59d311f65 draft test  H
  |
  | o  6:1d14c3ce6ac0 draft test  G
  | |
  | | o  5:7f219660301f draft test  F
  | | |
  | | o  4:8a5212ebc852 draft test  E
  | |/
  o |  3:b3325c91a4d9 draft test  D
  | |
  o |  2:f838bfaca5c7 draft test  C
  |/
  o  1:27547f69f254 draft test  B
  |
  o  0:4a2df7238c3b draft test  A
  
New heads are reported during transfer and properly accounted for in
pullop.cgresult

  $ cd ../clone
  $ hg pull
  pulling from $TESTTMP/repo
  searching for changes
  remote: changegroup1
  adding changesets
  adding manifests
  adding file changes
  pretxnchangegroup hook: HG_HOOKNAME=pretxnchangegroup
  HG_HOOKTYPE=pretxnchangegroup
  HG_NODE=b3325c91a4d916bcc4cdc83ea3fe4ece46a42f6e
  HG_NODE_LAST=8a5212ebc8527f9fb821601504794e3eb11a1ed3
  HG_PENDING=$TESTTMP/clone
  HG_SOURCE=pull
  HG_TXNID=TXN:$ID$
  HG_TXNNAME=pull
  file:/*/$TESTTMP/repo (glob)
  HG_URL=file:$TESTTMP/repo
  
  remote: changegroup2
  adding changesets
  adding manifests
  adding file changes
  pretxnchangegroup hook: HG_HOOKNAME=pretxnchangegroup
  HG_HOOKTYPE=pretxnchangegroup
  HG_NODE=7f219660301fe4c8a116f714df5e769695cc2b46
  HG_NODE_LAST=5cd59d311f6508b8e0ed28a266756c859419c9f1
  HG_PENDING=$TESTTMP/clone
  HG_PHASES_MOVED=1
  HG_SOURCE=pull
  HG_TXNID=TXN:$ID$
  HG_TXNNAME=pull
  file:/*/$TESTTMP/repo (glob)
  HG_URL=file:$TESTTMP/repo
  
  added 5 changesets with 5 changes to 5 files (+2 heads)
  new changesets b3325c91a4d9:5cd59d311f65
  changegroup hook: HG_HOOKNAME=changegroup
  HG_HOOKTYPE=changegroup
  HG_NODE=b3325c91a4d916bcc4cdc83ea3fe4ece46a42f6e
  HG_NODE_LAST=8a5212ebc8527f9fb821601504794e3eb11a1ed3
  HG_SOURCE=pull
  HG_TXNID=TXN:$ID$
  HG_TXNNAME=pull
  file:/*/$TESTTMP/repo (glob)
  HG_URL=file:$TESTTMP/repo
  
  incoming hook: HG_HOOKNAME=incoming
  HG_HOOKTYPE=incoming
  HG_NODE=b3325c91a4d916bcc4cdc83ea3fe4ece46a42f6e
  HG_SOURCE=pull
  HG_TXNID=TXN:$ID$
  HG_TXNNAME=pull
  file:/*/$TESTTMP/repo (glob)
  HG_URL=file:$TESTTMP/repo
  
  incoming hook: HG_HOOKNAME=incoming
  HG_HOOKTYPE=incoming
  HG_NODE=8a5212ebc8527f9fb821601504794e3eb11a1ed3
  HG_SOURCE=pull
  HG_TXNID=TXN:$ID$
  HG_TXNNAME=pull
  file:/*/$TESTTMP/repo (glob)
  HG_URL=file:$TESTTMP/repo
  
  changegroup hook: HG_HOOKNAME=changegroup
  HG_HOOKTYPE=changegroup
  HG_NODE=7f219660301fe4c8a116f714df5e769695cc2b46
  HG_NODE_LAST=5cd59d311f6508b8e0ed28a266756c859419c9f1
  HG_PHASES_MOVED=1
  HG_SOURCE=pull
  HG_TXNID=TXN:$ID$
  HG_TXNNAME=pull
  file:/*/$TESTTMP/repo (glob)
  HG_URL=file:$TESTTMP/repo
  
  incoming hook: HG_HOOKNAME=incoming
  HG_HOOKTYPE=incoming
  HG_NODE=7f219660301fe4c8a116f714df5e769695cc2b46
  HG_PHASES_MOVED=1
  HG_SOURCE=pull
  HG_TXNID=TXN:$ID$
  HG_TXNNAME=pull
  file:/*/$TESTTMP/repo (glob)
  HG_URL=file:$TESTTMP/repo
  
  incoming hook: HG_HOOKNAME=incoming
  HG_HOOKTYPE=incoming
  HG_NODE=1d14c3ce6ac0582d2809220d33e8cd7a696e0156
  HG_PHASES_MOVED=1
  HG_SOURCE=pull
  HG_TXNID=TXN:$ID$
  HG_TXNNAME=pull
  file:/*/$TESTTMP/repo (glob)
  HG_URL=file:$TESTTMP/repo
  
  incoming hook: HG_HOOKNAME=incoming
  HG_HOOKTYPE=incoming
  HG_NODE=5cd59d311f6508b8e0ed28a266756c859419c9f1
  HG_PHASES_MOVED=1
  HG_SOURCE=pull
  HG_TXNID=TXN:$ID$
  HG_TXNNAME=pull
  file:/*/$TESTTMP/repo (glob)
  HG_URL=file:$TESTTMP/repo
  
  pullop.cgresult is 3
  (run 'hg heads' to see heads, 'hg merge' to merge)
  $ hg log -G
  o  7:5cd59d311f65 public test  H
  |
  | o  6:1d14c3ce6ac0 public test  G
  | |
  | | o  5:7f219660301f public test  F
  | | |
  | | o  4:8a5212ebc852 public test  E
  | |/
  o |  3:b3325c91a4d9 public test  D
  | |
  @ |  2:f838bfaca5c7 public test  C
  |/
  o  1:27547f69f254 public test  B
  |
  o  0:4a2df7238c3b public test  A
  
Removing a head from the original repository by merging it

  $ cd ../repo
  $ hg merge -r 6 -q
  $ hg commit -m Merge
  $ echo I > I
  $ hg commit -A -m H -q
  $ hg log -G
  @  9:9d18e5bd9ab0 draft test  H
  |
  o    8:71bd7b46de72 draft test  Merge
  |\
  | o  7:5cd59d311f65 draft test  H
  | |
  o |  6:1d14c3ce6ac0 draft test  G
  | |
  | | o  5:7f219660301f draft test  F
  | | |
  +---o  4:8a5212ebc852 draft test  E
  | |
  | o  3:b3325c91a4d9 draft test  D
  | |
  | o  2:f838bfaca5c7 draft test  C
  |/
  o  1:27547f69f254 draft test  B
  |
  o  0:4a2df7238c3b draft test  A
  
Removed heads are reported during transfer and properly accounted for in
pullop.cgresult

  $ cd ../clone
  $ hg pull
  pulling from $TESTTMP/repo
  searching for changes
  remote: changegroup1
  adding changesets
  adding manifests
  adding file changes
  pretxnchangegroup hook: HG_HOOKNAME=pretxnchangegroup
  HG_HOOKTYPE=pretxnchangegroup
  HG_NODE=71bd7b46de72e69a32455bf88d04757d542e6cf4
  HG_NODE_LAST=71bd7b46de72e69a32455bf88d04757d542e6cf4
  HG_PENDING=$TESTTMP/clone
  HG_SOURCE=pull
  HG_TXNID=TXN:$ID$
  HG_TXNNAME=pull
  file:/*/$TESTTMP/repo (glob)
  HG_URL=file:$TESTTMP/repo
  
  remote: changegroup2
  adding changesets
  adding manifests
  adding file changes
  pretxnchangegroup hook: HG_HOOKNAME=pretxnchangegroup
  HG_HOOKTYPE=pretxnchangegroup
  HG_NODE=9d18e5bd9ab09337802595d49f1dad0c98df4d84
  HG_NODE_LAST=9d18e5bd9ab09337802595d49f1dad0c98df4d84
  HG_PENDING=$TESTTMP/clone
  HG_PHASES_MOVED=1
  HG_SOURCE=pull
  HG_TXNID=TXN:$ID$
  HG_TXNNAME=pull
  file:/*/$TESTTMP/repo (glob)
  HG_URL=file:$TESTTMP/repo
  
  added 2 changesets with 1 changes to 1 files (-1 heads)
  new changesets 71bd7b46de72:9d18e5bd9ab0
  changegroup hook: HG_HOOKNAME=changegroup
  HG_HOOKTYPE=changegroup
  HG_NODE=71bd7b46de72e69a32455bf88d04757d542e6cf4
  HG_NODE_LAST=71bd7b46de72e69a32455bf88d04757d542e6cf4
  HG_SOURCE=pull
  HG_TXNID=TXN:$ID$
  HG_TXNNAME=pull
  file:/*/$TESTTMP/repo (glob)
  HG_URL=file:$TESTTMP/repo
  
  incoming hook: HG_HOOKNAME=incoming
  HG_HOOKTYPE=incoming
  HG_NODE=71bd7b46de72e69a32455bf88d04757d542e6cf4
  HG_SOURCE=pull
  HG_TXNID=TXN:$ID$
  HG_TXNNAME=pull
  file:/*/$TESTTMP/repo (glob)
  HG_URL=file:$TESTTMP/repo
  
  changegroup hook: HG_HOOKNAME=changegroup
  HG_HOOKTYPE=changegroup
  HG_NODE=9d18e5bd9ab09337802595d49f1dad0c98df4d84
  HG_NODE_LAST=9d18e5bd9ab09337802595d49f1dad0c98df4d84
  HG_PHASES_MOVED=1
  HG_SOURCE=pull
  HG_TXNID=TXN:$ID$
  HG_TXNNAME=pull
  file:/*/$TESTTMP/repo (glob)
  HG_URL=file:$TESTTMP/repo
  
  incoming hook: HG_HOOKNAME=incoming
  HG_HOOKTYPE=incoming
  HG_NODE=9d18e5bd9ab09337802595d49f1dad0c98df4d84
  HG_PHASES_MOVED=1
  HG_SOURCE=pull
  HG_TXNID=TXN:$ID$
  HG_TXNNAME=pull
  file:/*/$TESTTMP/repo (glob)
  HG_URL=file:$TESTTMP/repo
  
  pullop.cgresult is -2
  (run 'hg update' to get a working copy)
  $ hg log -G
  o  9:9d18e5bd9ab0 public test  H
  |
  o    8:71bd7b46de72 public test  Merge
  |\
  | o  7:5cd59d311f65 public test  H
  | |
  o |  6:1d14c3ce6ac0 public test  G
  | |
  | | o  5:7f219660301f public test  F
  | | |
  +---o  4:8a5212ebc852 public test  E
  | |
  | o  3:b3325c91a4d9 public test  D
  | |
  | @  2:f838bfaca5c7 public test  C
  |/
  o  1:27547f69f254 public test  B
  |
  o  0:4a2df7238c3b public test  A
  

Test that "split" from inline content works fine (cf issue6811)
===============================================================

setup
-----

(create a compression free version where the split is easier to trigger)

  $ cat >> $HGRCPATH << EOF
  > [format]
  > revlog-compression=none
  > use-persistent-nodemap=no
  > EOF

  $ cd ..
  $ hg clone --pull repo repo-test-split --quiet
  pullop.cgresult is 2
  $ cat > repo-test-split/.hg/hgrc << EOF
  > [extensions]
  > bundle2=$TESTTMP/bundle2.py
  > EOF
  $ hg clone --pull repo-test-split clone-test-split --quiet
  pullop.cgresult is 2
  $ cd repo-test-split
  $ cat > .hg/hgrc << EOF
  > [extensions]
  > bundle2=$TESTTMP/bundle2.py
  > EOF


IMPORTANT: must be a non-split revlog with only a .i
  $ ls -1 .hg/store/00manifest.*
  .hg/store/00manifest.i
  $ ls -1 .hg/store/data/_a.*
  .hg/store/data/_a.i

  $ $TESTDIR/seq.py 100000 > A
  $ mkdir foo
  $ cd foo
  $ touch `$TESTDIR/seq.py 10000`
  $ cd ..
  $ hg add -q foo
  $ hg commit -m 'split the manifest and one filelog'


IMPORTANT: now the revlogs must be split
  $ ls -1 .hg/store/00manifest.*
  .hg/store/00manifest.d
  .hg/store/00manifest.i
  $ ls -1 .hg/store/data/_a.*
  .hg/store/data/_a.d
  .hg/store/data/_a.i

Add an extra commit on top of that

  $ echo foo >> A
  $ hg commit -m 'one extra commit'


Actual testing
--------------

  $ cd ../clone-test-split

We now pull this in the clone the split should be in one changegroup, the update in another one

  $ hg  pull
  pulling from $TESTTMP/repo-test-split
  searching for changes
  remote: changegroup1
  adding changesets
  adding manifests
  adding file changes
  remote: changegroup2
  adding changesets
  adding manifests
  adding file changes
  added 2 changesets with 10002 changes to 10002 files
  new changesets 5e5bf9c91085:900b170f70d0
  pullop.cgresult is 1
  (run 'hg update' to get a working copy)

IMPORTANT: now the revlogs must be split
  $ ls -1 .hg/store/00manifest.*
  .hg/store/00manifest.d
  .hg/store/00manifest.i
  $ ls -1 .hg/store/data/_a.*
  .hg/store/data/_a.d
  .hg/store/data/_a.i

manifest should work

  $ hg  files -r tip | wc -l
  \s*10007 (re)

file content should work

  $ hg  cat -r tip A | wc -l
  \s*100001 (re)
