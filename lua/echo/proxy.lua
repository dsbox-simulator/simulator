local dsbox = require("dsbox")

local init = dsbox.recv()
assert(init.body.type == "init")
local own_name = init.body.name

local function forward(message)
    if message.body.type == "proxy" then
        -- forward a message from the actual node to its destination
        dsbox.send(message.body.message)
    else
        -- forward a message from the outside to the actual node
        dsbox.Message:send(own_name, "core", "proxy", { message = message })
    end
end

forward(init)
for message in dsbox.recv_iter() do
    dsbox.log("PROXY", message)
    forward(message)
end