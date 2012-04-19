qimport null revision

  $ hg qimport -r null
  abort: revision -1 is not mutable
  (see "hg help phases" for details)
  [255]
  $ hg qseries

  $ cat > appendfoo.diff <<EOF
  > append foo
  >  
  > diff -r 07f494440405 -r 261500830e46 baz
  > --- /dev/null	Thu Jan 01 00:00:00 1970 +0000
  > +++ b/baz	Thu Jan 01 00:00:00 1970 +0000
  > @@ -0,0 +1,1 @@
  > +foo
  > EOF

  $ cat > appendbar.diff <<EOF
  > append bar
  >  
  > diff -r 07f494440405 -r 261500830e46 baz
  > --- a/baz	Thu Jan 01 00:00:00 1970 +0000
  > +++ b/baz	Thu Jan 01 00:00:00 1970 +0000
  > @@ -1,1 +1,2 @@
  >  foo
  > +bar
  > EOF

  $ hg qimport --push appendfoo.diff appendbar.diff
  adding appendfoo.diff to series file
  adding appendbar.diff to series file
  applying appendfoo.diff
  applying appendbar.diff
  now at: appendbar.diff
  $ hg qimport -r 'p1(.)::' -P
  popping 3.diff
  $ hg qdel 3.diff