  $ "$TESTDIR/hghave" symlink || exit 80

  $ hg init

Copy:
  $ hg import -d "1000000 0" -mcopy - <<EOF
  3:37bacb7ca14d
  $ if "$TESTDIR/hghave" -q execbit; then
  >     test -f copy -a ! -x copy || echo bad
  >     test -x copyx || echo bad
  > else
  >     test -f copy || echo bad
  > fi
  4:47b81a94361d
  5:d9b001d98336
  6:ebe901e7576b
  7:18f368958ecd
  8:c32b0d7e6f44
  9:034a6bf95330
  9 rename2 rename3 rename3-2 / rename3 (rename2)rename3-2 (rename2)
  11:c39bce63e786
  12:30b530085242
  13:04750ef42fb3
  14:c4cd9cdeaa74
  repository tip rolled back to revision 15 (undo import)
  working directory now based on revision 15