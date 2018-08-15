#require repofncache

Init repo1:

  $ hg init repo1
  $ cd repo1
  $ echo "some text" > a
  $ hg add
  adding a
  $ hg ci -m first
  $ cat .hg/store/fncache | sort
  data/a.i

Testing a.i/b:

  $ mkdir a.i
  $ echo "some other text" > a.i/b
  $ hg add
  adding a.i/b
  $ hg ci -m second
  $ cat .hg/store/fncache | sort
  data/a.i
  data/a.i.hg/b.i

Testing a.i.hg/c:

  $ mkdir a.i.hg
  $ echo "yet another text" > a.i.hg/c
  $ hg add
  adding a.i.hg/c
  $ hg ci -m third
  $ cat .hg/store/fncache | sort
  data/a.i
  data/a.i.hg.hg/c.i
  data/a.i.hg/b.i

Testing verify:

  $ hg verify
  checking changesets
  checking manifests
  crosschecking files in changesets and manifests
  checking files
  3 files, 3 changesets, 3 total revisions

  $ rm .hg/store/fncache

  $ hg verify
  checking changesets
  checking manifests
  crosschecking files in changesets and manifests
  checking files
   warning: revlog 'data/a.i' not in fncache!
   warning: revlog 'data/a.i.hg/c.i' not in fncache!
   warning: revlog 'data/a.i/b.i' not in fncache!
  3 files, 3 changesets, 3 total revisions
  3 warnings encountered!
  hint: run "hg debugrebuildfncache" to recover from corrupt fncache

Follow the hint to make sure it works

  $ hg debugrebuildfncache
  adding data/a.i
  adding data/a.i.hg/c.i
  adding data/a.i/b.i
  3 items added, 0 removed from fncache

  $ hg verify
  checking changesets
  checking manifests
  crosschecking files in changesets and manifests
  checking files
  3 files, 3 changesets, 3 total revisions

  $ cd ..

Non store repo:

  $ hg --config format.usestore=False init foo
  $ cd foo
  $ mkdir tst.d
  $ echo foo > tst.d/foo
  $ hg ci -Amfoo
  adding tst.d/foo
  $ find .hg | sort
  .hg
  .hg/00changelog.i
  .hg/00manifest.i
  .hg/cache
  .hg/cache/branch2-served
  .hg/cache/manifestfulltextcache (reporevlogstore !)
  .hg/cache/rbc-names-v1
  .hg/cache/rbc-revs-v1
  .hg/data
  .hg/data/tst.d.hg
  .hg/data/tst.d.hg/foo.i
  .hg/dirstate
  .hg/fsmonitor.state (fsmonitor !)
  .hg/last-message.txt
  .hg/phaseroots
  .hg/requires
  .hg/undo
  .hg/undo.backup.dirstate
  .hg/undo.backupfiles
  .hg/undo.bookmarks
  .hg/undo.branch
  .hg/undo.desc
  .hg/undo.dirstate
  .hg/undo.phaseroots
  $ cd ..

Non fncache repo:

  $ hg --config format.usefncache=False init bar
  $ cd bar
  $ mkdir tst.d
  $ echo foo > tst.d/Foo
  $ hg ci -Amfoo
  adding tst.d/Foo
  $ find .hg | sort
  .hg
  .hg/00changelog.i
  .hg/cache
  .hg/cache/branch2-served
  .hg/cache/manifestfulltextcache (reporevlogstore !)
  .hg/cache/rbc-names-v1
  .hg/cache/rbc-revs-v1
  .hg/dirstate
  .hg/fsmonitor.state (fsmonitor !)
  .hg/last-message.txt
  .hg/requires
  .hg/store
  .hg/store/00changelog.i
  .hg/store/00manifest.i
  .hg/store/data
  .hg/store/data/tst.d.hg
  .hg/store/data/tst.d.hg/_foo.i
  .hg/store/phaseroots
  .hg/store/undo
  .hg/store/undo.backupfiles
  .hg/store/undo.phaseroots
  .hg/undo.backup.dirstate
  .hg/undo.bookmarks
  .hg/undo.branch
  .hg/undo.desc
  .hg/undo.dirstate
  $ cd ..

Encoding of reserved / long paths in the store

  $ hg init r2
  $ cd r2
  $ cat <<EOF > .hg/hgrc
  > [ui]
  > portablefilenames = ignore
  > EOF

  $ hg import -q --bypass - <<EOF
  > # HG changeset patch
  > # User test
  > # Date 0 0
  > # Node ID 1c7a2f7cb77be1a0def34e4c7cabc562ad98fbd7
  > # Parent  0000000000000000000000000000000000000000
  > 1
  > 
  > diff --git a/12345678/12345678/12345678/12345678/12345678/12345678/12345678/12345/xxxxxxxxx-xxxxxxxxx-xxxxxxxxx-123456789-12.3456789-12345-ABCDEFGHIJKLMNOPRSTUVWXYZ-abcdefghjiklmnopqrstuvwxyz b/12345678/12345678/12345678/12345678/12345678/12345678/12345678/12345/xxxxxxxxx-xxxxxxxxx-xxxxxxxxx-123456789-12.3456789-12345-ABCDEFGHIJKLMNOPRSTUVWXYZ-abcdefghjiklmnopqrstuvwxyz
  > new file mode 100644
  > --- /dev/null
  > +++ b/12345678/12345678/12345678/12345678/12345678/12345678/12345678/12345/xxxxxxxxx-xxxxxxxxx-xxxxxxxxx-123456789-12.3456789-12345-ABCDEFGHIJKLMNOPRSTUVWXYZ-abcdefghjiklmnopqrstuvwxyz
  > @@ -0,0 +1,1 @@
  > +foo
  > diff --git a/AUX/SECOND/X.PRN/FOURTH/FI:FTH/SIXTH/SEVENTH/EIGHTH/NINETH/TENTH/ELEVENTH/LOREMIPSUM.TXT b/AUX/SECOND/X.PRN/FOURTH/FI:FTH/SIXTH/SEVENTH/EIGHTH/NINETH/TENTH/ELEVENTH/LOREMIPSUM.TXT
  > new file mode 100644
  > --- /dev/null
  > +++ b/AUX/SECOND/X.PRN/FOURTH/FI:FTH/SIXTH/SEVENTH/EIGHTH/NINETH/TENTH/ELEVENTH/LOREMIPSUM.TXT
  > @@ -0,0 +1,1 @@
  > +foo
  > diff --git a/Project Planning/Resources/AnotherLongDirectoryName/Followedbyanother/AndAnother/AndThenAnExtremelyLongFileName.txt b/Project Planning/Resources/AnotherLongDirectoryName/Followedbyanother/AndAnother/AndThenAnExtremelyLongFileName.txt
  > new file mode 100644
  > --- /dev/null
  > +++ b/Project Planning/Resources/AnotherLongDirectoryName/Followedbyanother/AndAnother/AndThenAnExtremelyLongFileName.txt	
  > @@ -0,0 +1,1 @@
  > +foo
  > diff --git a/bla.aux/prn/PRN/lpt/com3/nul/coma/foo.NUL/normal.c b/bla.aux/prn/PRN/lpt/com3/nul/coma/foo.NUL/normal.c
  > new file mode 100644
  > --- /dev/null
  > +++ b/bla.aux/prn/PRN/lpt/com3/nul/coma/foo.NUL/normal.c
  > @@ -0,0 +1,1 @@
  > +foo
  > diff --git a/enterprise/openesbaddons/contrib-imola/corba-bc/netbeansplugin/wsdlExtension/src/main/java/META-INF/services/org.netbeans.modules.xml.wsdl.bindingsupport.spi.ExtensibilityElementTemplateProvider b/enterprise/openesbaddons/contrib-imola/corba-bc/netbeansplugin/wsdlExtension/src/main/java/META-INF/services/org.netbeans.modules.xml.wsdl.bindingsupport.spi.ExtensibilityElementTemplateProvider
  > new file mode 100644
  > --- /dev/null
  > +++ b/enterprise/openesbaddons/contrib-imola/corba-bc/netbeansplugin/wsdlExtension/src/main/java/META-INF/services/org.netbeans.modules.xml.wsdl.bindingsupport.spi.ExtensibilityElementTemplateProvider
  > @@ -0,0 +1,1 @@
  > +foo
  > EOF

  $ find .hg/store -name *.i  | sort
  .hg/store/00changelog.i
  .hg/store/00manifest.i
  .hg/store/data/bla.aux/pr~6e/_p_r_n/lpt/co~6d3/nu~6c/coma/foo._n_u_l/normal.c.i
  .hg/store/dh/12345678/12345678/12345678/12345678/12345678/12345678/12345678/12345/xxxxxx168e07b38e65eff86ab579afaaa8e30bfbe0f35f.i
  .hg/store/dh/au~78/second/x.prn/fourth/fi~3afth/sixth/seventh/eighth/nineth/tenth/loremia20419e358ddff1bf8751e38288aff1d7c32ec05.i
  .hg/store/dh/enterpri/openesba/contrib-/corba-bc/netbeans/wsdlexte/src/main/java/org.net7018f27961fdf338a598a40c4683429e7ffb9743.i
  .hg/store/dh/project_/resource/anotherl/followed/andanoth/andthenanextremelylongfilename0d8e1f4187c650e2f1fdca9fd90f786bc0976b6b.i

  $ cd ..

Aborting lock does not prevent fncache writes

  $ cat > exceptionext.py <<EOF
  > from __future__ import absolute_import
  > import os
  > from mercurial import commands, error, extensions
  > 
  > def lockexception(orig, vfs, lockname, wait, releasefn, *args, **kwargs):
  >     def releasewrap():
  >         l.held = False # ensure __del__ is a noop
  >         raise error.Abort("forced lock failure")
  >     l = orig(vfs, lockname, wait, releasewrap, *args, **kwargs)
  >     return l
  > 
  > def reposetup(ui, repo):
  >     extensions.wrapfunction(repo, '_lock', lockexception)
  > 
  > cmdtable = {}
  > 
  > # wrap "commit" command to prevent wlock from being '__del__()'-ed
  > # at the end of dispatching (for intentional "forced lcok failure")
  > def commitwrap(orig, ui, repo, *pats, **opts):
  >     repo = repo.unfiltered() # to use replaced repo._lock certainly
  >     wlock = repo.wlock()
  >     try:
  >         return orig(ui, repo, *pats, **opts)
  >     finally:
  >         # multiple 'relase()' is needed for complete releasing wlock,
  >         # because "forced" abort at last releasing store lock
  >         # prevents wlock from being released at same 'lockmod.release()'
  >         for i in range(wlock.held):
  >             wlock.release()
  > 
  > def extsetup(ui):
  >     extensions.wrapcommand(commands.table, b"commit", commitwrap)
  > EOF
  $ extpath=`pwd`/exceptionext.py
  $ hg init fncachetxn
  $ cd fncachetxn
  $ printf "[extensions]\nexceptionext=$extpath\n" >> .hg/hgrc
  $ touch y
  $ hg ci -qAm y
  abort: forced lock failure
  [255]
  $ cat .hg/store/fncache
  data/y.i

Aborting transaction prevents fncache change

  $ cat > ../exceptionext.py <<EOF
  > from __future__ import absolute_import
  > import os
  > from mercurial import commands, error, extensions, localrepo
  > 
  > def wrapper(orig, self, *args, **kwargs):
  >     tr = orig(self, *args, **kwargs)
  >     def fail(tr):
  >         raise error.Abort(b"forced transaction failure")
  >     # zzz prefix to ensure it sorted after store.write
  >     tr.addfinalize(b'zzz-forcefails', fail)
  >     return tr
  > 
  > def uisetup(ui):
  >     extensions.wrapfunction(
  >         localrepo.localrepository, b'transaction', wrapper)
  > 
  > cmdtable = {}
  > 
  > EOF

Clean cached version
  $ rm -f "${extpath}c"
  $ rm -Rf "`dirname $extpath`/__pycache__"

  $ touch z
  $ hg ci -qAm z
  transaction abort!
  rollback completed
  abort: forced transaction failure
  [255]
  $ cat .hg/store/fncache
  data/y.i

Aborted transactions can be recovered later

  $ cat > ../exceptionext.py <<EOF
  > from __future__ import absolute_import
  > import os
  > from mercurial import (
  >   commands,
  >   error,
  >   extensions,
  >   localrepo,
  >   transaction,
  > )
  > 
  > def trwrapper(orig, self, *args, **kwargs):
  >     tr = orig(self, *args, **kwargs)
  >     def fail(tr):
  >         raise error.Abort("forced transaction failure")
  >     # zzz prefix to ensure it sorted after store.write
  >     tr.addfinalize('zzz-forcefails', fail)
  >     return tr
  > 
  > def abortwrapper(orig, self, *args, **kwargs):
  >     raise error.Abort("forced transaction failure")
  > 
  > def uisetup(ui):
  >     extensions.wrapfunction(localrepo.localrepository, 'transaction',
  >                             trwrapper)
  >     extensions.wrapfunction(transaction.transaction, '_abort',
  >                             abortwrapper)
  > 
  > cmdtable = {}
  > 
  > EOF

Clean cached versions
  $ rm -f "${extpath}c"
  $ rm -Rf "`dirname $extpath`/__pycache__"

  $ hg up -q 1
  $ touch z
  $ hg ci -qAm z 2>/dev/null
  [255]
  $ cat .hg/store/fncache | sort
  data/y.i
  data/z.i
  $ hg recover
  rolling back interrupted transaction
  checking changesets
  checking manifests
  crosschecking files in changesets and manifests
  checking files
  1 files, 1 changesets, 1 total revisions
  $ cat .hg/store/fncache
  data/y.i

  $ cd ..

debugrebuildfncache does nothing unless repo has fncache requirement

  $ hg --config format.usefncache=false init nofncache
  $ cd nofncache
  $ hg debugrebuildfncache
  (not rebuilding fncache because repository does not support fncache)

  $ cd ..

debugrebuildfncache works on empty repository

  $ hg init empty
  $ cd empty
  $ hg debugrebuildfncache
  fncache already up to date
  $ cd ..

debugrebuildfncache on an up to date repository no-ops

  $ hg init repo
  $ cd repo
  $ echo initial > foo
  $ echo initial > .bar
  $ hg commit -A -m initial
  adding .bar
  adding foo

  $ cat .hg/store/fncache | sort
  data/.bar.i
  data/foo.i

  $ hg debugrebuildfncache
  fncache already up to date

debugrebuildfncache restores deleted fncache file

  $ rm -f .hg/store/fncache
  $ hg debugrebuildfncache
  adding data/.bar.i
  adding data/foo.i
  2 items added, 0 removed from fncache

  $ cat .hg/store/fncache | sort
  data/.bar.i
  data/foo.i

Rebuild after rebuild should no-op

  $ hg debugrebuildfncache
  fncache already up to date

A single missing file should get restored, an extra file should be removed

  $ cat > .hg/store/fncache << EOF
  > data/foo.i
  > data/bad-entry.i
  > EOF

  $ hg debugrebuildfncache
  removing data/bad-entry.i
  adding data/.bar.i
  1 items added, 1 removed from fncache

  $ cat .hg/store/fncache | sort
  data/.bar.i
  data/foo.i

  $ cd ..

Try a simple variation without dotencode to ensure fncache is ignorant of encoding

  $ hg --config format.dotencode=false init nodotencode
  $ cd nodotencode
  $ echo initial > foo
  $ echo initial > .bar
  $ hg commit -A -m initial
  adding .bar
  adding foo

  $ cat .hg/store/fncache | sort
  data/.bar.i
  data/foo.i

  $ rm .hg/store/fncache
  $ hg debugrebuildfncache
  adding data/.bar.i
  adding data/foo.i
  2 items added, 0 removed from fncache

  $ cat .hg/store/fncache | sort
  data/.bar.i
  data/foo.i

  $ cd ..

In repositories that have accumulated a large number of files over time, the
fncache file is going to be large. If we possibly can avoid loading it, so much the better.
The cache should not loaded when committing changes to existing files, or when unbundling
changesets that only contain changes to existing files:

  $ cat > fncacheloadwarn.py << EOF
  > from __future__ import absolute_import
  > from mercurial import extensions, store
  > 
  > def extsetup(ui):
  >     def wrapstore(orig, requirements, *args):
  >         store = orig(requirements, *args)
  >         if 'store' in requirements and 'fncache' in requirements:
  >             instrumentfncachestore(store, ui)
  >         return store
  >     extensions.wrapfunction(store, 'store', wrapstore)
  > 
  > def instrumentfncachestore(fncachestore, ui):
  >     class instrumentedfncache(type(fncachestore.fncache)):
  >         def _load(self):
  >             ui.warn('fncache load triggered!\n')
  >             super(instrumentedfncache, self)._load()
  >     fncachestore.fncache.__class__ = instrumentedfncache
  > EOF

  $ fncachextpath=`pwd`/fncacheloadwarn.py
  $ hg init nofncacheload
  $ cd nofncacheload
  $ printf "[extensions]\nfncacheloadwarn=$fncachextpath\n" >> .hg/hgrc

A new file should trigger a load, as we'd want to update the fncache set in that case:

  $ touch foo
  $ hg ci -qAm foo
  fncache load triggered!

But modifying that file should not:

  $ echo bar >> foo
  $ hg ci -qm foo

If a transaction has been aborted, the zero-size truncated index file will
not prevent the fncache from being loaded; rather than actually abort
a transaction, we simulate the situation by creating a zero-size index file:

  $ touch .hg/store/data/bar.i
  $ touch bar
  $ hg ci -qAm bar
  fncache load triggered!

Unbundling should follow the same rules; existing files should not cause a load:

  $ hg clone -q . tobundle
  $ echo 'new line' > tobundle/bar
  $ hg -R tobundle ci -qm bar
  $ hg -R tobundle bundle -q barupdated.hg
  $ hg unbundle -q barupdated.hg

but adding new files should:

  $ touch tobundle/newfile
  $ hg -R tobundle ci -qAm newfile
  $ hg -R tobundle bundle -q newfile.hg
  $ hg unbundle -q newfile.hg
  fncache load triggered!

  $ cd ..
