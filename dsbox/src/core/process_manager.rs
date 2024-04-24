//! Manages running processes for the [`Core`](crate::core::Core)

use std::collections::HashMap;
use std::ops::{Index, IndexMut, RangeBounds};
use std::slice::{Iter, IterMut, SliceIndex};
use std::vec::{Drain, IntoIter};

use tokio::sync::mpsc::Sender;

use crate::process::{Launcher, Process, ProcessEvent};

/// Manages running processes and their corresponding node names
///
/// Each running process may have one (or more) name(s) associated with it.
/// Server processes should have exactly one name, because a server process implements a single node in the system.
/// The (singular) client process may have multiple names, because all client nodes are implemented in a single process.
pub struct ProcessManager {
    /// A [`Launcher`] that is used for launching new processes
    launcher: Launcher,
    /// A list of all launched processes (running and exited). The index into this [`Vec`] is the
    /// unique id of each process. For this reason, "old" processes (that have exited) are not removed from the list
    /// (this is ok, since [`Process`] is just a lightweight handle).
    processes: Vec<Process>,
    /// a map of node names and their corresponding process id
    names: HashMap<String, usize>,
    /// this sender is cloned for each new process that is launched and is passed to the process,
    /// so that it uses it to send [`ProcessEvent`]s to the running [`Core`](crate::core::Core).
    sender: Sender<ProcessEvent>,
}

impl ProcessManager {
    /// Creates a new [`ProcessManager`], with a given [`Sender`] that is passed to newly launched processes.
    pub fn new(sender: Sender<ProcessEvent>) -> Self {
        Self { launcher: Launcher::new(), processes: Vec::new(), names: HashMap::new(), sender }
    }

    /// Launches a new process from the given command. This can be just a path to an executable
    /// or a more complex command, like "python server.py"
    /// Returns [`Ok`] with the new processes id, if the process was launched successfully,
    /// otherwise returns [`Err`] with underlying error.
    pub async fn launch(&mut self, command: &str, name: String) -> std::io::Result<usize> {
        let id = self.processes.len();
        let process = self.launcher.launch(command, &self.sender, id, name).await?;
        self.processes.push(process);
        Ok(id)
    }

    /// adds a name to the given process
    pub fn add_name(&mut self, name: String, id: usize) -> Option<usize> {
        self.names.insert(name, id)
    }

    /// Returns `true` if the given process has the given name associated with it
    pub fn has_name(&self, process: &Process, name: &str) -> bool {
        self.names.get(name).copied() == Some(process.id())
    }

    /// Returns the id of the nodes process with the given name, or `None` if it does not exist.
    pub fn id_by_name(&mut self, name: &str) -> Option<usize> {
        self.names.get(name).copied()
    }

    pub fn iter(&self) -> Iter<Process> {
        self.into_iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<Process> {
        self.into_iter()
    }

    /// Clears all given names for all processes
    pub fn reset_names(&mut self) {
        self.names.clear();
    }

    /// drains (removes) all processes in the given range. Especially useful when you want to terminate
    /// said processes
    pub fn drain(&mut self, range: impl RangeBounds<usize>) -> Drain<'_, Process> {
        self.processes.drain(range)
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