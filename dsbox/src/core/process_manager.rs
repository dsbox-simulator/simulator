use std::collections::HashMap;
use std::ops::{Index, IndexMut};
use std::path::Path;
use std::slice::{Iter, IterMut, SliceIndex};
use std::vec::IntoIter;

use crossbeam_channel::Sender;

use crate::process::{Launcher, Process, ProcessEvent};

pub struct ProcessManager {
    launcher: Launcher,
    processes: Vec<Process>,
    names: HashMap<String, usize>,
    sender: Sender<ProcessEvent>,
}

impl ProcessManager {
    pub fn new(sender: Sender<ProcessEvent>) -> Self {
        Self { launcher: Launcher::new(), processes: Vec::new(), names: HashMap::new(), sender }
    }

    pub fn spawn(&mut self, file: &Path) -> std::io::Result<usize> {
        let id = self.processes.len();
        let process = self.launcher.spawn(file, &self.sender, id)?;
        self.processes.push(process);
        Ok(id)
    }

    pub fn add_name(&mut self, name: String, id: usize) -> Option<usize> {
        self.names.insert(name, id)
    }

    pub fn has_name(&self, process: &Process, name: &str) -> bool {
        self.names.get(name).copied() == Some(process.id())
    }

    pub fn id_by_name(&mut self, name: &str) -> Option<usize> {
        self.names.get(name).copied()
    }

    pub fn iter(&self) -> Iter<Process> {
        self.into_iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<Process> {
        self.into_iter()
    }

    pub fn reset_names(&mut self) {
        self.names.clear();
    }
}

impl<I> Index<I> for ProcessManager
    where I: SliceIndex<[Process]> {
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.processes.index(index)
    }
}

impl<I> IndexMut<I> for ProcessManager
    where I: SliceIndex<[Process]> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.processes.index_mut(index)
    }
}

impl IntoIterator for ProcessManager {
    type Item = Process;
    type IntoIter = IntoIter<Process>;

    fn into_iter(self) -> Self::IntoIter {
        self.processes.into_iter()
    }
}

impl<'a> IntoIterator for &'a ProcessManager {
    type Item = &'a Process;
    type IntoIter = Iter<'a, Process>;

    fn into_iter(self) -> Self::IntoIter {
        self.processes.iter()
    }
}

impl<'a> IntoIterator for &'a mut ProcessManager {
    type Item = &'a mut Process;
    type IntoIter = IterMut<'a, Process>;

    fn into_iter(self) -> Self::IntoIter {
        self.processes.iter_mut()
    }
}