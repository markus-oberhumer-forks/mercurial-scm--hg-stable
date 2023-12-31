#if no-extraextensions
  $ hg debugextensions
#endif

  $ debugpath=`pwd`/extwithoutinfos.py

  $ cat > extwithoutinfos.py <<EOF
  > EOF
  $ cat > extwithinfos.py <<EOF
  > testedwith = b'3.0 3.1 3.2.1'
  > buglink = b'https://example.org/bts'
  > EOF

  $ cat >> $HGRCPATH <<EOF
  > [extensions]
  > histedit=
  > patchbomb=
  > rebase=
  > mq=
  > ext1 = $debugpath
  > ext2 = `pwd`/extwithinfos.py
  > EOF

  $ for extension in $HGTESTEXTRAEXTENSIONS; do
  >     echo "$extension=!" >> $HGRCPATH
  > done

  $ hg debugextensions
  ext1 (untested!)
  ext2 (3.2.1!)
  histedit
  mq
  patchbomb
  rebase

  $ hg debugextensions -v
  ext1
    location: */extwithoutinfos.py* (glob)
    bundled: no
  ext2
    location: */extwithinfos.py* (glob)
    bundled: no
    tested with: 3.0 3.1 3.2.1
    bug reporting: https://example.org/bts
  histedit
    location: */hgext/histedit.py* (glob) (no-pyoxidizer-in-memory !)
    location: */release/app/hg* (glob) (pyoxidizer-in-memory !)
    bundled: yes
  mq
    location: */hgext/mq.py* (glob) (no-pyoxidizer-in-memory !)
    location: */release/app/hg* (glob) (pyoxidizer-in-memory !)
    bundled: yes
  patchbomb
    location: */hgext/patchbomb.py* (glob) (no-pyoxidizer-in-memory !)
    location: */release/app/hg* (glob) (pyoxidizer-in-memory !)
    bundled: yes
  rebase
    location: */hgext/rebase.py* (glob) (no-pyoxidizer-in-memory !)
    location: */release/app/hg* (glob) (pyoxidizer-in-memory !)
    bundled: yes

  $ hg debugextensions -Tjson | sed 's|\\\\|/|g'
  [
   {
    "buglink": "",
    "bundled": false,
    "name": "ext1",
    "source": "*/extwithoutinfos.py*", (glob)
    "testedwith": []
   },
   {
    "buglink": "https://example.org/bts",
    "bundled": false,
    "name": "ext2",
    "source": "*/extwithinfos.py*", (glob)
    "testedwith": ["3.0", "3.1", "3.2.1"]
   },
   {
    "buglink": "",
    "bundled": true,
    "name": "histedit",
    "source": "*/hgext/histedit.py*", (glob) (no-pyoxidizer-in-memory !)
    "source": */release/app/hg* (glob) (pyoxidizer-in-memory !)
    "testedwith": []
   },
   {
    "buglink": "",
    "bundled": true,
    "name": "mq",
    "source": "*/hgext/mq.py*", (glob) (no-pyoxidizer-in-memory !)
    "source": */release/app/hg* (glob) (pyoxidizer-in-memory !)
    "testedwith": []
   },
   {
    "buglink": "",
    "bundled": true,
    "name": "patchbomb",
    "source": "*/hgext/patchbomb.py*", (glob) (no-pyoxidizer-in-memory !)
    "source": */release/app/hg* (glob) (pyoxidizer-in-memory !)
    "testedwith": []
   },
   {
    "buglink": "",
    "bundled": true,
    "name": "rebase",
    "source": "*/hgext/rebase.py*", (glob) (no-pyoxidizer-in-memory !)
    "source": */release/app/hg* (glob) (pyoxidizer-in-memory !)
    "testedwith": []
   }
  ]

  $ hg debugextensions -T '{ifcontains("3.1", testedwith, "{name}\n")}'
  ext2
  $ hg debugextensions \
  > -T '{ifcontains("3.2", testedwith, "no substring match: {name}\n")}'
