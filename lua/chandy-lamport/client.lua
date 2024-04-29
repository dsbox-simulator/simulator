local dsbox = require('dsbox');

function table.shuffle(table)
    for i = #table, 2, -1 do
        local j            = math.random(i)
        table[i], table[j] = table[j], table[i]
    end
end

function table.copy(table)
    local new = {}
    for k, v in ipairs(table) do
        new[k] = v
    end
    return new
end

local num_servers = 5
local num_tokens = 20

local server_names = {}
for i = 1, num_servers do
    server_names[i] = string.format("s%d", i)
end

dsbox.Message:new("client", "core", "setup", { clients = { "c0" }, servers = server_names }):send()
assert(dsbox.recv().body.type == "setup_ok")
math.randomseed(12345)

dsbox.log("creating handoff order")
for _, server in ipairs(server_names) do
    local order = table.copy(server_names)
    table.shuffle(order)
    dsbox.Message:new("c0", server, "handoff_order", { order = order }):send()
end

dsbox.log("distributing tokens")
for _, server in ipairs(server_names) do
    for _ = 1, num_tokens / num_servers do
        dsbox.Message:new("c0", server, "token"):send()
    end
end

while true do
    dsbox.sleep(10)
end
