// ancestors.rs
//
// Copyright 2018 Georges Racinet <gracinet@anybox.fr>
//
// This software may be used and distributed according to the terms of the
// GNU General Public License version 2 or any later version.

//! Rust versions of generic DAG ancestors algorithms for Mercurial

use super::{Graph, GraphError, Revision, NULL_REVISION};
use std::cmp::max;
use std::collections::{BinaryHeap, HashSet};

/// Iterator over the ancestors of a given list of revisions
/// This is a generic type, defined and implemented for any Graph, so that
/// it's easy to
///
/// - unit test in pure Rust
/// - bind to main Mercurial code, potentially in several ways and have these
///   bindings evolve over time
pub struct AncestorsIterator<G: Graph> {
    graph: G,
    visit: BinaryHeap<Revision>,
    seen: HashSet<Revision>,
    stoprev: Revision,
}

/// Lazy ancestors set, backed by AncestorsIterator
pub struct LazyAncestors<G: Graph + Clone> {
    graph: G,
    containsiter: AncestorsIterator<G>,
    initrevs: Vec<Revision>,
    stoprev: Revision,
    inclusive: bool,
}

pub struct MissingAncestors<G: Graph> {
    graph: G,
    bases: HashSet<Revision>,
}

impl<G: Graph> AncestorsIterator<G> {
    /// Constructor.
    ///
    /// if `inclusive` is true, then the init revisions are emitted in
    /// particular, otherwise iteration starts from their parents.
    pub fn new(
        graph: G,
        initrevs: impl IntoIterator<Item = Revision>,
        stoprev: Revision,
        inclusive: bool,
    ) -> Result<Self, GraphError> {
        let filtered_initrevs = initrevs.into_iter().filter(|&r| r >= stoprev);
        if inclusive {
            let visit: BinaryHeap<Revision> = filtered_initrevs.collect();
            let seen = visit.iter().map(|&x| x).collect();
            return Ok(AncestorsIterator {
                visit: visit,
                seen: seen,
                stoprev: stoprev,
                graph: graph,
            });
        }
        let mut this = AncestorsIterator {
            visit: BinaryHeap::new(),
            seen: HashSet::new(),
            stoprev: stoprev,
            graph: graph,
        };
        this.seen.insert(NULL_REVISION);
        for rev in filtered_initrevs {
            for parent in this.graph.parents(rev)?.iter().cloned() {
                this.conditionally_push_rev(parent);
            }
        }
        Ok(this)
    }

    #[inline]
    fn conditionally_push_rev(&mut self, rev: Revision) {
        if self.stoprev <= rev && !self.seen.contains(&rev) {
            self.seen.insert(rev);
            self.visit.push(rev);
        }
    }

    /// Consumes partially the iterator to tell if the given target
    /// revision
    /// is in the ancestors it emits.
    /// This is meant for iterators actually dedicated to that kind of
    /// purpose
    pub fn contains(&mut self, target: Revision) -> Result<bool, GraphError> {
        if self.seen.contains(&target) && target != NULL_REVISION {
            return Ok(true);
        }
        for item in self {
            let rev = item?;
            if rev == target {
                return Ok(true);
            }
            if rev < target {
                return Ok(false);
            }
        }
        Ok(false)
    }

    pub fn peek(&self) -> Option<Revision> {
        self.visit.peek().map(|&r| r)
    }

    /// Tell if the iterator is about an empty set
    ///
    /// The result does not depend whether the iterator has been consumed
    /// or not.
    /// This is mostly meant for iterators backing a lazy ancestors set
    pub fn is_empty(&self) -> bool {
        if self.visit.len() > 0 {
            return false;
        }
        if self.seen.len() > 1 {
            return false;
        }
        // at this point, the seen set is at most a singleton.
        // If not `self.inclusive`, it's still possible that it has only
        // the null revision
        self.seen.is_empty() || self.seen.contains(&NULL_REVISION)
    }
}

/// Main implementation for the iterator
///
/// The algorithm is the same as in `_lazyancestorsiter()` from `ancestors.py`
/// with a few non crucial differences:
///
/// - there's no filtering of invalid parent revisions. Actually, it should be
///   consistent and more efficient to filter them from the end caller.
/// - we don't have the optimization for adjacent revisions (i.e., the case
///   where `p1 == rev - 1`), because it amounts to update the first element of
///   the heap without sifting, which Rust's BinaryHeap doesn't let us do.
/// - we save a few pushes by comparing with `stoprev` before pushing
impl<G: Graph> Iterator for AncestorsIterator<G> {
    type Item = Result<Revision, GraphError>;

    fn next(&mut self) -> Option<Self::Item> {
        let current = match self.visit.peek() {
            None => {
                return None;
            }
            Some(c) => *c,
        };
        let [p1, p2] = match self.graph.parents(current) {
            Ok(ps) => ps,
            Err(e) => return Some(Err(e)),
        };
        if p1 < self.stoprev || self.seen.contains(&p1) {
            self.visit.pop();
        } else {
            *(self.visit.peek_mut().unwrap()) = p1;
            self.seen.insert(p1);
        };

        self.conditionally_push_rev(p2);
        Some(Ok(current))
    }
}

impl<G: Graph + Clone> LazyAncestors<G> {
    pub fn new(
        graph: G,
        initrevs: impl IntoIterator<Item = Revision>,
        stoprev: Revision,
        inclusive: bool,
    ) -> Result<Self, GraphError> {
        let v: Vec<Revision> = initrevs.into_iter().collect();
        Ok(LazyAncestors {
            graph: graph.clone(),
            containsiter: AncestorsIterator::new(
                graph,
                v.iter().cloned(),
                stoprev,
                inclusive,
            )?,
            initrevs: v,
            stoprev: stoprev,
            inclusive: inclusive,
        })
    }

    pub fn contains(&mut self, rev: Revision) -> Result<bool, GraphError> {
        self.containsiter.contains(rev)
    }

    pub fn is_empty(&self) -> bool {
        self.containsiter.is_empty()
    }

    pub fn iter(&self) -> AncestorsIterator<G> {
        // the arguments being the same as for self.containsiter, we know
        // for sure that AncestorsIterator constructor can't fail
        AncestorsIterator::new(
            self.graph.clone(),
            self.initrevs.iter().cloned(),
            self.stoprev,
            self.inclusive,
        )
        .unwrap()
    }
}

impl<G: Graph> MissingAncestors<G> {
    pub fn new(graph: G, bases: impl IntoIterator<Item = Revision>) -> Self {
        let mut bases: HashSet<Revision> = bases.into_iter().collect();
        if bases.is_empty() {
            bases.insert(NULL_REVISION);
        }
        MissingAncestors { graph, bases }
    }

    pub fn has_bases(&self) -> bool {
        self.bases.iter().any(|&b| b != NULL_REVISION)
    }

    /// Return a reference to current bases.
    ///
    /// This is useful in unit tests, but also setdiscovery.py does
    /// read the bases attribute of a ancestor.missingancestors instance.
    pub fn get_bases<'a>(&'a self) -> &'a HashSet<Revision> {
        &self.bases
    }

    pub fn add_bases(
        &mut self,
        new_bases: impl IntoIterator<Item = Revision>,
    ) {
        self.bases.extend(new_bases);
    }

    /// Remove all ancestors of self.bases from the revs set (in place)
    pub fn remove_ancestors_from(
        &mut self,
        revs: &mut HashSet<Revision>,
    ) -> Result<(), GraphError> {
        revs.retain(|r| !self.bases.contains(r));
        // the null revision is always an ancestor
        revs.remove(&NULL_REVISION);
        if revs.is_empty() {
            return Ok(());
        }
        // anything in revs > start is definitely not an ancestor of bases
        // revs <= start need to be investigated
        // TODO optim: if a missingancestors is to be used several times,
        // we shouldn't need to iterate each time on bases
        let start = match self.bases.iter().cloned().max() {
            Some(m) => m,
            None => {
                // bases is empty (shouldn't happen, but let's be safe)
                return Ok(());
            }
        };
        // whatever happens, we'll keep at least keepcount of them
        // knowing this gives us a earlier stop condition than
        // going all the way to the root
        let keepcount = revs.iter().filter(|r| **r > start).count();

        let mut curr = start;
        while curr != NULL_REVISION && revs.len() > keepcount {
            if self.bases.contains(&curr) {
                revs.remove(&curr);
                self.add_parents(curr)?;
            }
            curr -= 1;
        }
        Ok(())
    }

    /// Add rev's parents to self.bases
    #[inline]
    fn add_parents(&mut self, rev: Revision) -> Result<(), GraphError> {
        // No need to bother the set with inserting NULL_REVISION over and
        // over
        for p in self.graph.parents(rev)?.iter().cloned() {
            if p != NULL_REVISION {
                self.bases.insert(p);
            }
        }
        Ok(())
    }

    /// Return all the ancestors of revs that are not ancestors of self.bases
    ///
    /// This may include elements from revs.
    ///
    /// Equivalent to the revset (::revs - ::self.bases). Revs are returned in
    /// revision number order, which is a topological order.
    pub fn missing_ancestors(
        &mut self,
        revs: impl IntoIterator<Item = Revision>,
    ) -> Result<Vec<Revision>, GraphError> {
        // just for convenience and comparison with Python version
        let bases_visit = &mut self.bases;
        let mut revs: HashSet<Revision> = revs
            .into_iter()
            .filter(|r| !bases_visit.contains(r))
            .collect();
        let revs_visit = &mut revs;
        let mut both_visit: HashSet<Revision> =
            revs_visit.intersection(&bases_visit).cloned().collect();
        if revs_visit.is_empty() {
            return Ok(Vec::new());
        }

        let max_bases =
            bases_visit.iter().cloned().max().unwrap_or(NULL_REVISION);
        let max_revs =
            revs_visit.iter().cloned().max().unwrap_or(NULL_REVISION);
        let start = max(max_bases, max_revs);

        // TODO heuristics for with_capacity()?
        let mut missing: Vec<Revision> = Vec::new();
        for curr in (0..=start).rev() {
            if revs_visit.is_empty() {
                break;
            }
            if both_visit.contains(&curr) {
                // curr's parents might have made it into revs_visit through
                // another path
                // TODO optim: Rust's HashSet.remove returns a boolean telling
                // if it happened. This will spare us one set lookup
                both_visit.remove(&curr);
                for p in self.graph.parents(curr)?.iter().cloned() {
                    if p == NULL_REVISION {
                        continue;
                    }
                    revs_visit.remove(&p);
                    bases_visit.insert(p);
                    both_visit.insert(p);
                }
                continue;
            }
            // in Rust, one can't just use mutable variables assignation
            // to be more straightforward. Instead of Python's
            // thisvisit and othervisit, we'll differentiate with a boolean
            let this_visit_is_revs = {
                if revs_visit.remove(&curr) {
                    missing.push(curr);
                    true
                } else if bases_visit.contains(&curr) {
                    false
                } else {
                    // not an ancestor of revs or bases: ignore
                    continue;
                }
            };

            for p in self.graph.parents(curr)?.iter().cloned() {
                if p == NULL_REVISION {
                    continue;
                }
                let in_other_visit = if this_visit_is_revs {
                    bases_visit.contains(&p)
                } else {
                    revs_visit.contains(&p)
                };
                if in_other_visit || both_visit.contains(&p) {
                    // p is implicitely in this_visit.
                    // This means p is or should be in bothvisit
                    // TODO optim: hence if bothvisit, we look up twice
                    revs_visit.remove(&p);
                    bases_visit.insert(p);
                    both_visit.insert(p);
                } else {
                    // visit later
                    if this_visit_is_revs {
                        revs_visit.insert(p);
                    } else {
                        bases_visit.insert(p);
                    }
                }
            }
        }
        missing.reverse();
        Ok(missing)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::iter::FromIterator;

    #[derive(Clone, Debug)]
    struct Stub;

    /// This is the same as the dict from test-ancestors.py
    impl Graph for Stub {
        fn parents(&self, rev: Revision) -> Result<[Revision; 2], GraphError> {
            match rev {
                0 => Ok([-1, -1]),
                1 => Ok([0, -1]),
                2 => Ok([1, -1]),
                3 => Ok([1, -1]),
                4 => Ok([2, -1]),
                5 => Ok([4, -1]),
                6 => Ok([4, -1]),
                7 => Ok([4, -1]),
                8 => Ok([-1, -1]),
                9 => Ok([6, 7]),
                10 => Ok([5, -1]),
                11 => Ok([3, 7]),
                12 => Ok([9, -1]),
                13 => Ok([8, -1]),
                r => Err(GraphError::ParentOutOfRange(r)),
            }
        }
    }

    fn list_ancestors<G: Graph>(
        graph: G,
        initrevs: Vec<Revision>,
        stoprev: Revision,
        inclusive: bool,
    ) -> Vec<Revision> {
        AncestorsIterator::new(graph, initrevs, stoprev, inclusive)
            .unwrap()
            .map(|res| res.unwrap())
            .collect()
    }

    #[test]
    /// Same tests as test-ancestor.py, without membership
    /// (see also test-ancestor.py.out)
    fn test_list_ancestor() {
        assert_eq!(list_ancestors(Stub, vec![], 0, false), vec![]);
        assert_eq!(
            list_ancestors(Stub, vec![11, 13], 0, false),
            vec![8, 7, 4, 3, 2, 1, 0]
        );
        assert_eq!(list_ancestors(Stub, vec![1, 3], 0, false), vec![1, 0]);
        assert_eq!(
            list_ancestors(Stub, vec![11, 13], 0, true),
            vec![13, 11, 8, 7, 4, 3, 2, 1, 0]
        );
        assert_eq!(list_ancestors(Stub, vec![11, 13], 6, false), vec![8, 7]);
        assert_eq!(
            list_ancestors(Stub, vec![11, 13], 6, true),
            vec![13, 11, 8, 7]
        );
        assert_eq!(list_ancestors(Stub, vec![11, 13], 11, true), vec![13, 11]);
        assert_eq!(list_ancestors(Stub, vec![11, 13], 12, true), vec![13]);
        assert_eq!(
            list_ancestors(Stub, vec![10, 1], 0, true),
            vec![10, 5, 4, 2, 1, 0]
        );
    }

    #[test]
    /// Corner case that's not directly in test-ancestors.py, but
    /// that happens quite often, as demonstrated by running the whole
    /// suite.
    /// For instance, run tests/test-obsolete-checkheads.t
    fn test_nullrev_input() {
        let mut iter =
            AncestorsIterator::new(Stub, vec![-1], 0, false).unwrap();
        assert_eq!(iter.next(), None)
    }

    #[test]
    fn test_contains() {
        let mut lazy =
            AncestorsIterator::new(Stub, vec![10, 1], 0, true).unwrap();
        assert!(lazy.contains(1).unwrap());
        assert!(!lazy.contains(3).unwrap());

        let mut lazy =
            AncestorsIterator::new(Stub, vec![0], 0, false).unwrap();
        assert!(!lazy.contains(NULL_REVISION).unwrap());
    }

    #[test]
    fn test_peek() {
        let mut iter =
            AncestorsIterator::new(Stub, vec![10], 0, true).unwrap();
        // peek() gives us the next value
        assert_eq!(iter.peek(), Some(10));
        // but it's not been consumed
        assert_eq!(iter.next(), Some(Ok(10)));
        // and iteration resumes normally
        assert_eq!(iter.next(), Some(Ok(5)));

        // let's drain the iterator to test peek() at the end
        while iter.next().is_some() {}
        assert_eq!(iter.peek(), None);
    }

    #[test]
    fn test_empty() {
        let mut iter =
            AncestorsIterator::new(Stub, vec![10], 0, true).unwrap();
        assert!(!iter.is_empty());
        while iter.next().is_some() {}
        assert!(!iter.is_empty());

        let iter = AncestorsIterator::new(Stub, vec![], 0, true).unwrap();
        assert!(iter.is_empty());

        // case where iter.seen == {NULL_REVISION}
        let iter = AncestorsIterator::new(Stub, vec![0], 0, false).unwrap();
        assert!(iter.is_empty());
    }

    /// A corrupted Graph, supporting error handling tests
    #[derive(Clone, Debug)]
    struct Corrupted;

    impl Graph for Corrupted {
        fn parents(&self, rev: Revision) -> Result<[Revision; 2], GraphError> {
            match rev {
                1 => Ok([0, -1]),
                r => Err(GraphError::ParentOutOfRange(r)),
            }
        }
    }

    #[test]
    fn test_initrev_out_of_range() {
        // inclusive=false looks up initrev's parents right away
        match AncestorsIterator::new(Stub, vec![25], 0, false) {
            Ok(_) => panic!("Should have been ParentOutOfRange"),
            Err(e) => assert_eq!(e, GraphError::ParentOutOfRange(25)),
        }
    }

    #[test]
    fn test_next_out_of_range() {
        // inclusive=false looks up initrev's parents right away
        let mut iter =
            AncestorsIterator::new(Corrupted, vec![1], 0, false).unwrap();
        assert_eq!(iter.next(), Some(Err(GraphError::ParentOutOfRange(0))));
    }

    #[test]
    fn test_lazy_iter_contains() {
        let mut lazy =
            LazyAncestors::new(Stub, vec![11, 13], 0, false).unwrap();

        let revs: Vec<Revision> = lazy.iter().map(|r| r.unwrap()).collect();
        // compare with iterator tests on the same initial revisions
        assert_eq!(revs, vec![8, 7, 4, 3, 2, 1, 0]);

        // contains() results are correct, unaffected by the fact that
        // we consumed entirely an iterator out of lazy
        assert_eq!(lazy.contains(2), Ok(true));
        assert_eq!(lazy.contains(9), Ok(false));
    }

    #[test]
    fn test_lazy_contains_iter() {
        let mut lazy =
            LazyAncestors::new(Stub, vec![11, 13], 0, false).unwrap(); // reminder: [8, 7, 4, 3, 2, 1, 0]

        assert_eq!(lazy.contains(2), Ok(true));
        assert_eq!(lazy.contains(6), Ok(false));

        // after consumption of 2 by the inner iterator, results stay
        // consistent
        assert_eq!(lazy.contains(2), Ok(true));
        assert_eq!(lazy.contains(5), Ok(false));

        // iter() still gives us a fresh iterator
        let revs: Vec<Revision> = lazy.iter().map(|r| r.unwrap()).collect();
        assert_eq!(revs, vec![8, 7, 4, 3, 2, 1, 0]);
    }

    #[test]
    /// Test constructor, add/get bases
    fn test_missing_bases() {
        let mut missing_ancestors =
            MissingAncestors::new(Stub, [5, 3, 1, 3].iter().cloned());
        let mut as_vec: Vec<Revision> =
            missing_ancestors.get_bases().iter().cloned().collect();
        as_vec.sort();
        assert_eq!(as_vec, [1, 3, 5]);

        missing_ancestors.add_bases([3, 7, 8].iter().cloned());
        as_vec = missing_ancestors.get_bases().iter().cloned().collect();
        as_vec.sort();
        assert_eq!(as_vec, [1, 3, 5, 7, 8]);
    }

    fn assert_missing_remove(
        bases: &[Revision],
        revs: &[Revision],
        expected: &[Revision],
    ) {
        let mut missing_ancestors =
            MissingAncestors::new(Stub, bases.iter().cloned());
        let mut revset: HashSet<Revision> = revs.iter().cloned().collect();
        missing_ancestors
            .remove_ancestors_from(&mut revset)
            .unwrap();
        let mut as_vec: Vec<Revision> = revset.into_iter().collect();
        as_vec.sort();
        assert_eq!(as_vec.as_slice(), expected);
    }

    #[test]
    fn test_missing_remove() {
        assert_missing_remove(
            &[1, 2, 3, 4, 7],
            Vec::from_iter(1..10).as_slice(),
            &[5, 6, 8, 9],
        );
        assert_missing_remove(&[10], &[11, 12, 13, 14], &[11, 12, 13, 14]);
        assert_missing_remove(&[7], &[1, 2, 3, 4, 5], &[3, 5]);
    }

    fn assert_missing_ancestors(
        bases: &[Revision],
        revs: &[Revision],
        expected: &[Revision],
    ) {
        let mut missing_ancestors =
            MissingAncestors::new(Stub, bases.iter().cloned());
        let missing = missing_ancestors
            .missing_ancestors(revs.iter().cloned())
            .unwrap();
        assert_eq!(missing.as_slice(), expected);
    }

    #[test]
    fn test_missing_ancestors() {
        // examples taken from test-ancestors.py by having it run
        // on the same graph (both naive and fast Python algs)
        assert_missing_ancestors(&[10], &[11], &[3, 7, 11]);
        assert_missing_ancestors(&[11], &[10], &[5, 10]);
        assert_missing_ancestors(&[7], &[9, 11], &[3, 6, 9, 11]);
    }

    // A Graph represented by a vector whose indices are revisions
    // and values are parents of the revisions
    type VecGraph = Vec<[Revision; 2]>;

    impl Graph for VecGraph {
        fn parents(&self, rev: Revision) -> Result<[Revision; 2], GraphError> {
            Ok(self[rev as usize])
        }
    }

    /// An interesting case found by a random generator similar to
    /// the one in test-ancestor.py. An early version of Rust MissingAncestors
    /// failed this, yet none of the integration tests of the whole suite
    /// catched it.
    #[test]
    fn test_remove_ancestors_from_case1() {
        let graph: VecGraph = vec![
            [NULL_REVISION, NULL_REVISION],
            [0, NULL_REVISION],
            [1, 0],
            [2, 1],
            [3, NULL_REVISION],
            [4, NULL_REVISION],
            [5, 1],
            [2, NULL_REVISION],
            [7, NULL_REVISION],
            [8, NULL_REVISION],
            [9, NULL_REVISION],
            [10, 1],
            [3, NULL_REVISION],
            [12, NULL_REVISION],
            [13, NULL_REVISION],
            [14, NULL_REVISION],
            [4, NULL_REVISION],
            [16, NULL_REVISION],
            [17, NULL_REVISION],
            [18, NULL_REVISION],
            [19, 11],
            [20, NULL_REVISION],
            [21, NULL_REVISION],
            [22, NULL_REVISION],
            [23, NULL_REVISION],
            [2, NULL_REVISION],
            [3, NULL_REVISION],
            [26, 24],
            [27, NULL_REVISION],
            [28, NULL_REVISION],
            [12, NULL_REVISION],
            [1, NULL_REVISION],
            [1, 9],
            [32, NULL_REVISION],
            [33, NULL_REVISION],
            [34, 31],
            [35, NULL_REVISION],
            [36, 26],
            [37, NULL_REVISION],
            [38, NULL_REVISION],
            [39, NULL_REVISION],
            [40, NULL_REVISION],
            [41, NULL_REVISION],
            [42, 26],
            [0, NULL_REVISION],
            [44, NULL_REVISION],
            [45, 4],
            [40, NULL_REVISION],
            [47, NULL_REVISION],
            [36, 0],
            [49, NULL_REVISION],
            [NULL_REVISION, NULL_REVISION],
            [51, NULL_REVISION],
            [52, NULL_REVISION],
            [53, NULL_REVISION],
            [14, NULL_REVISION],
            [55, NULL_REVISION],
            [15, NULL_REVISION],
            [23, NULL_REVISION],
            [58, NULL_REVISION],
            [59, NULL_REVISION],
            [2, NULL_REVISION],
            [61, 59],
            [62, NULL_REVISION],
            [63, NULL_REVISION],
            [NULL_REVISION, NULL_REVISION],
            [65, NULL_REVISION],
            [66, NULL_REVISION],
            [67, NULL_REVISION],
            [68, NULL_REVISION],
            [37, 28],
            [69, 25],
            [71, NULL_REVISION],
            [72, NULL_REVISION],
            [50, 2],
            [74, NULL_REVISION],
            [12, NULL_REVISION],
            [18, NULL_REVISION],
            [77, NULL_REVISION],
            [78, NULL_REVISION],
            [79, NULL_REVISION],
            [43, 33],
            [81, NULL_REVISION],
            [82, NULL_REVISION],
            [83, NULL_REVISION],
            [84, 45],
            [85, NULL_REVISION],
            [86, NULL_REVISION],
            [NULL_REVISION, NULL_REVISION],
            [88, NULL_REVISION],
            [NULL_REVISION, NULL_REVISION],
            [76, 83],
            [44, NULL_REVISION],
            [92, NULL_REVISION],
            [93, NULL_REVISION],
            [9, NULL_REVISION],
            [95, 67],
            [96, NULL_REVISION],
            [97, NULL_REVISION],
            [NULL_REVISION, NULL_REVISION],
        ];
        let problem_rev = 28 as Revision;
        let problem_base = 70 as Revision;
        // making the problem obvious: problem_rev is a parent of problem_base
        assert_eq!(graph.parents(problem_base).unwrap()[1], problem_rev);

        let mut missing_ancestors: MissingAncestors<VecGraph> =
            MissingAncestors::new(
                graph,
                [60, 26, 70, 3, 96, 19, 98, 49, 97, 47, 1, 6]
                    .iter()
                    .cloned(),
            );
        assert!(missing_ancestors.bases.contains(&problem_base));

        let mut revs: HashSet<Revision> =
            [4, 12, 41, 28, 68, 38, 1, 30, 56, 44]
                .iter()
                .cloned()
                .collect();
        missing_ancestors.remove_ancestors_from(&mut revs).unwrap();
        assert!(!revs.contains(&problem_rev));
    }

}
