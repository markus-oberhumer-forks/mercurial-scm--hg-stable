# templateutil.py - utility for template evaluation
#
# Copyright 2005, 2006 Matt Mackall <mpm@selenic.com>
#
# This software may be used and distributed according to the terms of the
# GNU General Public License version 2 or any later version.

from __future__ import absolute_import

import types

from .i18n import _
from . import (
    error,
    pycompat,
    util,
)
from .utils import (
    stringutil,
)

class ResourceUnavailable(error.Abort):
    pass

class TemplateNotFound(error.Abort):
    pass

class hybrid(object):
    """Wrapper for list or dict to support legacy template

    This class allows us to handle both:
    - "{files}" (legacy command-line-specific list hack) and
    - "{files % '{file}\n'}" (hgweb-style with inlining and function support)
    and to access raw values:
    - "{ifcontains(file, files, ...)}", "{ifcontains(key, extras, ...)}"
    - "{get(extras, key)}"
    - "{files|json}"
    """

    def __init__(self, gen, values, makemap, joinfmt, keytype=None):
        if gen is not None:
            self.gen = gen  # generator or function returning generator
        self._values = values
        self._makemap = makemap
        self.joinfmt = joinfmt
        self.keytype = keytype  # hint for 'x in y' where type(x) is unresolved
    def gen(self):
        """Default generator to stringify this as {join(self, ' ')}"""
        for i, x in enumerate(self._values):
            if i > 0:
                yield ' '
            yield self.joinfmt(x)
    def itermaps(self):
        makemap = self._makemap
        for x in self._values:
            yield makemap(x)
    def __contains__(self, x):
        return x in self._values
    def __getitem__(self, key):
        return self._values[key]
    def __len__(self):
        return len(self._values)
    def __iter__(self):
        return iter(self._values)
    def __getattr__(self, name):
        if name not in (r'get', r'items', r'iteritems', r'iterkeys',
                        r'itervalues', r'keys', r'values'):
            raise AttributeError(name)
        return getattr(self._values, name)

class mappable(object):
    """Wrapper for non-list/dict object to support map operation

    This class allows us to handle both:
    - "{manifest}"
    - "{manifest % '{rev}:{node}'}"
    - "{manifest.rev}"

    Unlike a hybrid, this does not simulate the behavior of the underling
    value. Use unwrapvalue() or unwraphybrid() to obtain the inner object.
    """

    def __init__(self, gen, key, value, makemap):
        if gen is not None:
            self.gen = gen  # generator or function returning generator
        self._key = key
        self._value = value  # may be generator of strings
        self._makemap = makemap

    def gen(self):
        yield pycompat.bytestr(self._value)

    def tomap(self):
        return self._makemap(self._key)

    def itermaps(self):
        yield self.tomap()

def hybriddict(data, key='key', value='value', fmt=None, gen=None):
    """Wrap data to support both dict-like and string-like operations"""
    prefmt = pycompat.identity
    if fmt is None:
        fmt = '%s=%s'
        prefmt = pycompat.bytestr
    return hybrid(gen, data, lambda k: {key: k, value: data[k]},
                  lambda k: fmt % (prefmt(k), prefmt(data[k])))

def hybridlist(data, name, fmt=None, gen=None):
    """Wrap data to support both list-like and string-like operations"""
    prefmt = pycompat.identity
    if fmt is None:
        fmt = '%s'
        prefmt = pycompat.bytestr
    return hybrid(gen, data, lambda x: {name: x}, lambda x: fmt % prefmt(x))

def unwraphybrid(thing):
    """Return an object which can be stringified possibly by using a legacy
    template"""
    gen = getattr(thing, 'gen', None)
    if gen is None:
        return thing
    if callable(gen):
        return gen()
    return gen

def unwrapvalue(thing):
    """Move the inner value object out of the wrapper"""
    if not util.safehasattr(thing, '_value'):
        return thing
    return thing._value

def wraphybridvalue(container, key, value):
    """Wrap an element of hybrid container to be mappable

    The key is passed to the makemap function of the given container, which
    should be an item generated by iter(container).
    """
    makemap = getattr(container, '_makemap', None)
    if makemap is None:
        return value
    if util.safehasattr(value, '_makemap'):
        # a nested hybrid list/dict, which has its own way of map operation
        return value
    return mappable(None, key, value, makemap)

def compatdict(context, mapping, name, data, key='key', value='value',
               fmt=None, plural=None, separator=' '):
    """Wrap data like hybriddict(), but also supports old-style list template

    This exists for backward compatibility with the old-style template. Use
    hybriddict() for new template keywords.
    """
    c = [{key: k, value: v} for k, v in data.iteritems()]
    f = _showcompatlist(context, mapping, name, c, plural, separator)
    return hybriddict(data, key=key, value=value, fmt=fmt, gen=f)

def compatlist(context, mapping, name, data, element=None, fmt=None,
               plural=None, separator=' '):
    """Wrap data like hybridlist(), but also supports old-style list template

    This exists for backward compatibility with the old-style template. Use
    hybridlist() for new template keywords.
    """
    f = _showcompatlist(context, mapping, name, data, plural, separator)
    return hybridlist(data, name=element or name, fmt=fmt, gen=f)

def _showcompatlist(context, mapping, name, values, plural=None, separator=' '):
    """Return a generator that renders old-style list template

    name is name of key in template map.
    values is list of strings or dicts.
    plural is plural of name, if not simply name + 's'.
    separator is used to join values as a string

    expansion works like this, given name 'foo'.

    if values is empty, expand 'no_foos'.

    if 'foo' not in template map, return values as a string,
    joined by 'separator'.

    expand 'start_foos'.

    for each value, expand 'foo'. if 'last_foo' in template
    map, expand it instead of 'foo' for last key.

    expand 'end_foos'.
    """
    if not plural:
        plural = name + 's'
    if not values:
        noname = 'no_' + plural
        if context.preload(noname):
            yield context.process(noname, mapping)
        return
    if not context.preload(name):
        if isinstance(values[0], bytes):
            yield separator.join(values)
        else:
            for v in values:
                r = dict(v)
                r.update(mapping)
                yield r
        return
    startname = 'start_' + plural
    if context.preload(startname):
        yield context.process(startname, mapping)
    def one(v, tag=name):
        vmapping = {}
        try:
            vmapping.update(v)
        # Python 2 raises ValueError if the type of v is wrong. Python
        # 3 raises TypeError.
        except (AttributeError, TypeError, ValueError):
            try:
                # Python 2 raises ValueError trying to destructure an e.g.
                # bytes. Python 3 raises TypeError.
                for a, b in v:
                    vmapping[a] = b
            except (TypeError, ValueError):
                vmapping[name] = v
        vmapping = context.overlaymap(mapping, vmapping)
        return context.process(tag, vmapping)
    lastname = 'last_' + name
    if context.preload(lastname):
        last = values.pop()
    else:
        last = None
    for v in values:
        yield one(v)
    if last is not None:
        yield one(last, tag=lastname)
    endname = 'end_' + plural
    if context.preload(endname):
        yield context.process(endname, mapping)

def flatten(thing):
    """Yield a single stream from a possibly nested set of iterators"""
    thing = unwraphybrid(thing)
    if isinstance(thing, bytes):
        yield thing
    elif isinstance(thing, str):
        # We can only hit this on Python 3, and it's here to guard
        # against infinite recursion.
        raise error.ProgrammingError('Mercurial IO including templates is done'
                                     ' with bytes, not strings, got %r' % thing)
    elif thing is None:
        pass
    elif not util.safehasattr(thing, '__iter__'):
        yield pycompat.bytestr(thing)
    else:
        for i in thing:
            i = unwraphybrid(i)
            if isinstance(i, bytes):
                yield i
            elif i is None:
                pass
            elif not util.safehasattr(i, '__iter__'):
                yield pycompat.bytestr(i)
            else:
                for j in flatten(i):
                    yield j

def stringify(thing):
    """Turn values into bytes by converting into text and concatenating them"""
    if isinstance(thing, bytes):
        return thing  # retain localstr to be round-tripped
    return b''.join(flatten(thing))

def findsymbolicname(arg):
    """Find symbolic name for the given compiled expression; returns None
    if nothing found reliably"""
    while True:
        func, data = arg
        if func is runsymbol:
            return data
        elif func is runfilter:
            arg = data[0]
        else:
            return None

def evalrawexp(context, mapping, arg):
    """Evaluate given argument as a bare template object which may require
    further processing (such as folding generator of strings)"""
    func, data = arg
    return func(context, mapping, data)

def evalfuncarg(context, mapping, arg):
    """Evaluate given argument as value type"""
    return _unwrapvalue(evalrawexp(context, mapping, arg))

# TODO: unify this with unwrapvalue() once the bug of templatefunc.join()
# is fixed. we can't do that right now because join() has to take a generator
# of byte strings as it is, not a lazy byte string.
def _unwrapvalue(thing):
    thing = unwrapvalue(thing)
    # evalrawexp() may return string, generator of strings or arbitrary object
    # such as date tuple, but filter does not want generator.
    if isinstance(thing, types.GeneratorType):
        thing = stringify(thing)
    return thing

def evalboolean(context, mapping, arg):
    """Evaluate given argument as boolean, but also takes boolean literals"""
    func, data = arg
    if func is runsymbol:
        thing = func(context, mapping, data, default=None)
        if thing is None:
            # not a template keyword, takes as a boolean literal
            thing = stringutil.parsebool(data)
    else:
        thing = func(context, mapping, data)
    thing = unwrapvalue(thing)
    if isinstance(thing, bool):
        return thing
    # other objects are evaluated as strings, which means 0 is True, but
    # empty dict/list should be False as they are expected to be ''
    return bool(stringify(thing))

def evalinteger(context, mapping, arg, err=None):
    v = evalfuncarg(context, mapping, arg)
    try:
        return int(v)
    except (TypeError, ValueError):
        raise error.ParseError(err or _('not an integer'))

def evalstring(context, mapping, arg):
    return stringify(evalrawexp(context, mapping, arg))

def evalstringliteral(context, mapping, arg):
    """Evaluate given argument as string template, but returns symbol name
    if it is unknown"""
    func, data = arg
    if func is runsymbol:
        thing = func(context, mapping, data, default=data)
    else:
        thing = func(context, mapping, data)
    return stringify(thing)

_evalfuncbytype = {
    bytes: evalstring,
    int: evalinteger,
}

def evalastype(context, mapping, arg, typ):
    """Evaluate given argument and coerce its type"""
    try:
        f = _evalfuncbytype[typ]
    except KeyError:
        raise error.ProgrammingError('invalid type specified: %r' % typ)
    return f(context, mapping, arg)

def runinteger(context, mapping, data):
    return int(data)

def runstring(context, mapping, data):
    return data

def _recursivesymbolblocker(key):
    def showrecursion(**args):
        raise error.Abort(_("recursive reference '%s' in template") % key)
    return showrecursion

def runsymbol(context, mapping, key, default=''):
    v = context.symbol(mapping, key)
    if v is None:
        # put poison to cut recursion. we can't move this to parsing phase
        # because "x = {x}" is allowed if "x" is a keyword. (issue4758)
        safemapping = mapping.copy()
        safemapping[key] = _recursivesymbolblocker(key)
        try:
            v = context.process(key, safemapping)
        except TemplateNotFound:
            v = default
    if callable(v) and getattr(v, '_requires', None) is None:
        # old templatekw: expand all keywords and resources
        # (TODO: deprecate this after porting web template keywords to new API)
        props = {k: context._resources.lookup(context, mapping, k)
                 for k in context._resources.knownkeys()}
        # pass context to _showcompatlist() through templatekw._showlist()
        props['templ'] = context
        props.update(mapping)
        return v(**pycompat.strkwargs(props))
    if callable(v):
        # new templatekw
        try:
            return v(context, mapping)
        except ResourceUnavailable:
            # unsupported keyword is mapped to empty just like unknown keyword
            return None
    return v

def runtemplate(context, mapping, template):
    for arg in template:
        yield evalrawexp(context, mapping, arg)

def runfilter(context, mapping, data):
    arg, filt = data
    thing = evalfuncarg(context, mapping, arg)
    try:
        return filt(thing)
    except (ValueError, AttributeError, TypeError):
        sym = findsymbolicname(arg)
        if sym:
            msg = (_("template filter '%s' is not compatible with keyword '%s'")
                   % (pycompat.sysbytes(filt.__name__), sym))
        else:
            msg = (_("incompatible use of template filter '%s'")
                   % pycompat.sysbytes(filt.__name__))
        raise error.Abort(msg)

def runmap(context, mapping, data):
    darg, targ = data
    d = evalrawexp(context, mapping, darg)
    if util.safehasattr(d, 'itermaps'):
        diter = d.itermaps()
    else:
        try:
            diter = iter(d)
        except TypeError:
            sym = findsymbolicname(darg)
            if sym:
                raise error.ParseError(_("keyword '%s' is not iterable") % sym)
            else:
                raise error.ParseError(_("%r is not iterable") % d)

    for i, v in enumerate(diter):
        if isinstance(v, dict):
            lm = context.overlaymap(mapping, v)
            lm['index'] = i
            yield evalrawexp(context, lm, targ)
        else:
            # v is not an iterable of dicts, this happen when 'key'
            # has been fully expanded already and format is useless.
            # If so, return the expanded value.
            yield v

def runmember(context, mapping, data):
    darg, memb = data
    d = evalrawexp(context, mapping, darg)
    if util.safehasattr(d, 'tomap'):
        lm = context.overlaymap(mapping, d.tomap())
        return runsymbol(context, lm, memb)
    if util.safehasattr(d, 'get'):
        return getdictitem(d, memb)

    sym = findsymbolicname(darg)
    if sym:
        raise error.ParseError(_("keyword '%s' has no member") % sym)
    else:
        raise error.ParseError(_("%r has no member") % pycompat.bytestr(d))

def runnegate(context, mapping, data):
    data = evalinteger(context, mapping, data,
                       _('negation needs an integer argument'))
    return -data

def runarithmetic(context, mapping, data):
    func, left, right = data
    left = evalinteger(context, mapping, left,
                       _('arithmetic only defined on integers'))
    right = evalinteger(context, mapping, right,
                        _('arithmetic only defined on integers'))
    try:
        return func(left, right)
    except ZeroDivisionError:
        raise error.Abort(_('division by zero is not defined'))

def getdictitem(dictarg, key):
    val = dictarg.get(key)
    if val is None:
        return
    return wraphybridvalue(dictarg, key, val)
