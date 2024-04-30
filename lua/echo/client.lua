local dsbox = require("dsbox")

dsbox.Message:send("client", "core", "setup", { clients = { "c" }, servers = { "s" }, proxy = "lua/echo/proxy.lua" })
assert(dsbox.recv().body.type == "setup_ok")

dsbox.Message:send("c", "s", "echo", { echo = "Hello, World!" })
local response = dsbox.recv()
assert(response.body.type == "echo_ok")
assert(response.body.echo == "Hello, World!")