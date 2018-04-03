# hgweb/webutil.py - utility library for the web interface.
#
# Copyright 21 May 2005 - (c) 2005 Jake Edge <jake@edge2.net>
# Copyright 2005-2007 Matt Mackall <mpm@selenic.com>
#
# This software may be used and distributed according to the terms of the
# GNU General Public License version 2 or any later version.

from __future__ import absolute_import

import copy
import difflib
import os
import re

from ..i18n import _
from ..node import hex, nullid, short

from .common import (
    ErrorResponse,
    HTTP_BAD_REQUEST,
    HTTP_NOT_FOUND,
    paritygen,
)

from .. import (
    context,
    error,
    match,
    mdiff,
    obsutil,
    patch,
    pathutil,
    pycompat,
    scmutil,
    templatefilters,
    templatekw,
    templateutil,
    ui as uimod,
    util,
)

from ..utils import (
    stringutil,
)

archivespecs = util.sortdict((
    ('zip', ('application/zip', 'zip', '.zip', None)),
    ('gz', ('application/x-gzip', 'tgz', '.tar.gz', None)),
    ('bz2', ('application/x-bzip2', 'tbz2', '.tar.bz2', None)),
))

def archivelist(ui, nodeid, url=None):
    allowed = ui.configlist('web', 'allow_archive', untrusted=True)
    archives = []

    for typ, spec in archivespecs.iteritems():
        if typ in allowed or ui.configbool('web', 'allow' + typ,
                                           untrusted=True):
            archives.append({
                'type': typ,
                'extension': spec[2],
                'node': nodeid,
                'url': url,
            })

    return templateutil.mappinglist(archives)

def up(p):
    if p[0:1] != "/":
        p = "/" + p
    if p[-1:] == "/":
        p = p[:-1]
    up = os.path.dirname(p)
    if up == "/":
        return "/"
    return up + "/"

def _navseq(step, firststep=None):
    if firststep:
        yield firststep
        if firststep >= 20 and firststep <= 40:
            firststep = 50
            yield firststep
        assert step > 0
        assert firststep > 0
        while step <= firststep:
            step *= 10
    while True:
        yield 1 * step
        yield 3 * step
        step *= 10

class revnav(object):

    def __init__(self, repo):
        """Navigation generation object

        :repo: repo object we generate nav for
        """
        # used for hex generation
        self._revlog = repo.changelog

    def __nonzero__(self):
        """return True if any revision to navigate over"""
        return self._first() is not None

    __bool__ = __nonzero__

    def _first(self):
        """return the minimum non-filtered changeset or None"""
        try:
            return next(iter(self._revlog))
        except StopIteration:
            return None

    def hex(self, rev):
        return hex(self._revlog.node(rev))

    def gen(self, pos, pagelen, limit):
        """computes label and revision id for navigation link

        :pos: is the revision relative to which we generate navigation.
        :pagelen: the size of each navigation page
        :limit: how far shall we link

        The return is:
            - a single element mappinglist
            - containing a dictionary with a `before` and `after` key
            - values are dictionaries with `label` and `node` keys
        """
        if not self:
            # empty repo
            return templateutil.mappinglist([
                {'before': templateutil.mappinglist([]),
                 'after': templateutil.mappinglist([])},
            ])

        targets = []
        for f in _navseq(1, pagelen):
            if f > limit:
                break
            targets.append(pos + f)
            targets.append(pos - f)
        targets.sort()

        first = self._first()
        navbefore = [{'label': '(%i)' % first, 'node': self.hex(first)}]
        navafter = []
        for rev in targets:
            if rev not in self._revlog:
                continue
            if pos < rev < limit:
                navafter.append({'label': '+%d' % abs(rev - pos),
                                 'node': self.hex(rev)})
            if 0 < rev < pos:
                navbefore.append({'label': '-%d' % abs(rev - pos),
                                  'node': self.hex(rev)})

        navafter.append({'label': 'tip', 'node': 'tip'})

        # TODO: maybe this can be a scalar object supporting tomap()
        return templateutil.mappinglist([
            {'before': templateutil.mappinglist(navbefore),
             'after': templateutil.mappinglist(navafter)},
        ])

class filerevnav(revnav):

    def __init__(self, repo, path):
        """Navigation generation object

        :repo: repo object we generate nav for
        :path: path of the file we generate nav for
        """
        # used for iteration
        self._changelog = repo.unfiltered().changelog
        # used for hex generation
        self._revlog = repo.file(path)

    def hex(self, rev):
        return hex(self._changelog.node(self._revlog.linkrev(rev)))

# TODO: maybe this can be a wrapper class for changectx/filectx list, which
# yields {'ctx': ctx}
def _ctxsgen(context, ctxs):
    for s in ctxs:
        d = {
            'node': s.hex(),
            'rev': s.rev(),
            'user': s.user(),
            'date': s.date(),
            'description': s.description(),
            'branch': s.branch(),
        }
        if util.safehasattr(s, 'path'):
            d['file'] = s.path()
        yield d

def _siblings(siblings=None, hiderev=None):
    if siblings is None:
        siblings = []
    siblings = [s for s in siblings if s.node() != nullid]
    if len(siblings) == 1 and siblings[0].rev() == hiderev:
        siblings = []
    return templateutil.mappinggenerator(_ctxsgen, args=(siblings,))

def difffeatureopts(req, ui, section):
    diffopts = patch.difffeatureopts(ui, untrusted=True,
                                     section=section, whitespace=True)

    for k in ('ignorews', 'ignorewsamount', 'ignorewseol', 'ignoreblanklines'):
        v = req.qsparams.get(k)
        if v is not None:
            v = stringutil.parsebool(v)
            setattr(diffopts, k, v if v is not None else True)

    return diffopts

def annotate(req, fctx, ui):
    diffopts = difffeatureopts(req, ui, 'annotate')
    return fctx.annotate(follow=True, diffopts=diffopts)

def parents(ctx, hide=None):
    if isinstance(ctx, context.basefilectx):
        introrev = ctx.introrev()
        if ctx.changectx().rev() != introrev:
            return _siblings([ctx.repo()[introrev]], hide)
    return _siblings(ctx.parents(), hide)

def children(ctx, hide=None):
    return _siblings(ctx.children(), hide)

def renamelink(fctx):
    r = fctx.renamed()
    if r:
        return templateutil.mappinglist([{'file': r[0], 'node': hex(r[1])}])
    return templateutil.mappinglist([])

def nodetagsdict(repo, node):
    return templateutil.hybridlist(repo.nodetags(node), name='name')

def nodebookmarksdict(repo, node):
    return templateutil.hybridlist(repo.nodebookmarks(node), name='name')

def nodebranchdict(repo, ctx):
    branches = []
    branch = ctx.branch()
    # If this is an empty repo, ctx.node() == nullid,
    # ctx.branch() == 'default'.
    try:
        branchnode = repo.branchtip(branch)
    except error.RepoLookupError:
        branchnode = None
    if branchnode == ctx.node():
        branches.append(branch)
    return templateutil.hybridlist(branches, name='name')

def nodeinbranch(repo, ctx):
    branches = []
    branch = ctx.branch()
    try:
        branchnode = repo.branchtip(branch)
    except error.RepoLookupError:
        branchnode = None
    if branch != 'default' and branchnode != ctx.node():
        branches.append(branch)
    return templateutil.hybridlist(branches, name='name')

def nodebranchnodefault(ctx):
    branches = []
    branch = ctx.branch()
    if branch != 'default':
        branches.append(branch)
    return templateutil.hybridlist(branches, name='name')

def _nodenamesgen(context, f, node, name):
    for t in f(node):
        yield {name: t}

def showtag(repo, t1, node=nullid):
    args = (repo.nodetags, node, 'tag')
    return templateutil.mappinggenerator(_nodenamesgen, args=args, name=t1)

def showbookmark(repo, t1, node=nullid):
    args = (repo.nodebookmarks, node, 'bookmark')
    return templateutil.mappinggenerator(_nodenamesgen, args=args, name=t1)

def branchentries(repo, stripecount, limit=0):
    tips = []
    heads = repo.heads()
    parity = paritygen(stripecount)
    sortkey = lambda item: (not item[1], item[0].rev())

    def entries(context):
        count = 0
        if not tips:
            for tag, hs, tip, closed in repo.branchmap().iterbranches():
                tips.append((repo[tip], closed))
        for ctx, closed in sorted(tips, key=sortkey, reverse=True):
            if limit > 0 and count >= limit:
                return
            count += 1
            if closed:
                status = 'closed'
            elif ctx.node() not in heads:
                status = 'inactive'
            else:
                status = 'open'
            yield {
                'parity': next(parity),
                'branch': ctx.branch(),
                'status': status,
                'node': ctx.hex(),
                'date': ctx.date()
            }

    return templateutil.mappinggenerator(entries)

def cleanpath(repo, path):
    path = path.lstrip('/')
    return pathutil.canonpath(repo.root, '', path)

def changectx(repo, req):
    changeid = "tip"
    if 'node' in req.qsparams:
        changeid = req.qsparams['node']
        ipos = changeid.find(':')
        if ipos != -1:
            changeid = changeid[(ipos + 1):]

    return scmutil.revsymbol(repo, changeid)

def basechangectx(repo, req):
    if 'node' in req.qsparams:
        changeid = req.qsparams['node']
        ipos = changeid.find(':')
        if ipos != -1:
            changeid = changeid[:ipos]
            return scmutil.revsymbol(repo, changeid)

    return None

def filectx(repo, req):
    if 'file' not in req.qsparams:
        raise ErrorResponse(HTTP_NOT_FOUND, 'file not given')
    path = cleanpath(repo, req.qsparams['file'])
    if 'node' in req.qsparams:
        changeid = req.qsparams['node']
    elif 'filenode' in req.qsparams:
        changeid = req.qsparams['filenode']
    else:
        raise ErrorResponse(HTTP_NOT_FOUND, 'node or filenode not given')
    try:
        fctx = scmutil.revsymbol(repo, changeid)[path]
    except error.RepoError:
        fctx = repo.filectx(path, fileid=changeid)

    return fctx

def linerange(req):
    linerange = req.qsparams.getall('linerange')
    if not linerange:
        return None
    if len(linerange) > 1:
        raise ErrorResponse(HTTP_BAD_REQUEST,
                            'redundant linerange parameter')
    try:
        fromline, toline = map(int, linerange[0].split(':', 1))
    except ValueError:
        raise ErrorResponse(HTTP_BAD_REQUEST,
                            'invalid linerange parameter')
    try:
        return util.processlinerange(fromline, toline)
    except error.ParseError as exc:
        raise ErrorResponse(HTTP_BAD_REQUEST, pycompat.bytestr(exc))

def formatlinerange(fromline, toline):
    return '%d:%d' % (fromline + 1, toline)

def _succsandmarkersgen(context, mapping):
    repo = context.resource(mapping, 'repo')
    itemmappings = templatekw.showsuccsandmarkers(context, mapping)
    for item in itemmappings.tovalue(context, mapping):
        item['successors'] = _siblings(repo[successor]
                                       for successor in item['successors'])
        yield item

def succsandmarkers(context, mapping):
    return templateutil.mappinggenerator(_succsandmarkersgen, args=(mapping,))

# teach templater succsandmarkers is switched to (context, mapping) API
succsandmarkers._requires = {'repo', 'ctx'}

def _whyunstablegen(context, mapping):
    repo = context.resource(mapping, 'repo')
    ctx = context.resource(mapping, 'ctx')

    entries = obsutil.whyunstable(repo, ctx)
    for entry in entries:
        if entry.get('divergentnodes'):
            entry['divergentnodes'] = _siblings(entry['divergentnodes'])
        yield entry

def whyunstable(context, mapping):
    return templateutil.mappinggenerator(_whyunstablegen, args=(mapping,))

whyunstable._requires = {'repo', 'ctx'}

def commonentry(repo, ctx):
    node = ctx.node()
    return {
        # TODO: perhaps ctx.changectx() should be assigned if ctx is a
        # filectx, but I'm not pretty sure if that would always work because
        # fctx.parents() != fctx.changectx.parents() for example.
        'ctx': ctx,
        'rev': ctx.rev(),
        'node': hex(node),
        'author': ctx.user(),
        'desc': ctx.description(),
        'date': ctx.date(),
        'extra': ctx.extra(),
        'phase': ctx.phasestr(),
        'obsolete': ctx.obsolete(),
        'succsandmarkers': succsandmarkers,
        'instabilities': templateutil.hybridlist(ctx.instabilities(),
                                                 name='instability'),
        'whyunstable': whyunstable,
        'branch': nodebranchnodefault(ctx),
        'inbranch': nodeinbranch(repo, ctx),
        'branches': nodebranchdict(repo, ctx),
        'tags': nodetagsdict(repo, node),
        'bookmarks': nodebookmarksdict(repo, node),
        'parent': lambda **x: parents(ctx),
        'child': lambda **x: children(ctx),
    }

def changelistentry(web, ctx):
    '''Obtain a dictionary to be used for entries in a changelist.

    This function is called when producing items for the "entries" list passed
    to the "shortlog" and "changelog" templates.
    '''
    repo = web.repo
    rev = ctx.rev()
    n = ctx.node()
    showtags = showtag(repo, 'changelogtag', n)
    files = listfilediffs(web.tmpl, ctx.files(), n, web.maxfiles)

    entry = commonentry(repo, ctx)
    entry.update(
        allparents=lambda **x: parents(ctx),
        parent=lambda **x: parents(ctx, rev - 1),
        child=lambda **x: children(ctx, rev + 1),
        changelogtag=showtags,
        files=files,
    )
    return entry

def symrevorshortnode(req, ctx):
    if 'node' in req.qsparams:
        return templatefilters.revescape(req.qsparams['node'])
    else:
        return short(ctx.node())

def _listfilesgen(context, ctx, stripecount):
    parity = paritygen(stripecount)
    for blockno, f in enumerate(ctx.files()):
        template = 'filenodelink' if f in ctx else 'filenolink'
        yield context.process(template, {
            'node': ctx.hex(),
            'file': f,
            'blockno': blockno + 1,
            'parity': next(parity),
        })

def changesetentry(web, ctx):
    '''Obtain a dictionary to be used to render the "changeset" template.'''

    showtags = showtag(web.repo, 'changesettag', ctx.node())
    showbookmarks = showbookmark(web.repo, 'changesetbookmark', ctx.node())
    showbranch = nodebranchnodefault(ctx)

    basectx = basechangectx(web.repo, web.req)
    if basectx is None:
        basectx = ctx.p1()

    style = web.config('web', 'style')
    if 'style' in web.req.qsparams:
        style = web.req.qsparams['style']

    diff = diffs(web, ctx, basectx, None, style)

    parity = paritygen(web.stripecount)
    diffstatsgen = diffstatgen(ctx, basectx)
    diffstats = diffstat(web.tmpl, ctx, diffstatsgen, parity)

    return dict(
        diff=diff,
        symrev=symrevorshortnode(web.req, ctx),
        basenode=basectx.hex(),
        changesettag=showtags,
        changesetbookmark=showbookmarks,
        changesetbranch=showbranch,
        files=templateutil.mappedgenerator(_listfilesgen,
                                           args=(ctx, web.stripecount)),
        diffsummary=lambda **x: diffsummary(diffstatsgen),
        diffstat=diffstats,
        archives=web.archivelist(ctx.hex()),
        **pycompat.strkwargs(commonentry(web.repo, ctx)))

def _listfilediffsgen(context, files, node, max):
    for f in files[:max]:
        yield context.process('filedifflink', {'node': hex(node), 'file': f})
    if len(files) > max:
        yield context.process('fileellipses', {})

def listfilediffs(tmpl, files, node, max):
    return templateutil.mappedgenerator(_listfilediffsgen,
                                        args=(files, node, max))

def diffs(web, ctx, basectx, files, style, linerange=None,
          lineidprefix=''):

    def prettyprintlines(lines, blockno):
        for lineno, l in enumerate(lines, 1):
            difflineno = "%d.%d" % (blockno, lineno)
            if l.startswith('+'):
                ltype = "difflineplus"
            elif l.startswith('-'):
                ltype = "difflineminus"
            elif l.startswith('@'):
                ltype = "difflineat"
            else:
                ltype = "diffline"
            yield web.tmpl.generate(ltype, {
                'line': l,
                'lineno': lineno,
                'lineid': lineidprefix + "l%s" % difflineno,
                'linenumber': "% 8s" % difflineno,
            })

    repo = web.repo
    if files:
        m = match.exact(repo.root, repo.getcwd(), files)
    else:
        m = match.always(repo.root, repo.getcwd())

    diffopts = patch.diffopts(repo.ui, untrusted=True)
    node1 = basectx.node()
    node2 = ctx.node()
    parity = paritygen(web.stripecount)

    diffhunks = patch.diffhunks(repo, node1, node2, m, opts=diffopts)
    for blockno, (fctx1, fctx2, header, hunks) in enumerate(diffhunks, 1):
        if style != 'raw':
            header = header[1:]
        lines = [h + '\n' for h in header]
        for hunkrange, hunklines in hunks:
            if linerange is not None and hunkrange is not None:
                s1, l1, s2, l2 = hunkrange
                if not mdiff.hunkinrange((s2, l2), linerange):
                    continue
            lines.extend(hunklines)
        if lines:
            yield web.tmpl.generate('diffblock', {
                'parity': next(parity),
                'blockno': blockno,
                'lines': prettyprintlines(lines, blockno),
            })

def compare(tmpl, context, leftlines, rightlines):
    '''Generator function that provides side-by-side comparison data.'''

    def compline(type, leftlineno, leftline, rightlineno, rightline):
        lineid = leftlineno and ("l%d" % leftlineno) or ''
        lineid += rightlineno and ("r%d" % rightlineno) or ''
        llno = '%d' % leftlineno if leftlineno else ''
        rlno = '%d' % rightlineno if rightlineno else ''
        return tmpl.generate('comparisonline', {
            'type': type,
            'lineid': lineid,
            'leftlineno': leftlineno,
            'leftlinenumber': "% 6s" % llno,
            'leftline': leftline or '',
            'rightlineno': rightlineno,
            'rightlinenumber': "% 6s" % rlno,
            'rightline': rightline or '',
        })

    def getblock(opcodes):
        for type, llo, lhi, rlo, rhi in opcodes:
            len1 = lhi - llo
            len2 = rhi - rlo
            count = min(len1, len2)
            for i in xrange(count):
                yield compline(type=type,
                               leftlineno=llo + i + 1,
                               leftline=leftlines[llo + i],
                               rightlineno=rlo + i + 1,
                               rightline=rightlines[rlo + i])
            if len1 > len2:
                for i in xrange(llo + count, lhi):
                    yield compline(type=type,
                                   leftlineno=i + 1,
                                   leftline=leftlines[i],
                                   rightlineno=None,
                                   rightline=None)
            elif len2 > len1:
                for i in xrange(rlo + count, rhi):
                    yield compline(type=type,
                                   leftlineno=None,
                                   leftline=None,
                                   rightlineno=i + 1,
                                   rightline=rightlines[i])

    s = difflib.SequenceMatcher(None, leftlines, rightlines)
    if context < 0:
        yield tmpl.generate('comparisonblock',
                            {'lines': getblock(s.get_opcodes())})
    else:
        for oc in s.get_grouped_opcodes(n=context):
            yield tmpl.generate('comparisonblock', {'lines': getblock(oc)})

def diffstatgen(ctx, basectx):
    '''Generator function that provides the diffstat data.'''

    stats = patch.diffstatdata(
        util.iterlines(ctx.diff(basectx, noprefix=False)))
    maxname, maxtotal, addtotal, removetotal, binary = patch.diffstatsum(stats)
    while True:
        yield stats, maxname, maxtotal, addtotal, removetotal, binary

def diffsummary(statgen):
    '''Return a short summary of the diff.'''

    stats, maxname, maxtotal, addtotal, removetotal, binary = next(statgen)
    return _(' %d files changed, %d insertions(+), %d deletions(-)\n') % (
             len(stats), addtotal, removetotal)

def diffstat(tmpl, ctx, statgen, parity):
    '''Return a diffstat template for each file in the diff.'''

    stats, maxname, maxtotal, addtotal, removetotal, binary = next(statgen)
    files = ctx.files()

    def pct(i):
        if maxtotal == 0:
            return 0
        return (float(i) / maxtotal) * 100

    fileno = 0
    for filename, adds, removes, isbinary in stats:
        template = 'diffstatlink' if filename in files else 'diffstatnolink'
        total = adds + removes
        fileno += 1
        yield tmpl.generate(template, {
            'node': ctx.hex(),
            'file': filename,
            'fileno': fileno,
            'total': total,
            'addpct': pct(adds),
            'removepct': pct(removes),
            'parity': next(parity),
        })

class sessionvars(templateutil.wrapped):
    def __init__(self, vars, start='?'):
        self._start = start
        self._vars = vars

    def __getitem__(self, key):
        return self._vars[key]

    def __setitem__(self, key, value):
        self._vars[key] = value

    def __copy__(self):
        return sessionvars(copy.copy(self._vars), self._start)

    def itermaps(self, context):
        separator = self._start
        for key, value in sorted(self._vars.iteritems()):
            yield {'name': key,
                   'value': pycompat.bytestr(value),
                   'separator': separator,
            }
            separator = '&'

    def join(self, context, mapping, sep):
        # could be '{separator}{name}={value|urlescape}'
        raise error.ParseError(_('not displayable without template'))

    def show(self, context, mapping):
        return self.join(context, '')

    def tovalue(self, context, mapping):
        return self._vars

class wsgiui(uimod.ui):
    # default termwidth breaks under mod_wsgi
    def termwidth(self):
        return 80

def getwebsubs(repo):
    websubtable = []
    websubdefs = repo.ui.configitems('websub')
    # we must maintain interhg backwards compatibility
    websubdefs += repo.ui.configitems('interhg')
    for key, pattern in websubdefs:
        # grab the delimiter from the character after the "s"
        unesc = pattern[1:2]
        delim = re.escape(unesc)

        # identify portions of the pattern, taking care to avoid escaped
        # delimiters. the replace format and flags are optional, but
        # delimiters are required.
        match = re.match(
            br'^s%s(.+)(?:(?<=\\\\)|(?<!\\))%s(.*)%s([ilmsux])*$'
            % (delim, delim, delim), pattern)
        if not match:
            repo.ui.warn(_("websub: invalid pattern for %s: %s\n")
                              % (key, pattern))
            continue

        # we need to unescape the delimiter for regexp and format
        delim_re = re.compile(br'(?<!\\)\\%s' % delim)
        regexp = delim_re.sub(unesc, match.group(1))
        format = delim_re.sub(unesc, match.group(2))

        # the pattern allows for 6 regexp flags, so set them if necessary
        flagin = match.group(3)
        flags = 0
        if flagin:
            for flag in flagin.upper():
                flags |= re.__dict__[flag]

        try:
            regexp = re.compile(regexp, flags)
            websubtable.append((regexp, format))
        except re.error:
            repo.ui.warn(_("websub: invalid regexp for %s: %s\n")
                         % (key, regexp))
    return websubtable

def getgraphnode(repo, ctx):
    return (templatekw.getgraphnodecurrent(repo, ctx) +
            templatekw.getgraphnodesymbol(ctx))
