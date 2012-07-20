  $ "$TESTDIR/hghave" serve || exit 80
  $ hg import -q --bypass - <<EOF
  > # HG changeset patch
  > # User test
  > # Date 0 0
  > b
  > 
  > diff --git a/a b/a
  > old mode 100644
  > new mode 100755
  > diff --git a/b b/b
  > deleted file mode 100644
  > --- a/b
  > +++ /dev/null
  > @@ -1,1 +0,0 @@
  > -b
  > EOF
  $ "$TESTDIR/get-with-headers.py" localhost:$HGPORT 'rev/0'
   <td class="author"> <a href="/rev/559edbd9ed20">559edbd9ed20</a></td>
  $ "$TESTDIR/get-with-headers.py" localhost:$HGPORT 'raw-rev/0'
  $ "$TESTDIR/get-with-headers.py" localhost:$HGPORT 'diff/tip/b'
  <title>test: b diff</title>
  <li><a href="/shortlog/559edbd9ed20">log</a></li>
  <li><a href="/graph/559edbd9ed20">graph</a></li>
  <li><a href="/rev/559edbd9ed20">changeset</a></li>
  <li><a href="/file/559edbd9ed20">browse</a></li>
  <li><a href="/file/559edbd9ed20/b">file</a></li>
  <li><a href="/file/tip/b">latest</a></li>
  <li><a href="/comparison/559edbd9ed20/b">comparison</a></li>
  <li><a href="/annotate/559edbd9ed20/b">annotate</a></li>
  <li><a href="/log/559edbd9ed20/b">file log</a></li>
  <li><a href="/raw-file/559edbd9ed20/b">raw</a></li>
  <h3>diff b @ 1:559edbd9ed20</h3>
   <td><a href="/file/0cd96de13884/b">0cd96de13884</a> </td>
  <div class="source bottomline parity0"><pre><a href="#l1.1" id="l1.1">     1.1</a> <span class="minusline">--- a/b	Thu Jan 01 00:00:00 1970 +0000
  </span><a href="#l1.2" id="l1.2">     1.2</a> <span class="plusline">+++ /dev/null	Thu Jan 01 00:00:00 1970 +0000
  </span><a href="#l1.3" id="l1.3">     1.3</a> <span class="atline">@@ -1,1 +0,0 @@
  </span><a href="#l1.4" id="l1.4">     1.4</a> <span class="minusline">-b
  $ "$TESTDIR/get-with-headers.py" localhost:$HGPORT 'rev/0'
   <td class="author"> <a href="/rev/559edbd9ed20">559edbd9ed20</a></td>
  $ "$TESTDIR/get-with-headers.py" localhost:$HGPORT 'raw-rev/0'