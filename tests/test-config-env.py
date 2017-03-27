# Test the config layer generated by environment variables

from __future__ import absolute_import, print_function

import os

from mercurial import (
    encoding,
    rcutil,
    ui as uimod,
)

testtmp = encoding.environ['TESTTMP']

# prepare hgrc files
def join(name):
    return os.path.join(testtmp, name)

with open(join('sysrc'), 'w') as f:
    f.write('[ui]\neditor=e0\n[pager]\npager=p0\n')

with open(join('userrc'), 'w') as f:
    f.write('[ui]\neditor=e1')

# replace rcpath functions so they point to the files above
def systemrcpath():
    return [join('sysrc')]

def userrcpath():
    return [join('userrc')]

rcutil.systemrcpath = systemrcpath
rcutil.userrcpath = userrcpath
os.path.isdir = lambda x: False # hack: do not load default.d/*.rc

# utility to print configs
def printconfigs(env):
    encoding.environ = env
    rcutil._rccomponents = None # reset cache
    ui = uimod.ui.load()
    for section, name, value in ui.walkconfig():
        source = ui.configsource(section, name)
        print('%s.%s=%s # %s' % (section, name, value, source))
    print('')

# environment variable overrides
printconfigs({})
printconfigs({'EDITOR': 'e2', 'PAGER': 'p2'})
