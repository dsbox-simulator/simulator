use std::collections::HashMap;
use std::future::Future;
use std::iter::FusedIterator;
use std::ops::{Index, IndexMut, RangeBounds};
use std::pin::Pin;
use std::slice::{Iter, IterMut, SliceIndex};
use std::task::{Context, Poll};
use std::vec::IntoIter;

use crate::core::node::Node;
use crate::process::ProcessEvent;

pub struct NodeList {
    nodes: Vec<Node>,
    names: HashMap<String, usize>,
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

    pub fn push(&mut self, mut node: Node) -> &mut Node {
        node.id = self.len();
        self.names.insert(node.name.clone(), self.len());
        self.nodes.push(node);
        self.nodes.last_mut().unwrap()
    }

    pub fn pop(&mut self) -> Option<Node> {
        let node = self.nodes.pop()?;
        self.names.remove(&node.name);
        Some(node)
    }
    pub fn lookup(&self, name: &str) -> Option<&Node> {
        Some(&self.nodes[*self.names.get(name)?])
    }
    pub fn lookup_mut(&mut self, name: &str) -> Option<&mut Node> {
        Some(&mut self.nodes[*self.names.get(name)?])
    }

    pub fn is_alias_of(&self, node: &Node, name: &str) -> bool {
        let Some(id) = self.names.get(name) else { return false; };
        self.nodes[*id].is_same_process(node)
    }

    pub fn iter(&self) -> Iter<'_, Node> {
        self.nodes.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, Node> {
        self.nodes.iter_mut()
    }

    pub fn drain(&mut self, range: impl RangeBounds<usize>) -> Drain {
        Drain {
            names: &mut self.names,
            inner: self.nodes.drain(range),
        }
    }

    pub fn recv_any<'a>(&'a self) -> impl Future<Output=Option<(ProcessEvent, bool, usize)>> + Unpin + 'a {
        RecvAny {
            nodes: &self.nodes,
        }
    }
}

impl<I> Index<I> for NodeList
    where I: SliceIndex<[Node]> {
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.nodes.index(index)
    }
}

impl<I> IndexMut<I> for NodeList
    where I: SliceIndex<[Node]> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.nodes.index_mut(index)
    }
}

impl IntoIterator for NodeList {
    type Item = Node;
    type IntoIter = IntoIter<Node>;

    fn into_iter(self) -> Self::IntoIter {
        self.nodes.into_iter()
    }
}

impl<'a> IntoIterator for &'a NodeList {
    type Item = &'a Node;
    type IntoIter = Iter<'a, Node>;

    fn into_iter(self) -> Self::IntoIter {
        self.nodes.iter()
    }
}

impl<'a> IntoIterator for &'a mut NodeList {
    type Item = &'a mut Node;
    type IntoIter = IterMut<'a, Node>;

    fn into_iter(self) -> Self::IntoIter {
        self.nodes.iter_mut()
    }
}

pub struct Drain<'a> {
    names: &'a mut HashMap<String, usize>,
    inner: std::vec::Drain<'a, Node>,
}

impl<'a> Iterator for Drain<'a> {
    type Item = Node;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.inner.next()?;
        self.names.remove(&next.name);
        Some(next)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a> DoubleEndedIterator for Drain<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let next = self.inner.next_back()?;
        self.names.remove(&next.name);
        Some(next)
    }
}

impl<'a> ExactSizeIterator for Drain<'a> {}

impl<'a> FusedIterator for Drain<'a> {}

pub struct RecvAny<'a> {
    nodes: &'a [Node],
}

impl<'a> Future for RecvAny<'a> {
    type Output = Option<(ProcessEvent, bool, usize)>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut num_closed = 0;
        for (idx, node) in self.nodes.iter().enumerate() {
            let pinned = std::pin::pin!(node.recv());
            match pinned.poll(cx) {
                Poll::Ready((Some(event), from_proxy)) => return Poll::Ready(Some((event, from_proxy, idx))),
                Poll::Ready((None, _)) => num_closed += 1,
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