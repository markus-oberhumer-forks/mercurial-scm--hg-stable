# Copyright 2017 Facebook, Inc.
#
# This software may be used and distributed according to the terms of the
# GNU General Public License version 2 or any later version.

from __future__ import absolute_import

from mercurial.i18n import _

from mercurial import (
    bundle2,
    changegroup,
    error,
    extensions,
    revsetlang,
    util,
)

from . import common

isremotebooksenabled = common.isremotebooksenabled

scratchbranchparttype = 'b2x:infinitepush'

def getscratchbranchparts(repo, peer, outgoing, confignonforwardmove,
                         ui, bookmark, create):
    if not outgoing.missing:
        raise error.Abort(_('no commits to push'))

    if scratchbranchparttype not in bundle2.bundle2caps(peer):
        raise error.Abort(_('no server support for %r') % scratchbranchparttype)

    _validaterevset(repo, revsetlang.formatspec('%ln', outgoing.missing),
                    bookmark)

    supportedversions = changegroup.supportedoutgoingversions(repo)
    # Explicitly avoid using '01' changegroup version in infinitepush to
    # support general delta
    supportedversions.discard('01')
    cgversion = min(supportedversions)
    _handlelfs(repo, outgoing.missing)
    cg = changegroup.makestream(repo, outgoing, cgversion, 'push')

    params = {}
    params['cgversion'] = cgversion
    if bookmark:
        params['bookmark'] = bookmark
        # 'prevbooknode' is necessary for pushkey reply part
        params['bookprevnode'] = ''
        if bookmark in repo:
            params['bookprevnode'] = repo[bookmark].hex()
        if create:
            params['create'] = '1'
    if confignonforwardmove:
        params['force'] = '1'

    # Do not send pushback bundle2 part with bookmarks if remotenames extension
    # is enabled. It will be handled manually in `_push()`
    if not isremotebooksenabled(ui):
        params['pushbackbookmarks'] = '1'

    parts = []

    # .upper() marks this as a mandatory part: server will abort if there's no
    #  handler
    parts.append(bundle2.bundlepart(
        scratchbranchparttype.upper(),
        advisoryparams=params.iteritems(),
        data=cg))

    try:
        treemod = extensions.find('treemanifest')
        mfnodes = []
        for node in outgoing.missing:
            mfnodes.append(('', repo[node].manifestnode()))

        # Only include the tree parts if they all exist
        if not repo.manifestlog.datastore.getmissing(mfnodes):
            parts.append(treemod.createtreepackpart(
                repo, outgoing, treemod.TREEGROUP_PARTTYPE2))
    except KeyError:
        pass

    return parts

def _validaterevset(repo, revset, bookmark):
    """Abort if the revs to be pushed aren't valid for a scratch branch."""
    if not repo.revs(revset):
        raise error.Abort(_('nothing to push'))
    if bookmark:
        # Allow bundle with many heads only if no bookmark is specified
        heads = repo.revs('heads(%r)', revset)
        if len(heads) > 1:
            raise error.Abort(
                _('cannot push more than one head to a scratch branch'))

def _handlelfs(repo, missing):
    '''Special case if lfs is enabled

    If lfs is enabled then we need to call prepush hook
    to make sure large files are uploaded to lfs
    '''
    try:
        lfsmod = extensions.find('lfs')
        lfsmod.wrapper.uploadblobsfromrevs(repo, missing)
    except KeyError:
        # Ignore if lfs extension is not enabled
        return

class copiedpart(object):
    """a copy of unbundlepart content that can be consumed later"""

    def __init__(self, part):
        # copy "public properties"
        self.type = part.type
        self.id = part.id
        self.mandatory = part.mandatory
        self.mandatoryparams = part.mandatoryparams
        self.advisoryparams = part.advisoryparams
        self.params = part.params
        self.mandatorykeys = part.mandatorykeys
        # copy the buffer
        self._io = util.stringio(part.read())

    def consume(self):
        return

    def read(self, size=None):
        if size is None:
            return self._io.read()
        else:
            return self._io.read(size)
