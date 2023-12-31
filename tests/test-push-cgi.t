#require no-msys # MSYS will translate web paths as if they were file paths

This is a test of the push wire protocol over CGI-based hgweb.

initialize repository

  $ hg init r
  $ cd r
  $ echo a > a
  $ hg ci -A -m "0"
  adding a
  $ echo '[web]' > .hg/hgrc
  $ echo 'allow_push = *' >> .hg/hgrc
  $ echo 'push_ssl = false' >> .hg/hgrc

create hgweb invocation script

  $ cat >hgweb.cgi <<HGWEB
  > from mercurial import demandimport; demandimport.enable()
  > from mercurial.hgweb import hgweb
  > from mercurial.hgweb import wsgicgi
  > application = hgweb(b'.', b'test repository')
  > wsgicgi.launch(application)
  > HGWEB
  $ chmod 755 hgweb.cgi

test preparation

  $ . "$TESTDIR/cgienv"
  $ REQUEST_METHOD="POST"; export REQUEST_METHOD
  $ CONTENT_TYPE="application/octet-stream"; export CONTENT_TYPE
  $ hg bundle --type v1 --all bundle.hg
  1 changesets found
  $ CONTENT_LENGTH=279; export CONTENT_LENGTH;

expect failure because heads doesn't match (formerly known as 'unsynced changes')

  $ QUERY_STRING="cmd=unbundle&heads=0000000000000000000000000000000000000000"; export QUERY_STRING
  $ "$PYTHON" hgweb.cgi <bundle.hg >page1 2>&1
  $ cat page1
  Status: 200 Script output follows\r (esc)
  Content-Type: application/mercurial-0.1\r (esc)
  Content-Length: 64\r (esc)
  \r (esc)
  0
  repository changed while preparing changes - please try again

successful force push

  $ QUERY_STRING="cmd=unbundle&heads=666f726365"; export QUERY_STRING
  $ "$PYTHON" hgweb.cgi <bundle.hg >page2 2>&1
  $ cat page2
  Status: 200 Script output follows\r (esc)
  Content-Type: application/mercurial-0.1\r (esc)
  Content-Length: 102\r (esc)
  \r (esc)
  1
  adding changesets
  adding manifests
  adding file changes
  added 0 changesets with 0 changes to 1 files

successful push, list of heads

  $ QUERY_STRING="cmd=unbundle&heads=f7b1eb17ad24730a1651fccd46c43826d1bbc2ac"; export QUERY_STRING
  $ "$PYTHON" hgweb.cgi <bundle.hg >page3 2>&1
  $ cat page3
  Status: 200 Script output follows\r (esc)
  Content-Type: application/mercurial-0.1\r (esc)
  Content-Length: 102\r (esc)
  \r (esc)
  1
  adding changesets
  adding manifests
  adding file changes
  added 0 changesets with 0 changes to 1 files

successful push, SHA1 hash of heads (unbundlehash capability)

  $ QUERY_STRING="cmd=unbundle&heads=686173686564 5a785a5f9e0d433b88ed862b206b011b0c3a9d13"; export QUERY_STRING
  $ "$PYTHON" hgweb.cgi <bundle.hg >page4 2>&1
  $ cat page4
  Status: 200 Script output follows\r (esc)
  Content-Type: application/mercurial-0.1\r (esc)
  Content-Length: 102\r (esc)
  \r (esc)
  1
  adding changesets
  adding manifests
  adding file changes
  added 0 changesets with 0 changes to 1 files

  $ cd ..
