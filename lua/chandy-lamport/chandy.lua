local dsbox = require("dsbox")
local mq = require("message_queue")
local init = mq.recv()
assert(init.body.type == "init")
local own_name = init.body.name
local servers = init.body.servers
local snapshot, recording_servers, reply_to_when_finished

local function forward(message)
    if message.body.type == "forward" then
        dsbox.send(message.body.message)
    else
        dsbox.Message:send(own_name, "core", "next", { message = message })
    end
end

local function get_state()
    dsbox.Message:send(own_name, "core", "next", { message = dsbox.Message:new(own_name, own_name, "state") })
    while true do
        local next_msg = mq.recv_filter(function(msg)
            return msg.src == "core" and msg.body.type == "forward"
        end)
        if next_msg.body.message.body.type == "state" then
            return next_msg.body.message.body.state
        else
            forward(next_msg)
        end
    end
end

local function begin_snapshot(src, reply_to)
    print("beginning snapshot")
    snapshot = { messages = {}, state = get_state() }
    recording_servers = {}
    reply_to_when_finished = reply_to
    for _, server in ipairs(servers) do
        if server ~= own_name and server ~= src then
            recording_servers[server] = true
        end
    end
    for _, server in ipairs(servers) do
        if server ~= own_name then
            dsbox.Message:send(own_name, server, "snapshot", { reply_to = reply_to })
        end
    end
end

local function stop_recording_from(src)
    recording_servers[src] = nil
    print(string.format("got marker from %s, outstanding = %s", src, dsbox.to_json(recording_servers)))
    local outstanding = 0
    for _, _ in pairs(recording_servers) do
        outstanding = outstanding + 1
    end
    if outstanding == 0 then
        dsbox.Message:send(own_name, reply_to_when_finished, "snapshot", { snapshot = snapshot })
        print("snapshot finished")
        snapshot = nil
        recording_servers = nil
        reply_to_when_finished = nil
    end
end

local function record_message(message)
    if not recording_servers[message.src] then
        return
    end
    print(string.format("recording message %s", message))
    snapshot.messages[#snapshot.messages + 1] = message
end

forward(init)
for message in mq.recv_iter() do
    if message.body.type == "snapshot" then
        if snapshot == nil then
            begin_snapshot(message.src, message.body.reply_to)
        else
            stop_recording_from(message.src)
        end
    else
        if snapshot ~= nil then
            record_message(message)
        end
        forward(message)
    end
end