  $ cat << EOF >> $HGRCPATH
  > [format]
  > usegeneraldelta=yes
  > EOF

  $ hg init debugrevlog
  $ cd debugrevlog
  $ echo a > a
  $ hg ci -Am adda
  adding a
  $ hg debugrevlog -m
  format : 1
  flags  : inline, generaldelta
  
  revisions     :  1
      merges    :  0 ( 0.00%)
      normal    :  1 (100.00%)
  revisions     :  1
      full      :  1 (100.00%)
      deltas    :  0 ( 0.00%)
  revision size : 44
      full      : 44 (100.00%)
      deltas    :  0 ( 0.00%)
  
  chunks        :  1
      0x75 (u)  :  1 (100.00%)
  chunks size   : 44
      0x75 (u)  : 44 (100.00%)
  
  avg chain length  :  0
  max chain length  :  0
  max chain reach   : 44
  compression ratio :  0
  
  uncompressed data size (min/max/avg) : 43 / 43 / 43
  full revision size (min/max/avg)     : 44 / 44 / 44
  delta size (min/max/avg)             : 0 / 0 / 0

Test debugindex, with and without the --debug flag
  $ hg debugindex a
     rev    offset  length  ..... linkrev nodeid       p1           p2 (re)
       0         0       3   ....       0 b789fdd96dc2 000000000000 000000000000 (re)
  $ hg --debug debugindex a
     rev    offset  length  ..... linkrev nodeid                                   p1                                       p2 (re)
       0         0       3   ....       0 b789fdd96dc2f3bd229c1dd8eedf0fc60e2b68e3 0000000000000000000000000000000000000000 0000000000000000000000000000000000000000 (re)
  $ hg debugindex -f 1 a
     rev flag   offset   length     size  .....   link     p1     p2       nodeid (re)
       0 0000        0        3        2   ....      0     -1     -1 b789fdd96dc2 (re)
  $ hg --debug debugindex -f 1 a
     rev flag   offset   length     size  .....   link     p1     p2                                   nodeid (re)
       0 0000        0        3        2   ....      0     -1     -1 b789fdd96dc2f3bd229c1dd8eedf0fc60e2b68e3 (re)

debugdelta chain basic output

  $ hg debugdeltachain -m
      rev  chain# chainlen     prev   delta       size    rawsize  chainsize     ratio   lindist extradist extraratio
        0       1        1       -1    base         44         43         44   1.02326        44         0    0.00000

  $ hg debugdeltachain -m -T '{rev} {chainid} {chainlen}\n'
  0 1 1

  $ hg debugdeltachain -m -Tjson
  [
   {
    "chainid": 1,
    "chainlen": 1,
    "chainratio": 1.02325581395,
    "chainsize": 44,
    "compsize": 44,
    "deltatype": "base",
    "extradist": 0,
    "extraratio": 0.0,
    "lindist": 44,
    "prevrev": -1,
    "rev": 0,
    "uncompsize": 43
   }
  ]

debugdelta chain with sparse read enabled

  $ cat >> $HGRCPATH <<EOF
  > [experimental]
  > sparse-read = True
  > EOF
  $ hg debugdeltachain -m
      rev  chain# chainlen     prev   delta       size    rawsize  chainsize     ratio   lindist extradist extraratio   readsize largestblk rddensity
        0       1        1       -1    base         44         43         44   1.02326        44         0    0.00000         44         44   1.00000

  $ hg debugdeltachain -m -T '{rev} {chainid} {chainlen} {readsize} {largestblock} {readdensity}\n'
  0 1 1 44 44 1.0

  $ hg debugdeltachain -m -Tjson
  [
   {
    "chainid": 1,
    "chainlen": 1,
    "chainratio": 1.02325581395,
    "chainsize": 44,
    "compsize": 44,
    "deltatype": "base",
    "extradist": 0,
    "extraratio": 0.0,
    "largestblock": 44,
    "lindist": 44,
    "prevrev": -1,
    "readdensity": 1.0,
    "readsize": 44,
    "rev": 0,
    "uncompsize": 43
   }
  ]

Test max chain len
  $ cat >> $HGRCPATH << EOF
  > [format]
  > maxchainlen=4
  > EOF

  $ printf "This test checks if maxchainlen config value is respected also it can serve as basic test for debugrevlog -d <file>.\n" >> a
  $ hg ci -m a
  $ printf "b\n" >> a
  $ hg ci -m a
  $ printf "c\n" >> a
  $ hg ci -m a
  $ printf "d\n" >> a
  $ hg ci -m a
  $ printf "e\n" >> a
  $ hg ci -m a
  $ printf "f\n" >> a
  $ hg ci -m a
  $ printf 'g\n' >> a
  $ hg ci -m a
  $ printf 'h\n' >> a
  $ hg ci -m a
  $ hg debugrevlog -d a
  # rev p1rev p2rev start   end deltastart base   p1   p2 rawsize totalsize compression heads chainlen
      0    -1    -1     0   ???          0    0    0    0     ???      ????           ?     1        0 (glob)
      1     0    -1   ???   ???          0    0    0    0     ???      ????           ?     1        1 (glob)
      2     1    -1   ???   ???        ???  ???  ???    0     ???      ????           ?     1        2 (glob)
      3     2    -1   ???   ???        ???  ???  ???    0     ???      ????           ?     1        3 (glob)
      4     3    -1   ???   ???        ???  ???  ???    0     ???      ????           ?     1        4 (glob)
      5     4    -1   ???   ???        ???  ???  ???    0     ???      ????           ?     1        0 (glob)
      6     5    -1   ???   ???        ???  ???  ???    0     ???      ????           ?     1        1 (glob)
      7     6    -1   ???   ???        ???  ???  ???    0     ???      ????           ?     1        2 (glob)
      8     7    -1   ???   ???        ???  ???  ???    0     ???      ????           ?     1        3 (glob)

Test WdirUnsupported exception

  $ hg debugdata -c ffffffffffffffffffffffffffffffffffffffff
  abort: working directory revision cannot be specified
  [255]

Test cache warming command

  $ rm -rf .hg/cache/
  $ hg debugupdatecaches --debug
  updating the branch cache
  $ ls -r .hg/cache/*
  .hg/cache/rbc-revs-v1
  .hg/cache/rbc-names-v1
  .hg/cache/branch2-served

  $ cd ..

Test internal debugstacktrace command

  $ cat > debugstacktrace.py << EOF
  > from __future__ import absolute_import
  > import sys
  > from mercurial import util
  > def f():
  >     util.debugstacktrace(f=sys.stdout)
  >     g()
  > def g():
  >     util.dst('hello from g\\n', skip=1)
  >     h()
  > def h():
  >     util.dst('hi ...\\nfrom h hidden in g', 1, depth=2)
  > f()
  > EOF
  $ $PYTHON debugstacktrace.py
  stacktrace at:
   debugstacktrace.py:12 in * (glob)
   debugstacktrace.py:5  in f
  hello from g at:
   debugstacktrace.py:12 in * (glob)
   debugstacktrace.py:6  in f
  hi ...
  from h hidden in g at:
   debugstacktrace.py:6 in f
   debugstacktrace.py:9 in g

Test debugcapabilities command:

  $ hg debugcapabilities ./debugrevlog/
  Main capabilities:
    branchmap
    bundle2=HG20%0Achangegroup%3D01%2C02%0Adigests%3Dmd5%2Csha1%2Csha512%0Aerror%3Dabort%2Cunsupportedcontent%2Cpushraced%2Cpushkey%0Ahgtagsfnodes%0Alistkeys%0Aphases%3Dheads%0Apushkey%0Aremote-changegroup%3Dhttp%2Chttps
    getbundle
    known
    lookup
    pushkey
    unbundle
  Bundle2 capabilities:
    HG20
    changegroup
      01
      02
    digests
      md5
      sha1
      sha512
    error
      abort
      unsupportedcontent
      pushraced
      pushkey
    hgtagsfnodes
    listkeys
    phases
      heads
    pushkey
    remote-changegroup
      http
      https
