
#if symlink
#else
  $ hg import -q --bypass - <<EOF
  > # HG changeset patch
  > link
  > 
  > diff --git a/a/b b/a/b
  > new file mode 120000
  > --- /dev/null
  > +++ b/a/b
  > @@ -0,0 +1,1 @@
  > +c
  > \ No newline at end of file
  > EOF
  $ hg up -q
#endif

  moving a/b to a/b~0ed027b96f31
  use 'hg resolve' to retry unresolved file merges or 'hg merge --abort' to abandon
  use 'hg resolve' to retry unresolved file merges or 'hg merge --abort' to abandon
  use 'hg resolve' to retry unresolved file merges or 'hg merge --abort' to abandon
  use 'hg resolve' to retry unresolved file merges or 'hg merge --abort' to abandon

#if symlink
#else
  $ cat a/b.old
  c (no-eol)
#endif
