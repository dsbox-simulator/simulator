use crate::core::Core;
use libproto::system::Command;

const DEFAULT_CORE_NAME: &'static str = "core";
const DEFAULT_TEST_NODE_NAME: &'static str = "test";

/// A builder for a [`Core`].
pub struct Builder {
    pub(super) test_command: Command,
    pub(super) server_command: Command,
    pub(super) interactive: bool,
    pub(super) allow_lua_unsafe: bool,
    pub(super) core_name: String,
    pub(super) test_node_name: String,
}

impl Builder {
    /// Create a new builder with the specified commands for the test and server nodes
    pub(super) fn new(test_command: Command, server_command: Command) -> Self {
        Self {
            test_command,
            server_command,
            interactive: false,
            allow_lua_unsafe: false,
            core_name: DEFAULT_CORE_NAME.to_string(),
            test_node_name: DEFAULT_TEST_NODE_NAME.to_string(),
        }
    }

    /// change the command for the test node
    pub fn test_command(mut self, test_command: Command) -> Self {
        self.test_command = test_command;
        self
    }

    /// change the command for the server nodes
    pub fn server_command(mut self, server_command: Command) -> Self {
        self.server_command = server_command;
        self
    }

    /// enable/disable interactive mode
    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    /// enable/disable unsafe lua libraries
    pub fn allow_lua_unsafe(mut self, allow_lua_unsafe: bool) -> Self {
        self.allow_lua_unsafe = allow_lua_unsafe;
        self
    }

    /// override the name of the simulation core (default: `"core"`)
    pub fn core_name(mut self, core_name: impl Into<String>) -> Self {
        self.core_name = core_name.into();
        self
    }

    /// override the name of the test node (default: `"test"`)
    pub fn test_node_name(mut self, test_node_name: impl Into<String>) -> Self {
        self.test_node_name = test_node_name.into();
        self
    }

    /// finish building and create a [`Core`]
    pub fn build(self) -> Core {
        Core::from(self)
    }
}
