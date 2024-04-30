local dsbox = require("dsbox")

dsbox.Message:send("client", "core", "setup", {
    clients = { "c" },
    servers = { "s" },
    middleware_before = { "lua/echo/middleware.lua first", "lua/echo/middleware.lua second" },
    middleware_after = { "lua/echo/middleware.lua third", "lua/echo/middleware.lua last" }
})

assert(dsbox.recv().body.type == "setup_ok")

dsbox.sleep(1);

dsbox.Message:send("c", "s", "echo", { echo = "Hello, World!" })
local response = dsbox.recv()
assert(response.body.type == "echo_ok")
assert(response.body.echo == "Hello, World!")