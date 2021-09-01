# dirstatenonnormalcheck.py - extension to check the consistency of the
# dirstate's non-normal map
#
# For most operations on dirstate, this extensions checks that the nonnormalset
# contains the right entries.
# It compares the nonnormal file to a nonnormalset built from the map of all
# the files in the dirstate to check that they contain the same files.

from __future__ import absolute_import

from mercurial import (
    dirstate,
    extensions,
    pycompat,
)


def nonnormalentries(dmap):
    """Compute nonnormal entries from dirstate's dmap"""
    res = set()
    for f, e in dmap.iteritems():
        if e.state != b'n' or e.mtime == -1:
            res.add(f)
    return res


INCONSISTENCY_MESSAGE = b"""%s call to %s
  inconsistency in nonnormalset
  result from dirstatemap: %s
  expected nonnormalset:   %s
"""


def checkconsistency(ui, orig, dmap, _nonnormalset, label):
    """Compute nonnormalset from dmap, check that it matches _nonnormalset"""
    nonnormalcomputedmap = nonnormalentries(dmap)
    if _nonnormalset != nonnormalcomputedmap:
        b_orig = pycompat.sysbytes(repr(orig))
        b_nonnormal = pycompat.sysbytes(repr(_nonnormalset))
        b_nonnormalcomputed = pycompat.sysbytes(repr(nonnormalcomputedmap))
        msg = INCONSISTENCY_MESSAGE % (
            label,
            b_orig,
            b_nonnormal,
            b_nonnormalcomputed,
        )
        ui.develwarn(msg, config=b'dirstate')


def _checkdirstate(orig, self, *args, **kwargs):
    """Check nonnormal set consistency before and after the call to orig"""
    checkconsistency(
        self._ui, orig, self._map, self._map.nonnormalset, b"before"
    )
    r = orig(self, *args, **kwargs)
    checkconsistency(
        self._ui, orig, self._map, self._map.nonnormalset, b"after"
    )
    return r


def extsetup(ui):
    """Wrap functions modifying dirstate to check nonnormalset consistency"""
    dirstatecl = dirstate.dirstate
    devel = ui.configbool(b'devel', b'all-warnings')
    paranoid = ui.configbool(b'experimental', b'nonnormalparanoidcheck')
    if devel:
        extensions.wrapfunction(dirstatecl, '_writedirstate', _checkdirstate)
        if paranoid:
            # We don't do all these checks when paranoid is disable as it would
            # make the extension run very slowly on large repos
            extensions.wrapfunction(dirstatecl, 'write', _checkdirstate)
            extensions.wrapfunction(dirstatecl, 'set_tracked', _checkdirstate)
            extensions.wrapfunction(dirstatecl, 'set_untracked', _checkdirstate)
            extensions.wrapfunction(
                dirstatecl, 'set_possibly_dirty', _checkdirstate
            )
            extensions.wrapfunction(
                dirstatecl, 'update_file_p1', _checkdirstate
            )
            extensions.wrapfunction(dirstatecl, 'update_file', _checkdirstate)
