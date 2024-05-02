local dsbox = require('dsbox')
local init = dsbox.recv()
assert(init.body.type == "init")
local own_name = init.body.name
local servers = init.body.servers

local tasks = {}

local function do_some_work()
    dsbox.sleep(0.1 + math.random() * 0.1)
    if #tasks > 0 then
        local next_server = servers[math.random(#servers)]
        dsbox.Message:send(own_name, next_server, "task", { task = table.remove(tasks, 1) })
    end
end

while true do
    local message = dsbox.recv(0.0)
    if message == nil then
        do_some_work()
    elseif message.body.type == "task" then
        tasks[#tasks + 1] = message.body.task
    elseif message.body.type == "state" then
        message:reply("state", { state = tasks })
    end
end
