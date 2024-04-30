local dsbox = require('dsbox')
local init = dsbox.recv()
assert(init.body.type == "init")
local own_name = init.body.name

local handoff_order = dsbox.recv()
assert(handoff_order.body.type == "handoff_order")

local handoff_sequence = { current = 1, order = handoff_order.body.order }
function handoff_sequence:next()
    local next = self.order[self.current]
    self.current = self.current + 1
    if self.current > #self.order then
        self.current = 1
    end
    return next
end

dsbox.log("handoff order:", handoff_sequence.order)

local num_tokens = 0

local function do_some_work()
    dsbox.sleep(1 + math.random())
    if num_tokens > 0 then
        local next_server = handoff_sequence:next()
    	dsbox.Message:send(own_name, next_server, "token")
    	num_tokens = num_tokens - 1
    end
end

while true do
    local message = dsbox.recv(0.0)
    if message == nil then
    	do_some_work()
    else
        assert(message.body.type == "token")
        dsbox.log("got token from", message.src)
        num_tokens = num_tokens + 1
    end
end
