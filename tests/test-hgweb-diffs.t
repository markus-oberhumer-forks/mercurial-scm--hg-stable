  $ "$TESTDIR/hghave" serve execbit || exit 80
  $ chmod +x a
  $ hg rm b
  $ hg ci -Amb
  $ "$TESTDIR/get-with-headers.py" localhost:$HGPORT '/rev/0'
  $ "$TESTDIR/get-with-headers.py" localhost:$HGPORT '/raw-rev/0'
  $ "$TESTDIR/get-with-headers.py" localhost:$HGPORT '/diff/tip/b'
  $ "$TESTDIR/get-with-headers.py" localhost:$HGPORT '/rev/0'
  $ "$TESTDIR/get-with-headers.py" localhost:$HGPORT '/raw-rev/0'