use crate::node::NodeId;
use crate::FxHashSet;

/// A frozen, sorted set of `NodeId`s for read-only edge storage.
///
/// After postprocessing, edges are never mutated — only iterated, counted,
/// and membership-tested. `CompactEdgeSet` stores targets as a sorted
/// `Vec<NodeId>`, giving sequential memory layout (cache-friendly iteration)
/// and `O(log n)` binary-search `contains`.
#[derive(Debug, Clone, Default)]
pub struct CompactEdgeSet {
    ids: Vec<NodeId>,
}

impl CompactEdgeSet {
    #[inline]
    pub fn contains(&self, id: &NodeId) -> bool {
        self.ids.binary_search(id).is_ok()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    #[inline]
    pub fn iter(&self) -> std::slice::Iter<'_, NodeId> {
        self.ids.iter()
    }
}

impl From<FxHashSet<NodeId>> for CompactEdgeSet {
    fn from(set: FxHashSet<NodeId>) -> Self {
        let mut ids: Vec<NodeId> = set.into_iter().collect();
        ids.sort_unstable();
        Self { ids }
    }
}

impl<'a> IntoIterator for &'a CompactEdgeSet {
    type Item = &'a NodeId;
    type IntoIter = std::slice::Iter<'a, NodeId>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.ids.iter()
    }
}
