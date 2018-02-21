# state.py - writing and reading state files in Mercurial
#
# Copyright 2018 Pulkit Goyal <pulkitmgoyal@gmail.com>
#
# This software may be used and distributed according to the terms of the
# GNU General Public License version 2 or any later version.

"""
This file contains class to wrap the state for commands and other
related logic.

All the data related to the command state is stored as dictionary in the object.
The class has methods using which the data can be stored to disk in a file under
.hg/ directory.

We store the data on disk in cbor, for which we use the third party cbor library
to serialize and deserialize data.
"""

from __future__ import absolute_import

from .thirdparty import cbor

from . import (
    util,
)

class cmdstate(object):
    """a wrapper class to store the state of commands like `rebase`, `graft`,
    `histedit`, `shelve` etc. Extensions can also use this to write state files.

    All the data for the state is stored in the form of key-value pairs in a
    dictionary.

    The class object can write all the data to a file in .hg/ directory and
    can populate the object data reading that file.

    Uses cbor to serialize and deserialize data while writing and reading from
    disk.
    """

    def __init__(self, repo, fname, opts=None):
        """ repo is the repo object
        fname is the file name in which data should be stored in .hg directory
        opts is a dictionary of data of the statefile
        """
        self._repo = repo
        self.fname = fname
        if not opts:
            self.opts = {}
        else:
            self.opts = opts

    def __nonzero__(self):
        return self.exists()

    def __getitem__(self, key):
        return self.opts[key]

    def __setitem__(self, key, value):
        updates = {key: value}
        self.opts.update(updates)

    def load(self):
        """load the existing state file into the class object"""
        op = self._read()
        self.opts.update(op)

    def addopts(self, opts):
        """add more key-value pairs to the data stored by the object"""
        self.opts.update(opts)

    def save(self):
        """write all the state data stored to .hg/<filename> file

        we use third-party library cbor to serialize data to write in the file.
        """
        with self._repo.vfs(self.fname, 'wb', atomictemp=True) as fp:
            cbor.dump(self.opts, fp)

    def _read(self):
        """reads the state file and returns a dictionary which contain
        data in the same format as it was before storing"""
        with self._repo.vfs(self.fname, 'rb') as fp:
            return cbor.load(fp)

    def delete(self):
        """drop the state file if exists"""
        util.unlinkpath(self._repo.vfs.join(self.fname), ignoremissing=True)

    def exists(self):
        """check whether the state file exists or not"""
        return self._repo.vfs.exists(self.fname)
