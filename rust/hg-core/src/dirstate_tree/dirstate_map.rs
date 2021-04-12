use std::collections::BTreeMap;
use std::path::PathBuf;

use super::path_with_basename::WithBasename;
use crate::dirstate::parsers::parse_dirstate_entries;
use crate::dirstate::parsers::parse_dirstate_parents;
use crate::dirstate::parsers::Timestamp;

use crate::matchers::Matcher;
use crate::revlog::node::NULL_NODE;
use crate::utils::hg_path::{HgPath, HgPathBuf};
use crate::CopyMapIter;
use crate::DirstateEntry;
use crate::DirstateError;
use crate::DirstateMapError;
use crate::DirstateParents;
use crate::DirstateStatus;
use crate::EntryState;
use crate::FastHashMap;
use crate::HgPathCow;
use crate::PatternFileWarning;
use crate::StateMapIter;
use crate::StatusError;
use crate::StatusOptions;

pub struct DirstateMap {
    parents: Option<DirstateParents>,
    dirty_parents: bool,
    root: ChildNodes,
}

/// Using a plain `HgPathBuf` of the full path from the repository root as a
/// map key would also work: all paths in a given map have the same parent
/// path, so comparing full paths gives the same result as comparing base
/// names. However `BTreeMap` would waste time always re-comparing the same
/// string prefix.
type ChildNodes = BTreeMap<WithBasename<HgPathBuf>, Node>;

#[derive(Default)]
struct Node {
    entry: Option<DirstateEntry>,
    copy_source: Option<HgPathBuf>,
    children: ChildNodes,
}

/// `(full_path, entry, copy_source)`
type NodeDataMut<'a> = (
    &'a WithBasename<HgPathBuf>,
    &'a mut Option<DirstateEntry>,
    &'a mut Option<HgPathBuf>,
);

impl DirstateMap {
    pub fn new() -> Self {
        Self {
            parents: None,
            dirty_parents: false,
            root: ChildNodes::new(),
        }
    }

    fn get_node(&self, path: &HgPath) -> Option<&Node> {
        let mut children = &self.root;
        let mut components = path.components();
        let mut component =
            components.next().expect("expected at least one components");
        loop {
            let child = children.get(component)?;
            if let Some(next_component) = components.next() {
                component = next_component;
                children = &child.children;
            } else {
                return Some(child);
            }
        }
    }

    fn get_or_insert_node(&mut self, path: &HgPath) -> &mut Node {
        let mut child_nodes = &mut self.root;
        let mut inclusive_ancestor_paths =
            WithBasename::inclusive_ancestors_of(path);
        let mut ancestor_path = inclusive_ancestor_paths
            .next()
            .expect("expected at least one inclusive ancestor");
        loop {
            // TODO: can we avoid double lookup in all cases without allocating
            // an owned key in cases where the map already contains that key?
            let child_node =
                if child_nodes.contains_key(ancestor_path.base_name()) {
                    child_nodes.get_mut(ancestor_path.base_name()).unwrap()
                } else {
                    // This is always a vacant entry, using `.entry()` lets us
                    // return a `&mut Node` of the newly-inserted node without
                    // yet another lookup. `BTreeMap::insert` doesn’t do this.
                    child_nodes.entry(ancestor_path.to_owned()).or_default()
                };
            if let Some(next) = inclusive_ancestor_paths.next() {
                ancestor_path = next;
                child_nodes = &mut child_node.children;
            } else {
                return child_node;
            }
        }
    }

    fn iter_nodes<'a>(
        &'a self,
    ) -> impl Iterator<Item = (&'a WithBasename<HgPathBuf>, &'a Node)> + 'a
    {
        // Depth first tree traversal.
        //
        // If we could afford internal iteration and recursion,
        // this would look like:
        //
        // ```
        // fn traverse_children(
        //     children: &ChildNodes,
        //     each: &mut impl FnMut(&Node),
        // ) {
        //     for child in children.values() {
        //         traverse_children(&child.children, each);
        //         each(child);
        //     }
        // }
        // ```
        //
        // However we want an external iterator and therefore can’t use the
        // call stack. Use an explicit stack instead:
        let mut stack = Vec::new();
        let mut iter = self.root.iter();
        std::iter::from_fn(move || {
            while let Some((key, child_node)) = iter.next() {
                // Pseudo-recursion
                let new_iter = child_node.children.iter();
                let old_iter = std::mem::replace(&mut iter, new_iter);
                stack.push((key, child_node, old_iter));
            }
            // Found the end of a `children.iter()` iterator.
            if let Some((key, child_node, next_iter)) = stack.pop() {
                // "Return" from pseudo-recursion by restoring state from the
                // explicit stack
                iter = next_iter;

                Some((key, child_node))
            } else {
                // Reached the bottom of the stack, we’re done
                None
            }
        })
    }

    /// Mutable iterator for the `(entry, copy source)` of each node.
    ///
    /// It would not be safe to yield mutable references to nodes themeselves
    /// with `-> impl Iterator<Item = &mut Node>` since child nodes are
    /// reachable from their ancestor nodes, potentially creating multiple
    /// `&mut` references to a given node.
    fn iter_node_data_mut<'a>(
        &'a mut self,
    ) -> impl Iterator<Item = NodeDataMut<'a>> + 'a {
        // Explict stack for pseudo-recursion, see `iter_nodes` above.
        let mut stack = Vec::new();
        let mut iter = self.root.iter_mut();
        std::iter::from_fn(move || {
            while let Some((key, child_node)) = iter.next() {
                // Pseudo-recursion
                let data =
                    (key, &mut child_node.entry, &mut child_node.copy_source);
                let new_iter = child_node.children.iter_mut();
                let old_iter = std::mem::replace(&mut iter, new_iter);
                stack.push((data, old_iter));
            }
            // Found the end of a `children.values_mut()` iterator.
            if let Some((data, next_iter)) = stack.pop() {
                // "Return" from pseudo-recursion by restoring state from the
                // explicit stack
                iter = next_iter;

                Some(data)
            } else {
                // Reached the bottom of the stack, we’re done
                None
            }
        })
    }
}

impl super::dispatch::DirstateMapMethods for DirstateMap {
    fn clear(&mut self) {
        self.set_parents(&DirstateParents {
            p1: NULL_NODE,
            p2: NULL_NODE,
        });
        self.root.clear()
    }

    fn add_file(
        &mut self,
        _filename: &HgPath,
        _old_state: EntryState,
        _entry: DirstateEntry,
    ) -> Result<(), DirstateMapError> {
        todo!()
    }

    fn remove_file(
        &mut self,
        _filename: &HgPath,
        _old_state: EntryState,
        _size: i32,
    ) -> Result<(), DirstateMapError> {
        todo!()
    }

    fn drop_file(
        &mut self,
        _filename: &HgPath,
        _old_state: EntryState,
    ) -> Result<bool, DirstateMapError> {
        todo!()
    }

    fn clear_ambiguous_times(
        &mut self,
        _filenames: Vec<HgPathBuf>,
        _now: i32,
    ) {
        todo!()
    }

    fn non_normal_entries_contains(&mut self, _key: &HgPath) -> bool {
        todo!()
    }

    fn non_normal_entries_remove(&mut self, _key: &HgPath) -> bool {
        todo!()
    }

    fn non_normal_or_other_parent_paths(
        &mut self,
    ) -> Box<dyn Iterator<Item = &HgPathBuf> + '_> {
        todo!()
    }

    fn set_non_normal_other_parent_entries(&mut self, _force: bool) {
        todo!()
    }

    fn iter_non_normal_paths(
        &mut self,
    ) -> Box<dyn Iterator<Item = &HgPathBuf> + Send + '_> {
        todo!()
    }

    fn iter_non_normal_paths_panic(
        &self,
    ) -> Box<dyn Iterator<Item = &HgPathBuf> + Send + '_> {
        todo!()
    }

    fn iter_other_parent_paths(
        &mut self,
    ) -> Box<dyn Iterator<Item = &HgPathBuf> + Send + '_> {
        todo!()
    }

    fn has_tracked_dir(
        &mut self,
        _directory: &HgPath,
    ) -> Result<bool, DirstateMapError> {
        todo!()
    }

    fn has_dir(
        &mut self,
        _directory: &HgPath,
    ) -> Result<bool, DirstateMapError> {
        todo!()
    }

    fn parents(
        &mut self,
        file_contents: &[u8],
    ) -> Result<&DirstateParents, DirstateError> {
        if self.parents.is_none() {
            let parents = if !file_contents.is_empty() {
                parse_dirstate_parents(file_contents)?.clone()
            } else {
                DirstateParents {
                    p1: NULL_NODE,
                    p2: NULL_NODE,
                }
            };
            self.parents = Some(parents);
        }
        Ok(self.parents.as_ref().unwrap())
    }

    fn set_parents(&mut self, parents: &DirstateParents) {
        self.parents = Some(parents.clone());
        self.dirty_parents = true;
    }

    fn read<'a>(
        &mut self,
        file_contents: &'a [u8],
    ) -> Result<Option<&'a DirstateParents>, DirstateError> {
        if file_contents.is_empty() {
            return Ok(None);
        }

        let parents = parse_dirstate_entries(
            file_contents,
            |path, entry, copy_source| {
                let node = self.get_or_insert_node(path);
                node.entry = Some(*entry);
                node.copy_source = copy_source.map(HgPath::to_owned);
            },
        )?;

        if !self.dirty_parents {
            self.set_parents(parents);
        }

        Ok(Some(parents))
    }

    fn pack(
        &mut self,
        _parents: DirstateParents,
        _now: Timestamp,
    ) -> Result<Vec<u8>, DirstateError> {
        let _ = self.iter_node_data_mut();
        todo!()
    }

    fn build_file_fold_map(&mut self) -> &FastHashMap<HgPathBuf, HgPathBuf> {
        todo!()
    }

    fn set_all_dirs(&mut self) -> Result<(), DirstateMapError> {
        todo!()
    }

    fn set_dirs(&mut self) -> Result<(), DirstateMapError> {
        todo!()
    }

    fn status<'a>(
        &'a self,
        _matcher: &'a (dyn Matcher + Sync),
        _root_dir: PathBuf,
        _ignore_files: Vec<PathBuf>,
        _options: StatusOptions,
    ) -> Result<
        (
            (Vec<HgPathCow<'a>>, DirstateStatus<'a>),
            Vec<PatternFileWarning>,
        ),
        StatusError,
    > {
        todo!()
    }

    fn copy_map_len(&self) -> usize {
        todo!()
    }

    fn copy_map_iter(&self) -> CopyMapIter<'_> {
        Box::new(self.iter_nodes().filter_map(|(path, node)| {
            node.copy_source
                .as_ref()
                .map(|copy_source| (path.full_path(), copy_source))
        }))
    }

    fn copy_map_contains_key(&self, key: &HgPath) -> bool {
        if let Some(node) = self.get_node(key) {
            node.copy_source.is_some()
        } else {
            false
        }
    }

    fn copy_map_get(&self, key: &HgPath) -> Option<&HgPathBuf> {
        self.get_node(key)?.copy_source.as_ref()
    }

    fn copy_map_remove(&mut self, _key: &HgPath) -> Option<HgPathBuf> {
        todo!()
    }

    fn copy_map_insert(
        &mut self,
        _key: HgPathBuf,
        _value: HgPathBuf,
    ) -> Option<HgPathBuf> {
        todo!()
    }

    fn len(&self) -> usize {
        todo!()
    }

    fn contains_key(&self, key: &HgPath) -> bool {
        self.get(key).is_some()
    }

    fn get(&self, key: &HgPath) -> Option<&DirstateEntry> {
        self.get_node(key)?.entry.as_ref()
    }

    fn iter(&self) -> StateMapIter<'_> {
        Box::new(self.iter_nodes().filter_map(|(path, node)| {
            node.entry.as_ref().map(|entry| (path.full_path(), entry))
        }))
    }
}
