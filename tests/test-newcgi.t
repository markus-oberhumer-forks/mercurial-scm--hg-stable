#require no-msys # MSYS will translate web paths as if they were file paths

This tests if CGI files from after d0db3462d568 but
before d74fc8dec2b4 still work.

  $ hg init test
  $ cat >hgweb.cgi <<HGWEB
  > #!$PYTHON
  > #
  > # An example CGI script to use hgweb, edit as necessary
  > 
  > from mercurial import demandimport; demandimport.enable()
  > from mercurial.hgweb import hgweb
  > from mercurial.hgweb import wsgicgi
  > from mercurial.hgweb.request import wsgiapplication
  > 
  > def make_web_app():
  >     return hgweb(b"test", b"Empty test repository")
  > 
  > wsgicgi.launch(wsgiapplication(make_web_app))
  > HGWEB

  $ chmod 755 hgweb.cgi

  $ cat >hgweb.config <<HGWEBDIRCONF
  > [paths]
  > test = test
  > HGWEBDIRCONF

  $ cat >hgwebdir.cgi <<HGWEBDIR
  > #!$PYTHON
  > #
  > # An example CGI script to export multiple hgweb repos, edit as necessary
  > 
  > from mercurial import demandimport; demandimport.enable()
  > from mercurial.hgweb import hgwebdir
  > from mercurial.hgweb import wsgicgi
  > from mercurial.hgweb.request import wsgiapplication
  > 
  > def make_web_app():
  >     return hgwebdir(b"hgweb.config")
  > 
  > wsgicgi.launch(wsgiapplication(make_web_app))
  > HGWEBDIR

  $ chmod 755 hgwebdir.cgi

  $ . "$TESTDIR/cgienv"
  $ "$PYTHON" hgweb.cgi > page1
  $ "$PYTHON" hgwebdir.cgi > page2

  $ PATH_INFO="/test/"
  $ PATH_TRANSLATED="/var/something/test.cgi"
  $ REQUEST_URI="/test/test/"
  $ SCRIPT_URI="http://hg.omnifarious.org/test/test/"
  $ SCRIPT_URL="/test/test/"
  $ "$PYTHON" hgwebdir.cgi > page3

  $ grep -i error page1 page2 page3
  [1]
