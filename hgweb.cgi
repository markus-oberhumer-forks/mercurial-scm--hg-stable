#!/usr/bin/env python3
#
# An example hgweb CGI script, edit as necessary
# See also https://mercurial-scm.org/wiki/PublishingRepositories

# Path to repo or hgweb config to serve (see 'hg help hgweb')
config = b"/path/to/repo/or/config"

# Uncomment and adjust if Mercurial is not installed system-wide
# (consult "installed modules" path from 'hg debuginstall'):
# import sys; sys.path.insert(0, "/path/to/python/lib")

from mercurial import demandimport

demandimport.enable()
from mercurial.hgweb import hgweb, wsgicgi

application = hgweb(config)
wsgicgi.launch(application)
