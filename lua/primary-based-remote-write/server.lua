local dsbox = require("dsbox")

local message_queue = {}
local function recv()
    if #message_queue > 0 then
        return table.remove(message_queue, 1)
    else
        return dsbox.recv()
    end
end

local function recv_iter()
    return recv
end

local function wait_for(src, type)
    for i, message in ipairs(message_queue) do
        if message.src == src and message.body.type == type then
            return table.remove(message_queue, i)
        end
    end
    while true do
        local message = dsbox.recv()
        if message.src == src and message.body.type == type then
            return message
        else
            message_queue[#message_queue + 1] = message
        end
    end
end

local init = recv()
assert(init.body.type == "init")
local own_name = init.body.name
local all_servers = init.body.servers

local sequence = {}

function store_primary(message)
    for i = 2, #all_servers do
        dsbox.Message:send(own_name, all_servers[i], "update", { value = message.body.value })
    end
    sequence[#sequence + 1] = message.body.value
    for i = 2, #all_servers do
        wait_for(all_servers[i], "ack")
    end
    message:reply("ack")
end
function store_secondary(message)
    -- forward to primary
    message.src = own_name
    message.dest = all_servers[1]
    message:send()
end
function update(message)
    sequence[#sequence + 1] = message.body.value
    message:reply("ack")
end

local function run()
    message_sources = {}
    for message in recv_iter() do
        if message.body.type == "store" then
            if own_name == all_servers[1] then
                store_primary(message)
            else
                message_sources[message.body.msg_id] = message.src
                store_secondary(message)
            end
        elseif message.body.type == "update" then
            update(message)
        elseif message.body.type == "ack" then
            assert(own_name ~= all_servers[1])
            message.src = own_name
            message.dest = message_sources[message.body.in_reply_to]
            message.body.msg_id = nil
            message:send()
        elseif message.body.type == "load" then
            message:reply("load_ok", { sequence = sequence })
        end
    end
end
run()