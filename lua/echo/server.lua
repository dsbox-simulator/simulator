local dsbox = require("dsbox")

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

forward(init)
for message in dsbox.recv_iter() do
    print(string.format("[SERVER] %s", message))
    forward(message)
end