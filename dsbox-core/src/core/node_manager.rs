use crate::core::node::{Node, NodeId};
use crate::process::ProcessEventOrExit;
use std::collections::hash_map::{self, Entry};
use std::collections::HashMap;
use std::fmt::Display;
use std::future::Future;
use std::iter::Map;
use std::ops::{Index, IndexMut};
use std::pin::Pin;
use std::task::{Context, Poll};

pub(super) struct NodeManager {
    by_name: HashMap<String, NodeId>,
    by_id: HashMap<NodeId, Node>,
}

type Iter<'a> = Map<hash_map::Iter<'a, NodeId, Node>, fn((&'a NodeId, &'a Node)) -> &'a Node>;
type IterMut<'a> =
    Map<hash_map::IterMut<'a, NodeId, Node>, fn((&'a NodeId, &'a mut Node)) -> &'a mut Node>;
type IntoIter = Map<hash_map::IntoIter<NodeId, Node>, fn((NodeId, Node)) -> Node>;

#[derive(Copy, Clone, Debug)]
pub struct DuplicateName;

impl NodeManager {
    pub fn new() -> Self {
        Self {
            by_name: HashMap::new(),
            by_id: HashMap::new(),
        }
    }

    pub fn add(&mut self, node: Node) -> Result<&mut Node, DuplicateName> {
        let node_id = node.id;
        debug_assert!(
            !self.by_id.contains_key(&node_id),
            "tried to insert node with existing NodeId"
        );
        match self.by_name.entry(node.name.clone()) {
            Entry::Occupied(_) => return Err(DuplicateName),
            Entry::Vacant(entry) => entry.insert(node_id),
        };
        self.by_id.insert(node_id, node);
        Ok(self.by_id.get_mut(&node_id).unwrap())
    }

    pub fn add_alias(
        &mut self,
        for_id: NodeId,
        name: String,
    ) -> Result<Option<&mut Node>, DuplicateName> {
        if !self.by_id.contains_key(&for_id) {
            return Ok(None);
        };
        match self.by_name.entry(name) {
            Entry::Occupied(_) => return Err(DuplicateName),
            Entry::Vacant(entry) => entry.insert(for_id),
        };
        Ok(Some(self.by_id.get_mut(&for_id).unwrap()))
    }

    /// remove a node and all of its aliases
    pub fn remove(&mut self, node_id: NodeId) {
        let Some(node) = self.by_id.remove(&node_id) else {
            return;
        };
        self.by_name.retain(|_, id| *id != node.id);
    }

    /// remove all aliases of a node and returns the removed aliases
    pub fn remove_aliases_of(&mut self, node_id: NodeId) -> Vec<String> {
        let Some(node) = self.by_id.get(&node_id) else {
            return Vec::new();
        };
        let original_name = &node.name;
        let mut aliases = self.aliases_of(node_id);
        for i in (0..aliases.len()).rev() {
            if &aliases[i] != original_name {
                self.by_name.remove(&aliases[i]);
            } else {
                aliases.swap_remove(i);
            }
        }
        aliases
    }

    pub fn get(&self, node_id: NodeId) -> Option<&Node> {
        self.by_id.get(&node_id)
    }

    pub fn lookup(&self, name: &str) -> Option<NodeId> {
        self.by_name.get(name).copied()
    }

    /// check if `name` is an alias for `node_id`
    pub fn has_alias(&self, node_id: NodeId, name: &str) -> bool {
        let Some(other) = self.lookup(name) else {
            return false;
        };
        node_id == other
    }

    pub fn aliases_of(&self, target: NodeId) -> Vec<String> {
        let mut aliases = Vec::new();
        for (name, id) in &self.by_name {
            if *id == target {
                aliases.push(name.clone());
            }
        }
        aliases
    }

    pub fn iter(&self) -> Iter<'_> {
        self.by_id.iter().map(|(_, node)| node)
    }

    pub fn iter_mut(&mut self) -> IterMut<'_> {
        self.by_id.iter_mut().map(|(_, node)| node)
    }

    pub fn recv_any<'a>(
        &'a mut self,
    ) -> impl Future<Output = Option<(ProcessEventOrExit, NodeId)>> + Unpin + 'a {
        RecvAny {
            nodes: &mut self.by_id,
        }
    }
}

impl Index<NodeId> for NodeManager {
    type Output = Node;

    fn index(&self, index: NodeId) -> &Self::Output {
        self.by_id.get(&index).unwrap()
    }
}

impl IndexMut<NodeId> for NodeManager {
    fn index_mut(&mut self, index: NodeId) -> &mut Self::Output {
        self.by_id.get_mut(&index).unwrap()
    }
}

impl IntoIterator for NodeManager {
    type Item = Node;
    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.by_id.into_iter().map(|(_, node)| node)
    }
}

impl<'a> IntoIterator for &'a NodeManager {
    type Item = &'a Node;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a mut NodeManager {
    type Item = &'a mut Node;
    type IntoIter = IterMut<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

pub(super) struct RecvAny<'a> {
    nodes: &'a mut HashMap<NodeId, Node>,
}

impl<'a> Future for RecvAny<'a> {
    type Output = Option<(ProcessEventOrExit, NodeId)>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut num_closed = 0;
        for (_, node) in self.nodes.iter_mut() {
            match node.poll_recv(cx) {
                Poll::Ready(Some(event)) => {
                    return Poll::Ready(Some((event, node.id)));
                }
                Poll::Ready(None) => num_closed += 1,
                _ => {}
            }
        }
        if num_closed < self.nodes.len() {
            Poll::Pending
        } else {
            Poll::Ready(None)
        }
    }
}

impl Display for DuplicateName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("duplicate node name")
    }
}

impl std::error::Error for DuplicateName {}
