use bytes_cast::BytesCast;
use micro_timer::timed;
use std::borrow::Cow;
use std::path::PathBuf;

use super::on_disk;
use super::on_disk::DirstateV2ParseError;
use super::owning::OwningDirstateMap;
use super::path_with_basename::WithBasename;
use crate::dirstate::parsers::pack_entry;
use crate::dirstate::parsers::packed_entry_size;
use crate::dirstate::parsers::parse_dirstate_entries;
use crate::dirstate::CopyMapIter;
use crate::dirstate::DirstateV2Data;
use crate::dirstate::ParentFileData;
use crate::dirstate::StateMapIter;
use crate::dirstate::TruncatedTimestamp;
use crate::matchers::Matcher;
use crate::utils::hg_path::{HgPath, HgPathBuf};
use crate::DirstateEntry;
use crate::DirstateError;
use crate::DirstateMapError;
use crate::DirstateParents;
use crate::DirstateStatus;
use crate::EntryState;
use crate::FastHashbrownMap as FastHashMap;
use crate::PatternFileWarning;
use crate::StatusError;
use crate::StatusOptions;

/// Append to an existing data file if the amount of unreachable data (not used
/// anymore) is less than this fraction of the total amount of existing data.
const ACCEPTABLE_UNREACHABLE_BYTES_RATIO: f32 = 0.5;

pub struct DirstateMap<'on_disk> {
    /// Contents of the `.hg/dirstate` file
    pub(super) on_disk: &'on_disk [u8],

    pub(super) root: ChildNodes<'on_disk>,

    /// Number of nodes anywhere in the tree that have `.entry.is_some()`.
    pub(super) nodes_with_entry_count: u32,

    /// Number of nodes anywhere in the tree that have
    /// `.copy_source.is_some()`.
    pub(super) nodes_with_copy_source_count: u32,

    /// See on_disk::Header
    pub(super) ignore_patterns_hash: on_disk::IgnorePatternsHash,

    /// How many bytes of `on_disk` are not used anymore
    pub(super) unreachable_bytes: u32,
}

/// Using a plain `HgPathBuf` of the full path from the repository root as a
/// map key would also work: all paths in a given map have the same parent
/// path, so comparing full paths gives the same result as comparing base
/// names. However `HashMap` would waste time always re-hashing the same
/// string prefix.
pub(super) type NodeKey<'on_disk> = WithBasename<Cow<'on_disk, HgPath>>;

/// Similar to `&'tree Cow<'on_disk, HgPath>`, but can also be returned
/// for on-disk nodes that don’t actually have a `Cow` to borrow.
pub(super) enum BorrowedPath<'tree, 'on_disk> {
    InMemory(&'tree HgPathBuf),
    OnDisk(&'on_disk HgPath),
}

pub(super) enum ChildNodes<'on_disk> {
    InMemory(FastHashMap<NodeKey<'on_disk>, Node<'on_disk>>),
    OnDisk(&'on_disk [on_disk::Node]),
}

pub(super) enum ChildNodesRef<'tree, 'on_disk> {
    InMemory(&'tree FastHashMap<NodeKey<'on_disk>, Node<'on_disk>>),
    OnDisk(&'on_disk [on_disk::Node]),
}

pub(super) enum NodeRef<'tree, 'on_disk> {
    InMemory(&'tree NodeKey<'on_disk>, &'tree Node<'on_disk>),
    OnDisk(&'on_disk on_disk::Node),
}

impl<'tree, 'on_disk> BorrowedPath<'tree, 'on_disk> {
    pub fn detach_from_tree(&self) -> Cow<'on_disk, HgPath> {
        match *self {
            BorrowedPath::InMemory(in_memory) => Cow::Owned(in_memory.clone()),
            BorrowedPath::OnDisk(on_disk) => Cow::Borrowed(on_disk),
        }
    }
}

impl<'tree, 'on_disk> std::ops::Deref for BorrowedPath<'tree, 'on_disk> {
    type Target = HgPath;

    fn deref(&self) -> &HgPath {
        match *self {
            BorrowedPath::InMemory(in_memory) => in_memory,
            BorrowedPath::OnDisk(on_disk) => on_disk,
        }
    }
}

impl Default for ChildNodes<'_> {
    fn default() -> Self {
        ChildNodes::InMemory(Default::default())
    }
}

impl<'on_disk> ChildNodes<'on_disk> {
    pub(super) fn as_ref<'tree>(
        &'tree self,
    ) -> ChildNodesRef<'tree, 'on_disk> {
        match self {
            ChildNodes::InMemory(nodes) => ChildNodesRef::InMemory(nodes),
            ChildNodes::OnDisk(nodes) => ChildNodesRef::OnDisk(nodes),
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        match self {
            ChildNodes::InMemory(nodes) => nodes.is_empty(),
            ChildNodes::OnDisk(nodes) => nodes.is_empty(),
        }
    }

    fn make_mut(
        &mut self,
        on_disk: &'on_disk [u8],
        unreachable_bytes: &mut u32,
    ) -> Result<
        &mut FastHashMap<NodeKey<'on_disk>, Node<'on_disk>>,
        DirstateV2ParseError,
    > {
        match self {
            ChildNodes::InMemory(nodes) => Ok(nodes),
            ChildNodes::OnDisk(nodes) => {
                *unreachable_bytes +=
                    std::mem::size_of_val::<[on_disk::Node]>(nodes) as u32;
                let nodes = nodes
                    .iter()
                    .map(|node| {
                        Ok((
                            node.path(on_disk)?,
                            node.to_in_memory_node(on_disk)?,
                        ))
                    })
                    .collect::<Result<_, _>>()?;
                *self = ChildNodes::InMemory(nodes);
                match self {
                    ChildNodes::InMemory(nodes) => Ok(nodes),
                    ChildNodes::OnDisk(_) => unreachable!(),
                }
            }
        }
    }
}

impl<'tree, 'on_disk> ChildNodesRef<'tree, 'on_disk> {
    pub(super) fn get(
        &self,
        base_name: &HgPath,
        on_disk: &'on_disk [u8],
    ) -> Result<Option<NodeRef<'tree, 'on_disk>>, DirstateV2ParseError> {
        match self {
            ChildNodesRef::InMemory(nodes) => Ok(nodes
                .get_key_value(base_name)
                .map(|(k, v)| NodeRef::InMemory(k, v))),
            ChildNodesRef::OnDisk(nodes) => {
                let mut parse_result = Ok(());
                let search_result = nodes.binary_search_by(|node| {
                    match node.base_name(on_disk) {
                        Ok(node_base_name) => node_base_name.cmp(base_name),
                        Err(e) => {
                            parse_result = Err(e);
                            // Dummy comparison result, `search_result` won’t
                            // be used since `parse_result` is an error
                            std::cmp::Ordering::Equal
                        }
                    }
                });
                parse_result.map(|()| {
                    search_result.ok().map(|i| NodeRef::OnDisk(&nodes[i]))
                })
            }
        }
    }

    /// Iterate in undefined order
    pub(super) fn iter(
        &self,
    ) -> impl Iterator<Item = NodeRef<'tree, 'on_disk>> {
        match self {
            ChildNodesRef::InMemory(nodes) => itertools::Either::Left(
                nodes.iter().map(|(k, v)| NodeRef::InMemory(k, v)),
            ),
            ChildNodesRef::OnDisk(nodes) => {
                itertools::Either::Right(nodes.iter().map(NodeRef::OnDisk))
            }
        }
    }

    /// Iterate in parallel in undefined order
    pub(super) fn par_iter(
        &self,
    ) -> impl rayon::iter::ParallelIterator<Item = NodeRef<'tree, 'on_disk>>
    {
        use rayon::prelude::*;
        match self {
            ChildNodesRef::InMemory(nodes) => rayon::iter::Either::Left(
                nodes.par_iter().map(|(k, v)| NodeRef::InMemory(k, v)),
            ),
            ChildNodesRef::OnDisk(nodes) => rayon::iter::Either::Right(
                nodes.par_iter().map(NodeRef::OnDisk),
            ),
        }
    }

    pub(super) fn sorted(&self) -> Vec<NodeRef<'tree, 'on_disk>> {
        match self {
            ChildNodesRef::InMemory(nodes) => {
                let mut vec: Vec<_> = nodes
                    .iter()
                    .map(|(k, v)| NodeRef::InMemory(k, v))
                    .collect();
                fn sort_key<'a>(node: &'a NodeRef) -> &'a HgPath {
                    match node {
                        NodeRef::InMemory(path, _node) => path.base_name(),
                        NodeRef::OnDisk(_) => unreachable!(),
                    }
                }
                // `sort_unstable_by_key` doesn’t allow keys borrowing from the
                // value: https://github.com/rust-lang/rust/issues/34162
                vec.sort_unstable_by(|a, b| sort_key(a).cmp(sort_key(b)));
                vec
            }
            ChildNodesRef::OnDisk(nodes) => {
                // Nodes on disk are already sorted
                nodes.iter().map(NodeRef::OnDisk).collect()
            }
        }
    }
}

impl<'tree, 'on_disk> NodeRef<'tree, 'on_disk> {
    pub(super) fn full_path(
        &self,
        on_disk: &'on_disk [u8],
    ) -> Result<&'tree HgPath, DirstateV2ParseError> {
        match self {
            NodeRef::InMemory(path, _node) => Ok(path.full_path()),
            NodeRef::OnDisk(node) => node.full_path(on_disk),
        }
    }

    /// Returns a `BorrowedPath`, which can be turned into a `Cow<'on_disk,
    /// HgPath>` detached from `'tree`
    pub(super) fn full_path_borrowed(
        &self,
        on_disk: &'on_disk [u8],
    ) -> Result<BorrowedPath<'tree, 'on_disk>, DirstateV2ParseError> {
        match self {
            NodeRef::InMemory(path, _node) => match path.full_path() {
                Cow::Borrowed(on_disk) => Ok(BorrowedPath::OnDisk(on_disk)),
                Cow::Owned(in_memory) => Ok(BorrowedPath::InMemory(in_memory)),
            },
            NodeRef::OnDisk(node) => {
                Ok(BorrowedPath::OnDisk(node.full_path(on_disk)?))
            }
        }
    }

    pub(super) fn base_name(
        &self,
        on_disk: &'on_disk [u8],
    ) -> Result<&'tree HgPath, DirstateV2ParseError> {
        match self {
            NodeRef::InMemory(path, _node) => Ok(path.base_name()),
            NodeRef::OnDisk(node) => node.base_name(on_disk),
        }
    }

    pub(super) fn children(
        &self,
        on_disk: &'on_disk [u8],
    ) -> Result<ChildNodesRef<'tree, 'on_disk>, DirstateV2ParseError> {
        match self {
            NodeRef::InMemory(_path, node) => Ok(node.children.as_ref()),
            NodeRef::OnDisk(node) => {
                Ok(ChildNodesRef::OnDisk(node.children(on_disk)?))
            }
        }
    }

    pub(super) fn has_copy_source(&self) -> bool {
        match self {
            NodeRef::InMemory(_path, node) => node.copy_source.is_some(),
            NodeRef::OnDisk(node) => node.has_copy_source(),
        }
    }

    pub(super) fn copy_source(
        &self,
        on_disk: &'on_disk [u8],
    ) -> Result<Option<&'tree HgPath>, DirstateV2ParseError> {
        match self {
            NodeRef::InMemory(_path, node) => {
                Ok(node.copy_source.as_ref().map(|s| &**s))
            }
            NodeRef::OnDisk(node) => node.copy_source(on_disk),
        }
    }
    /// Returns a `BorrowedPath`, which can be turned into a `Cow<'on_disk,
    /// HgPath>` detached from `'tree`
    pub(super) fn copy_source_borrowed(
        &self,
        on_disk: &'on_disk [u8],
    ) -> Result<Option<BorrowedPath<'tree, 'on_disk>>, DirstateV2ParseError>
    {
        Ok(match self {
            NodeRef::InMemory(_path, node) => {
                node.copy_source.as_ref().map(|source| match source {
                    Cow::Borrowed(on_disk) => BorrowedPath::OnDisk(on_disk),
                    Cow::Owned(in_memory) => BorrowedPath::InMemory(in_memory),
                })
            }
            NodeRef::OnDisk(node) => node
                .copy_source(on_disk)?
                .map(|source| BorrowedPath::OnDisk(source)),
        })
    }

    pub(super) fn entry(
        &self,
    ) -> Result<Option<DirstateEntry>, DirstateV2ParseError> {
        match self {
            NodeRef::InMemory(_path, node) => {
                Ok(node.data.as_entry().copied())
            }
            NodeRef::OnDisk(node) => node.entry(),
        }
    }

    pub(super) fn state(
        &self,
    ) -> Result<Option<EntryState>, DirstateV2ParseError> {
        Ok(self.entry()?.and_then(|e| {
            if e.any_tracked() {
                Some(e.state())
            } else {
                None
            }
        }))
    }

    pub(super) fn cached_directory_mtime(
        &self,
    ) -> Result<Option<TruncatedTimestamp>, DirstateV2ParseError> {
        match self {
            NodeRef::InMemory(_path, node) => Ok(match node.data {
                NodeData::CachedDirectory { mtime } => Some(mtime),
                _ => None,
            }),
            NodeRef::OnDisk(node) => node.cached_directory_mtime(),
        }
    }

    pub(super) fn descendants_with_entry_count(&self) -> u32 {
        match self {
            NodeRef::InMemory(_path, node) => {
                node.descendants_with_entry_count
            }
            NodeRef::OnDisk(node) => node.descendants_with_entry_count.get(),
        }
    }

    pub(super) fn tracked_descendants_count(&self) -> u32 {
        match self {
            NodeRef::InMemory(_path, node) => node.tracked_descendants_count,
            NodeRef::OnDisk(node) => node.tracked_descendants_count.get(),
        }
    }
}

/// Represents a file or a directory
#[derive(Default)]
pub(super) struct Node<'on_disk> {
    pub(super) data: NodeData,

    pub(super) copy_source: Option<Cow<'on_disk, HgPath>>,

    pub(super) children: ChildNodes<'on_disk>,

    /// How many (non-inclusive) descendants of this node have an entry.
    pub(super) descendants_with_entry_count: u32,

    /// How many (non-inclusive) descendants of this node have an entry whose
    /// state is "tracked".
    pub(super) tracked_descendants_count: u32,
}

pub(super) enum NodeData {
    Entry(DirstateEntry),
    CachedDirectory { mtime: TruncatedTimestamp },
    None,
}

impl Default for NodeData {
    fn default() -> Self {
        NodeData::None
    }
}

impl NodeData {
    fn has_entry(&self) -> bool {
        match self {
            NodeData::Entry(_) => true,
            _ => false,
        }
    }

    fn as_entry(&self) -> Option<&DirstateEntry> {
        match self {
            NodeData::Entry(entry) => Some(entry),
            _ => None,
        }
    }

    fn as_entry_mut(&mut self) -> Option<&mut DirstateEntry> {
        match self {
            NodeData::Entry(entry) => Some(entry),
            _ => None,
        }
    }
}

impl<'on_disk> DirstateMap<'on_disk> {
    pub(super) fn empty(on_disk: &'on_disk [u8]) -> Self {
        Self {
            on_disk,
            root: ChildNodes::default(),
            nodes_with_entry_count: 0,
            nodes_with_copy_source_count: 0,
            ignore_patterns_hash: [0; on_disk::IGNORE_PATTERNS_HASH_LEN],
            unreachable_bytes: 0,
        }
    }

    #[timed]
    pub fn new_v2(
        on_disk: &'on_disk [u8],
        data_size: usize,
        metadata: &[u8],
    ) -> Result<Self, DirstateError> {
        if let Some(data) = on_disk.get(..data_size) {
            Ok(on_disk::read(data, metadata)?)
        } else {
            Err(DirstateV2ParseError.into())
        }
    }

    #[timed]
    pub fn new_v1(
        on_disk: &'on_disk [u8],
    ) -> Result<(Self, Option<DirstateParents>), DirstateError> {
        let mut map = Self::empty(on_disk);
        if map.on_disk.is_empty() {
            return Ok((map, None));
        }

        let parents = parse_dirstate_entries(
            map.on_disk,
            |path, entry, copy_source| {
                let tracked = entry.state().is_tracked();
                let node = Self::get_or_insert_node(
                    map.on_disk,
                    &mut map.unreachable_bytes,
                    &mut map.root,
                    path,
                    WithBasename::to_cow_borrowed,
                    |ancestor| {
                        if tracked {
                            ancestor.tracked_descendants_count += 1
                        }
                        ancestor.descendants_with_entry_count += 1
                    },
                )?;
                assert!(
                    !node.data.has_entry(),
                    "duplicate dirstate entry in read"
                );
                assert!(
                    node.copy_source.is_none(),
                    "duplicate dirstate entry in read"
                );
                node.data = NodeData::Entry(*entry);
                node.copy_source = copy_source.map(Cow::Borrowed);
                map.nodes_with_entry_count += 1;
                if copy_source.is_some() {
                    map.nodes_with_copy_source_count += 1
                }
                Ok(())
            },
        )?;
        let parents = Some(parents.clone());

        Ok((map, parents))
    }

    /// Assuming dirstate-v2 format, returns whether the next write should
    /// append to the existing data file that contains `self.on_disk` (true),
    /// or create a new data file from scratch (false).
    pub(super) fn write_should_append(&self) -> bool {
        let ratio = self.unreachable_bytes as f32 / self.on_disk.len() as f32;
        ratio < ACCEPTABLE_UNREACHABLE_BYTES_RATIO
    }

    fn get_node<'tree>(
        &'tree self,
        path: &HgPath,
    ) -> Result<Option<NodeRef<'tree, 'on_disk>>, DirstateV2ParseError> {
        let mut children = self.root.as_ref();
        let mut components = path.components();
        let mut component =
            components.next().expect("expected at least one components");
        loop {
            if let Some(child) = children.get(component, self.on_disk)? {
                if let Some(next_component) = components.next() {
                    component = next_component;
                    children = child.children(self.on_disk)?;
                } else {
                    return Ok(Some(child));
                }
            } else {
                return Ok(None);
            }
        }
    }

    /// Returns a mutable reference to the node at `path` if it exists
    ///
    /// This takes `root` instead of `&mut self` so that callers can mutate
    /// other fields while the returned borrow is still valid
    fn get_node_mut<'tree>(
        on_disk: &'on_disk [u8],
        unreachable_bytes: &mut u32,
        root: &'tree mut ChildNodes<'on_disk>,
        path: &HgPath,
    ) -> Result<Option<&'tree mut Node<'on_disk>>, DirstateV2ParseError> {
        let mut children = root;
        let mut components = path.components();
        let mut component =
            components.next().expect("expected at least one components");
        loop {
            if let Some(child) = children
                .make_mut(on_disk, unreachable_bytes)?
                .get_mut(component)
            {
                if let Some(next_component) = components.next() {
                    component = next_component;
                    children = &mut child.children;
                } else {
                    return Ok(Some(child));
                }
            } else {
                return Ok(None);
            }
        }
    }

    pub(super) fn get_or_insert<'tree, 'path>(
        &'tree mut self,
        path: &HgPath,
    ) -> Result<&'tree mut Node<'on_disk>, DirstateV2ParseError> {
        Self::get_or_insert_node(
            self.on_disk,
            &mut self.unreachable_bytes,
            &mut self.root,
            path,
            WithBasename::to_cow_owned,
            |_| {},
        )
    }

    fn get_or_insert_node<'tree, 'path>(
        on_disk: &'on_disk [u8],
        unreachable_bytes: &mut u32,
        root: &'tree mut ChildNodes<'on_disk>,
        path: &'path HgPath,
        to_cow: impl Fn(
            WithBasename<&'path HgPath>,
        ) -> WithBasename<Cow<'on_disk, HgPath>>,
        mut each_ancestor: impl FnMut(&mut Node),
    ) -> Result<&'tree mut Node<'on_disk>, DirstateV2ParseError> {
        let mut child_nodes = root;
        let mut inclusive_ancestor_paths =
            WithBasename::inclusive_ancestors_of(path);
        let mut ancestor_path = inclusive_ancestor_paths
            .next()
            .expect("expected at least one inclusive ancestor");
        loop {
            let (_, child_node) = child_nodes
                .make_mut(on_disk, unreachable_bytes)?
                .raw_entry_mut()
                .from_key(ancestor_path.base_name())
                .or_insert_with(|| (to_cow(ancestor_path), Node::default()));
            if let Some(next) = inclusive_ancestor_paths.next() {
                each_ancestor(child_node);
                ancestor_path = next;
                child_nodes = &mut child_node.children;
            } else {
                return Ok(child_node);
            }
        }
    }

    fn reset_state(
        &mut self,
        filename: &HgPath,
        old_entry_opt: Option<DirstateEntry>,
        wc_tracked: bool,
        p1_tracked: bool,
        p2_info: bool,
        has_meaningful_mtime: bool,
        parent_file_data_opt: Option<ParentFileData>,
    ) -> Result<(), DirstateError> {
        let (had_entry, was_tracked) = match old_entry_opt {
            Some(old_entry) => (true, old_entry.tracked()),
            None => (false, false),
        };
        let node = Self::get_or_insert_node(
            self.on_disk,
            &mut self.unreachable_bytes,
            &mut self.root,
            filename,
            WithBasename::to_cow_owned,
            |ancestor| {
                if !had_entry {
                    ancestor.descendants_with_entry_count += 1;
                }
                if was_tracked {
                    if !wc_tracked {
                        ancestor.tracked_descendants_count = ancestor
                            .tracked_descendants_count
                            .checked_sub(1)
                            .expect("tracked count to be >= 0");
                    }
                } else {
                    if wc_tracked {
                        ancestor.tracked_descendants_count += 1;
                    }
                }
            },
        )?;

        let v2_data = if let Some(parent_file_data) = parent_file_data_opt {
            DirstateV2Data {
                wc_tracked,
                p1_tracked,
                p2_info,
                mode_size: parent_file_data.mode_size,
                mtime: if has_meaningful_mtime {
                    parent_file_data.mtime
                } else {
                    None
                },
                ..Default::default()
            }
        } else {
            DirstateV2Data {
                wc_tracked,
                p1_tracked,
                p2_info,
                ..Default::default()
            }
        };
        if !had_entry {
            self.nodes_with_entry_count += 1;
        }
        node.data = NodeData::Entry(DirstateEntry::from_v2_data(v2_data));
        Ok(())
    }

    fn set_tracked(
        &mut self,
        filename: &HgPath,
        old_entry_opt: Option<DirstateEntry>,
    ) -> Result<bool, DirstateV2ParseError> {
        let was_tracked = old_entry_opt.map_or(false, |e| e.tracked());
        let had_entry = old_entry_opt.is_some();
        let tracked_count_increment = if was_tracked { 0 } else { 1 };
        let mut new = false;

        let node = Self::get_or_insert_node(
            self.on_disk,
            &mut self.unreachable_bytes,
            &mut self.root,
            filename,
            WithBasename::to_cow_owned,
            |ancestor| {
                if !had_entry {
                    ancestor.descendants_with_entry_count += 1;
                }

                ancestor.tracked_descendants_count += tracked_count_increment;
            },
        )?;
        let new_entry = if let Some(old_entry) = old_entry_opt {
            let mut e = old_entry.clone();
            if e.tracked() {
                // XXX
                // This is probably overkill for more case, but we need this to
                // fully replace the `normallookup` call with `set_tracked`
                // one. Consider smoothing this in the future.
                e.set_possibly_dirty();
            } else {
                new = true;
                e.set_tracked();
            }
            e
        } else {
            self.nodes_with_entry_count += 1;
            new = true;
            DirstateEntry::new_tracked()
        };
        node.data = NodeData::Entry(new_entry);
        Ok(new)
    }

    /// It is the responsibility of the caller to know that there was an entry
    /// there before. Does not handle the removal of copy source
    fn set_untracked(
        &mut self,
        filename: &HgPath,
        old_entry: DirstateEntry,
    ) -> Result<(), DirstateV2ParseError> {
        let node = Self::get_or_insert_node(
            self.on_disk,
            &mut self.unreachable_bytes,
            &mut self.root,
            filename,
            WithBasename::to_cow_owned,
            |ancestor| {
                ancestor.tracked_descendants_count = ancestor
                    .tracked_descendants_count
                    .checked_sub(1)
                    .expect("tracked_descendants_count should be >= 0");
            },
        )?;
        let mut new_entry = old_entry.clone();
        new_entry.set_untracked();
        node.data = NodeData::Entry(new_entry);
        Ok(())
    }

    fn set_clean(
        &mut self,
        filename: &HgPath,
        old_entry: DirstateEntry,
        mode: u32,
        size: u32,
        mtime: TruncatedTimestamp,
    ) -> Result<(), DirstateError> {
        let node = Self::get_or_insert_node(
            self.on_disk,
            &mut self.unreachable_bytes,
            &mut self.root,
            filename,
            WithBasename::to_cow_owned,
            |ancestor| {
                if !old_entry.tracked() {
                    ancestor.tracked_descendants_count += 1;
                }
            },
        )?;
        let mut new_entry = old_entry.clone();
        new_entry.set_clean(mode, size, mtime);
        node.data = NodeData::Entry(new_entry);
        Ok(())
    }

    fn set_possibly_dirty(
        &mut self,
        filename: &HgPath,
    ) -> Result<(), DirstateError> {
        let node = Self::get_or_insert_node(
            self.on_disk,
            &mut self.unreachable_bytes,
            &mut self.root,
            filename,
            WithBasename::to_cow_owned,
            |_ancestor| {},
        )?;
        let entry = node.data.as_entry_mut().expect("entry should exist");
        entry.set_possibly_dirty();
        node.data = NodeData::Entry(*entry);
        Ok(())
    }

    fn iter_nodes<'tree>(
        &'tree self,
    ) -> impl Iterator<
        Item = Result<NodeRef<'tree, 'on_disk>, DirstateV2ParseError>,
    > + 'tree {
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
        let mut iter = self.root.as_ref().iter();
        std::iter::from_fn(move || {
            while let Some(child_node) = iter.next() {
                let children = match child_node.children(self.on_disk) {
                    Ok(children) => children,
                    Err(error) => return Some(Err(error)),
                };
                // Pseudo-recursion
                let new_iter = children.iter();
                let old_iter = std::mem::replace(&mut iter, new_iter);
                stack.push((child_node, old_iter));
            }
            // Found the end of a `children.iter()` iterator.
            if let Some((child_node, next_iter)) = stack.pop() {
                // "Return" from pseudo-recursion by restoring state from the
                // explicit stack
                iter = next_iter;

                Some(Ok(child_node))
            } else {
                // Reached the bottom of the stack, we’re done
                None
            }
        })
    }

    fn count_dropped_path(unreachable_bytes: &mut u32, path: &Cow<HgPath>) {
        if let Cow::Borrowed(path) = path {
            *unreachable_bytes += path.len() as u32
        }
    }
}

/// Like `Iterator::filter_map`, but over a fallible iterator of `Result`s.
///
/// The callback is only called for incoming `Ok` values. Errors are passed
/// through as-is. In order to let it use the `?` operator the callback is
/// expected to return a `Result` of `Option`, instead of an `Option` of
/// `Result`.
fn filter_map_results<'a, I, F, A, B, E>(
    iter: I,
    f: F,
) -> impl Iterator<Item = Result<B, E>> + 'a
where
    I: Iterator<Item = Result<A, E>> + 'a,
    F: Fn(A) -> Result<Option<B>, E> + 'a,
{
    iter.filter_map(move |result| match result {
        Ok(node) => f(node).transpose(),
        Err(e) => Some(Err(e)),
    })
}

impl OwningDirstateMap {
    pub fn clear(&mut self) {
        self.with_dmap_mut(|map| {
            map.root = Default::default();
            map.nodes_with_entry_count = 0;
            map.nodes_with_copy_source_count = 0;
        });
    }

    pub fn set_entry(
        &mut self,
        filename: &HgPath,
        entry: DirstateEntry,
    ) -> Result<(), DirstateV2ParseError> {
        self.with_dmap_mut(|map| {
            map.get_or_insert(&filename)?.data = NodeData::Entry(entry);
            Ok(())
        })
    }

    pub fn set_tracked(
        &mut self,
        filename: &HgPath,
    ) -> Result<bool, DirstateV2ParseError> {
        let old_entry_opt = self.get(filename)?;
        self.with_dmap_mut(|map| map.set_tracked(filename, old_entry_opt))
    }

    pub fn set_untracked(
        &mut self,
        filename: &HgPath,
    ) -> Result<bool, DirstateError> {
        let old_entry_opt = self.get(filename)?;
        match old_entry_opt {
            None => Ok(false),
            Some(old_entry) => {
                if !old_entry.tracked() {
                    // `DirstateMap::set_untracked` is not a noop if
                    // already not tracked as it will decrement the
                    // tracked counters while going down.
                    return Ok(true);
                }
                if old_entry.added() {
                    // Untracking an "added" entry will just result in a
                    // worthless entry (and other parts of the code will
                    // complain about it), just drop it entirely.
                    self.drop_entry_and_copy_source(filename)?;
                    return Ok(true);
                }
                if !old_entry.p2_info() {
                    self.copy_map_remove(filename)?;
                }

                self.with_dmap_mut(|map| {
                    map.set_untracked(filename, old_entry)?;
                    Ok(true)
                })
            }
        }
    }

    pub fn set_clean(
        &mut self,
        filename: &HgPath,
        mode: u32,
        size: u32,
        mtime: TruncatedTimestamp,
    ) -> Result<(), DirstateError> {
        let old_entry = match self.get(filename)? {
            None => {
                return Err(
                    DirstateMapError::PathNotFound(filename.into()).into()
                )
            }
            Some(e) => e,
        };
        self.copy_map_remove(filename)?;
        self.with_dmap_mut(|map| {
            map.set_clean(filename, old_entry, mode, size, mtime)
        })
    }

    pub fn set_possibly_dirty(
        &mut self,
        filename: &HgPath,
    ) -> Result<(), DirstateError> {
        if self.get(filename)?.is_none() {
            return Err(DirstateMapError::PathNotFound(filename.into()).into());
        }
        self.with_dmap_mut(|map| map.set_possibly_dirty(filename))
    }

    pub fn reset_state(
        &mut self,
        filename: &HgPath,
        wc_tracked: bool,
        p1_tracked: bool,
        p2_info: bool,
        has_meaningful_mtime: bool,
        parent_file_data_opt: Option<ParentFileData>,
    ) -> Result<(), DirstateError> {
        if !(p1_tracked || p2_info || wc_tracked) {
            self.drop_entry_and_copy_source(filename)?;
            return Ok(());
        }
        self.copy_map_remove(filename)?;
        let old_entry_opt = self.get(filename)?;
        self.with_dmap_mut(|map| {
            map.reset_state(
                filename,
                old_entry_opt,
                wc_tracked,
                p1_tracked,
                p2_info,
                has_meaningful_mtime,
                parent_file_data_opt,
            )
        })
    }

    pub fn drop_entry_and_copy_source(
        &mut self,
        filename: &HgPath,
    ) -> Result<(), DirstateError> {
        let was_tracked = self
            .get(filename)?
            .map_or(false, |e| e.state().is_tracked());
        struct Dropped {
            was_tracked: bool,
            had_entry: bool,
            had_copy_source: bool,
        }

        /// If this returns `Ok(Some((dropped, removed)))`, then
        ///
        /// * `dropped` is about the leaf node that was at `filename`
        /// * `removed` is whether this particular level of recursion just
        ///   removed a node in `nodes`.
        fn recur<'on_disk>(
            on_disk: &'on_disk [u8],
            unreachable_bytes: &mut u32,
            nodes: &mut ChildNodes<'on_disk>,
            path: &HgPath,
        ) -> Result<Option<(Dropped, bool)>, DirstateV2ParseError> {
            let (first_path_component, rest_of_path) =
                path.split_first_component();
            let nodes = nodes.make_mut(on_disk, unreachable_bytes)?;
            let node = if let Some(node) = nodes.get_mut(first_path_component)
            {
                node
            } else {
                return Ok(None);
            };
            let dropped;
            if let Some(rest) = rest_of_path {
                if let Some((d, removed)) = recur(
                    on_disk,
                    unreachable_bytes,
                    &mut node.children,
                    rest,
                )? {
                    dropped = d;
                    if dropped.had_entry {
                        node.descendants_with_entry_count = node
                            .descendants_with_entry_count
                            .checked_sub(1)
                            .expect(
                                "descendants_with_entry_count should be >= 0",
                            );
                    }
                    if dropped.was_tracked {
                        node.tracked_descendants_count = node
                            .tracked_descendants_count
                            .checked_sub(1)
                            .expect(
                                "tracked_descendants_count should be >= 0",
                            );
                    }

                    // Directory caches must be invalidated when removing a
                    // child node
                    if removed {
                        if let NodeData::CachedDirectory { .. } = &node.data {
                            node.data = NodeData::None
                        }
                    }
                } else {
                    return Ok(None);
                }
            } else {
                let entry = node.data.as_entry();
                let was_tracked = entry.map_or(false, |entry| entry.tracked());
                let had_entry = entry.is_some();
                if had_entry {
                    node.data = NodeData::None
                }
                let mut had_copy_source = false;
                if let Some(source) = &node.copy_source {
                    DirstateMap::count_dropped_path(unreachable_bytes, source);
                    had_copy_source = true;
                    node.copy_source = None
                }
                dropped = Dropped {
                    was_tracked,
                    had_entry,
                    had_copy_source,
                };
            }
            // After recursion, for both leaf (rest_of_path is None) nodes and
            // parent nodes, remove a node if it just became empty.
            let remove = !node.data.has_entry()
                && node.copy_source.is_none()
                && node.children.is_empty();
            if remove {
                let (key, _) =
                    nodes.remove_entry(first_path_component).unwrap();
                DirstateMap::count_dropped_path(
                    unreachable_bytes,
                    key.full_path(),
                )
            }
            Ok(Some((dropped, remove)))
        }

        self.with_dmap_mut(|map| {
            if let Some((dropped, _removed)) = recur(
                map.on_disk,
                &mut map.unreachable_bytes,
                &mut map.root,
                filename,
            )? {
                if dropped.had_entry {
                    map.nodes_with_entry_count = map
                        .nodes_with_entry_count
                        .checked_sub(1)
                        .expect("nodes_with_entry_count should be >= 0");
                }
                if dropped.had_copy_source {
                    map.nodes_with_copy_source_count = map
                        .nodes_with_copy_source_count
                        .checked_sub(1)
                        .expect("nodes_with_copy_source_count should be >= 0");
                }
            } else {
                debug_assert!(!was_tracked);
            }
            Ok(())
        })
    }

    pub fn has_tracked_dir(
        &mut self,
        directory: &HgPath,
    ) -> Result<bool, DirstateError> {
        self.with_dmap_mut(|map| {
            if let Some(node) = map.get_node(directory)? {
                // A node without a `DirstateEntry` was created to hold child
                // nodes, and is therefore a directory.
                let state = node.state()?;
                Ok(state.is_none() && node.tracked_descendants_count() > 0)
            } else {
                Ok(false)
            }
        })
    }

    pub fn has_dir(
        &mut self,
        directory: &HgPath,
    ) -> Result<bool, DirstateError> {
        self.with_dmap_mut(|map| {
            if let Some(node) = map.get_node(directory)? {
                // A node without a `DirstateEntry` was created to hold child
                // nodes, and is therefore a directory.
                let state = node.state()?;
                Ok(state.is_none() && node.descendants_with_entry_count() > 0)
            } else {
                Ok(false)
            }
        })
    }

    #[timed]
    pub fn pack_v1(
        &self,
        parents: DirstateParents,
    ) -> Result<Vec<u8>, DirstateError> {
        let map = self.get_map();
        // Optizimation (to be measured?): pre-compute size to avoid `Vec`
        // reallocations
        let mut size = parents.as_bytes().len();
        for node in map.iter_nodes() {
            let node = node?;
            if node.entry()?.is_some() {
                size += packed_entry_size(
                    node.full_path(map.on_disk)?,
                    node.copy_source(map.on_disk)?,
                );
            }
        }

        let mut packed = Vec::with_capacity(size);
        packed.extend(parents.as_bytes());

        for node in map.iter_nodes() {
            let node = node?;
            if let Some(entry) = node.entry()? {
                pack_entry(
                    node.full_path(map.on_disk)?,
                    &entry,
                    node.copy_source(map.on_disk)?,
                    &mut packed,
                );
            }
        }
        Ok(packed)
    }

    /// Returns new data and metadata together with whether that data should be
    /// appended to the existing data file whose content is at
    /// `map.on_disk` (true), instead of written to a new data file
    /// (false).
    #[timed]
    pub fn pack_v2(
        &self,
        can_append: bool,
    ) -> Result<(Vec<u8>, on_disk::TreeMetadata, bool), DirstateError> {
        let map = self.get_map();
        on_disk::write(map, can_append)
    }

    /// `callback` allows the caller to process and do something with the
    /// results of the status. This is needed to do so efficiently (i.e.
    /// without cloning the `DirstateStatus` object with its paths) because
    /// we need to borrow from `Self`.
    pub fn with_status<R>(
        &mut self,
        matcher: &(dyn Matcher + Sync),
        root_dir: PathBuf,
        ignore_files: Vec<PathBuf>,
        options: StatusOptions,
        callback: impl for<'r> FnOnce(
            Result<(DirstateStatus<'r>, Vec<PatternFileWarning>), StatusError>,
        ) -> R,
    ) -> R {
        self.with_dmap_mut(|map| {
            callback(super::status::status(
                map,
                matcher,
                root_dir,
                ignore_files,
                options,
            ))
        })
    }

    pub fn copy_map_len(&self) -> usize {
        let map = self.get_map();
        map.nodes_with_copy_source_count as usize
    }

    pub fn copy_map_iter(&self) -> CopyMapIter<'_> {
        let map = self.get_map();
        Box::new(filter_map_results(map.iter_nodes(), move |node| {
            Ok(if let Some(source) = node.copy_source(map.on_disk)? {
                Some((node.full_path(map.on_disk)?, source))
            } else {
                None
            })
        }))
    }

    pub fn copy_map_contains_key(
        &self,
        key: &HgPath,
    ) -> Result<bool, DirstateV2ParseError> {
        let map = self.get_map();
        Ok(if let Some(node) = map.get_node(key)? {
            node.has_copy_source()
        } else {
            false
        })
    }

    pub fn copy_map_get(
        &self,
        key: &HgPath,
    ) -> Result<Option<&HgPath>, DirstateV2ParseError> {
        let map = self.get_map();
        if let Some(node) = map.get_node(key)? {
            if let Some(source) = node.copy_source(map.on_disk)? {
                return Ok(Some(source));
            }
        }
        Ok(None)
    }

    pub fn copy_map_remove(
        &mut self,
        key: &HgPath,
    ) -> Result<Option<HgPathBuf>, DirstateV2ParseError> {
        self.with_dmap_mut(|map| {
            let count = &mut map.nodes_with_copy_source_count;
            let unreachable_bytes = &mut map.unreachable_bytes;
            Ok(DirstateMap::get_node_mut(
                map.on_disk,
                unreachable_bytes,
                &mut map.root,
                key,
            )?
            .and_then(|node| {
                if let Some(source) = &node.copy_source {
                    *count -= 1;
                    DirstateMap::count_dropped_path(unreachable_bytes, source);
                }
                node.copy_source.take().map(Cow::into_owned)
            }))
        })
    }

    pub fn copy_map_insert(
        &mut self,
        key: HgPathBuf,
        value: HgPathBuf,
    ) -> Result<Option<HgPathBuf>, DirstateV2ParseError> {
        self.with_dmap_mut(|map| {
            let node = DirstateMap::get_or_insert_node(
                map.on_disk,
                &mut map.unreachable_bytes,
                &mut map.root,
                &key,
                WithBasename::to_cow_owned,
                |_ancestor| {},
            )?;
            if node.copy_source.is_none() {
                map.nodes_with_copy_source_count += 1
            }
            Ok(node.copy_source.replace(value.into()).map(Cow::into_owned))
        })
    }

    pub fn len(&self) -> usize {
        let map = self.get_map();
        map.nodes_with_entry_count as usize
    }

    pub fn contains_key(
        &self,
        key: &HgPath,
    ) -> Result<bool, DirstateV2ParseError> {
        Ok(self.get(key)?.is_some())
    }

    pub fn get(
        &self,
        key: &HgPath,
    ) -> Result<Option<DirstateEntry>, DirstateV2ParseError> {
        let map = self.get_map();
        Ok(if let Some(node) = map.get_node(key)? {
            node.entry()?
        } else {
            None
        })
    }

    pub fn iter(&self) -> StateMapIter<'_> {
        let map = self.get_map();
        Box::new(filter_map_results(map.iter_nodes(), move |node| {
            Ok(if let Some(entry) = node.entry()? {
                Some((node.full_path(map.on_disk)?, entry))
            } else {
                None
            })
        }))
    }

    pub fn iter_tracked_dirs(
        &mut self,
    ) -> Result<
        Box<
            dyn Iterator<Item = Result<&HgPath, DirstateV2ParseError>>
                + Send
                + '_,
        >,
        DirstateError,
    > {
        let map = self.get_map();
        let on_disk = map.on_disk;
        Ok(Box::new(filter_map_results(
            map.iter_nodes(),
            move |node| {
                Ok(if node.tracked_descendants_count() > 0 {
                    Some(node.full_path(on_disk)?)
                } else {
                    None
                })
            },
        )))
    }

    pub fn debug_iter(
        &self,
        all: bool,
    ) -> Box<
        dyn Iterator<
                Item = Result<
                    (&HgPath, (u8, i32, i32, i32)),
                    DirstateV2ParseError,
                >,
            > + Send
            + '_,
    > {
        let map = self.get_map();
        Box::new(filter_map_results(map.iter_nodes(), move |node| {
            let debug_tuple = if let Some(entry) = node.entry()? {
                entry.debug_tuple()
            } else if !all {
                return Ok(None);
            } else if let Some(mtime) = node.cached_directory_mtime()? {
                (b' ', 0, -1, mtime.truncated_seconds() as i32)
            } else {
                (b' ', 0, -1, -1)
            };
            Ok(Some((node.full_path(map.on_disk)?, debug_tuple)))
        }))
    }
}
