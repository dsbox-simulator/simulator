use crate::core::node::{Node, NodeId};
use crate::process::ProcessEvent;
use std::collections::HashMap;
use std::future::Future;
use std::iter::FusedIterator;
use std::ops::{Index, IndexMut, RangeBounds};
use std::pin::Pin;
use std::slice::{Iter, IterMut, SliceIndex};
use std::task::{Context, Poll};
use std::vec::IntoIter;

pub struct NodeList {
    nodes: Vec<NodeRef>,
    names: HashMap<String, usize>,
}

pub enum NodeRef {
    Node(Node),
    Alias { name: String, id: NodeId },
}

impl NodeList {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            names: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn add(&mut self, mut node: Node) -> &mut Node {
        node.id = NodeId(self.len());
        self.names.insert(node.name.clone(), self.len());
        self.nodes.push(NodeRef::Node(node));
        self.nodes.last_mut().unwrap().as_node_mut().unwrap()
    }

    pub fn add_alias(&mut self, for_id: NodeId, name: String) -> (NodeId, &mut Node) {
        let alias_id = NodeId(self.len());
        self.names.insert(name.clone(), self.len());
        let target_node = self.resolve_alias(for_id);
        self.nodes.push(NodeRef::Alias {
            name,
            id: target_node.id,
        });
        (alias_id, self.resolve_alias_mut(for_id))
    }

    pub fn lookup(&self, name: &str) -> Option<NodeId> {
        Some(NodeId(*self.names.get(name)?))
    }

    pub fn lookup_and_resolve(&self, name: &str) -> Option<&Node> {
        Some(self.resolve_alias(NodeId(*self.names.get(name)?)))
    }

    pub fn lookup_and_resolve_mut(&mut self, name: &str) -> Option<&mut Node> {
        Some(self.resolve_alias_mut(NodeId(*self.names.get(name)?)))
    }

    /// returns `true` if `node` and the node with the given `name` reference the same actual node
    /// resolving any aliases for `node` and `name`
    pub fn alias_same_node(&self, node: &NodeRef, name: &str) -> bool {
        let left = match node {
            NodeRef::Node(node) => node,
            NodeRef::Alias { id, .. } => self.resolve_alias(*id),
        };
        let Some(id) = self.names.get(name) else {
            return false;
        };
        let right = self.resolve_alias(NodeId(*id));
        std::ptr::eq(left, right)
    }

    pub fn aliases_of(&self, target: &NodeRef) -> Vec<String> {
        let target = match target {
            NodeRef::Node(node) => node,
            NodeRef::Alias { id, .. } => self.resolve_alias(*id),
        };
        let mut aliases = Vec::new();
        for node in &self.nodes {
            match node {
                NodeRef::Node(node) if node.id == target.id => aliases.push(node.name.clone()),
                NodeRef::Alias { id, name } if *id == target.id => aliases.push(name.clone()),
                _ => {}
            }
        }
        aliases
    }

    pub fn iter(&self) -> Iter<'_, NodeRef> {
        self.nodes.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, NodeRef> {
        self.nodes.iter_mut()
    }

    pub fn drain(&mut self, range: impl RangeBounds<usize>) -> Drain<'_> {
        Drain {
            names: &mut self.names,
            inner: self.nodes.drain(range),
        }
    }

    pub fn recv_any<'a>(
        &'a mut self,
    ) -> impl Future<Output = Option<(ProcessEvent, NodeId)>> + Unpin + 'a {
        RecvAny {
            nodes: &mut self.nodes,
        }
    }

    pub fn resolve_alias(&self, id: NodeId) -> &Node {
        let mut index = id.0;
        loop {
            match &self.nodes[index] {
                NodeRef::Node(node) => return node,
                NodeRef::Alias { id, .. } => index = id.0,
            }
        }
    }

    pub fn resolve_alias_mut(&mut self, id: NodeId) -> &mut Node {
        // TODO: this deserves a cleaner implementation
        let mut index = id.0;
        loop {
            let node = &self.nodes[index];
            match node {
                NodeRef::Node(_) => break,
                NodeRef::Alias { id, .. } => index = id.0,
            }
        }
        self.nodes[index].as_node_mut().unwrap()
    }
}

impl NodeRef {
    pub fn is_node(&self) -> bool {
        matches!(self, Self::Node(_))
    }

    pub fn is_alias(&self) -> bool {
        matches!(self, Self::Alias { .. })
    }

    pub fn name(&self) -> &str {
        match self {
            NodeRef::Node(node) => &node.name,
            NodeRef::Alias { name, .. } => name,
        }
    }

    pub fn id(&self) -> NodeId {
        match self {
            NodeRef::Node(node) => node.id,
            NodeRef::Alias { id, .. } => *id,
        }
    }

    pub fn into_node(self) -> Option<Node> {
        match self {
            NodeRef::Node(node) => Some(node),
            NodeRef::Alias { .. } => None,
        }
    }
    pub fn as_node(&self) -> Option<&Node> {
        match self {
            NodeRef::Node(node) => Some(node),
            NodeRef::Alias { .. } => None,
        }
    }
    pub fn as_node_mut(&mut self) -> Option<&mut Node> {
        match self {
            NodeRef::Node(node) => Some(node),
            NodeRef::Alias { .. } => None,
        }
    }
}

impl<I> Index<I> for NodeList
where
    I: SliceIndex<[NodeRef]>,
{
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.nodes.index(index)
    }
}

impl<I> IndexMut<I> for NodeList
where
    I: SliceIndex<[NodeRef]>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.nodes.index_mut(index)
    }
}

impl Index<NodeId> for NodeList {
    type Output = NodeRef;

    fn index(&self, index: NodeId) -> &Self::Output {
        &self.nodes[index.0]
    }
}

impl IndexMut<NodeId> for NodeList {
    fn index_mut(&mut self, index: NodeId) -> &mut Self::Output {
        &mut self.nodes[index.0]
    }
}

impl IntoIterator for NodeList {
    type Item = NodeRef;
    type IntoIter = IntoIter<NodeRef>;

    fn into_iter(self) -> Self::IntoIter {
        self.nodes.into_iter()
    }
}

impl<'a> IntoIterator for &'a NodeList {
    type Item = &'a NodeRef;
    type IntoIter = Iter<'a, NodeRef>;

    fn into_iter(self) -> Self::IntoIter {
        self.nodes.iter()
    }
}

impl<'a> IntoIterator for &'a mut NodeList {
    type Item = &'a mut NodeRef;
    type IntoIter = IterMut<'a, NodeRef>;

    fn into_iter(self) -> Self::IntoIter {
        self.nodes.iter_mut()
    }
}

pub struct Drain<'a> {
    names: &'a mut HashMap<String, usize>,
    inner: std::vec::Drain<'a, NodeRef>,
}

impl<'a> Iterator for Drain<'a> {
    type Item = NodeRef;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next()? {
            NodeRef::Node(node) => {
                self.names.remove(&node.name);
                Some(NodeRef::Node(node))
            }
            NodeRef::Alias { name, id } => {
                self.names.remove(&name);
                Some(NodeRef::Alias { name, id })
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a> DoubleEndedIterator for Drain<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match self.inner.next_back()? {
            NodeRef::Node(node) => {
                self.names.remove(&node.name);
                Some(NodeRef::Node(node))
            }
            NodeRef::Alias { name, id } => {
                self.names.remove(&name);
                Some(NodeRef::Alias { name, id })
            }
        }
    }
}

impl<'a> ExactSizeIterator for Drain<'a> {}

impl<'a> FusedIterator for Drain<'a> {}

pub struct RecvAny<'a> {
    nodes: &'a mut [NodeRef],
}

impl<'a> Future for RecvAny<'a> {
    type Output = Option<(ProcessEvent, NodeId)>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut num_closed = 0;
        let num_open = self.nodes.iter().filter(|n| n.is_node()).count();
        for node in self.nodes.iter_mut() {
            let NodeRef::Node(node) = node else {
                continue;
            };
            match node.poll_recv(cx) {
                Poll::Ready(Some(event)) => {
                    return Poll::Ready(Some((event, node.id)));
                }
                Poll::Ready(None) => num_closed += 1,
                _ => {}
            }
        }
        if num_closed < num_open {
            Poll::Pending
        } else {
            Poll::Ready(None)
        }
    }
}
