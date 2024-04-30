local dsbox = require("dsbox")

local middleware_type = dsbox.args[1]
dsbox.log(middleware_type)

local init = dsbox.recv()
assert(init.body.type == "init")
local own_name = init.body.name

local function forward(message)
    if message.body.type == "forward" then
        -- forward a message from the process in the middleware stack below to the process in the middleware stack above
        dsbox.send(message.body.message)
    else
        -- forward a message from the process in the middleware stack above to the process in the middleware stack below
        dsbox.Message:send(own_name, "core", "next", { message = message })
    end
end

if middleware_type ~= "last" then
    forward(init)
    for message in dsbox.recv_iter() do
        dsbox.log(string.format("[MIDDLEWARE %s]", middleware_type), message)
        forward(message)
    end
else
    for message in dsbox.recv_iter() do
        message:reply("echo_ok", { echo = message.body.echo })
    end
end