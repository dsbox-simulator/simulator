local dsbox = require('dsbox');

local num_servers = 5

local server_names = {}
for i = 1, num_servers do
    server_names[i] = string.format("s%d", i)
end

dsbox.Message:send("client", "core", "setup", { clients = { "c0" }, servers = server_names, middleware_after = { "lua/chandy-lamport/server.lua" } })
assert(dsbox.recv().body.type == "setup_ok")
math.randomseed(12345)

print("distributing tokens")
for c = 1, 26 do
    local task = string.char(96 + c)
    local server = server_names[math.random(#server_names)]
    dsbox.Message:send("c0", server, "task", { task = task })
end

dsbox.sleep(1)

dsbox.Message:send("c0", "s1", "snapshot", { reply_to = "c0" })

local snapshots = {}
for _ = 1, num_servers do
    local message = dsbox.recv()
    assert(message.body.type == "snapshot")
    snapshots[message.src] = message.body.snapshot
end

local tasks_in_snapshot = {}
for server, snapshot in pairs(snapshots) do
    for _, task in ipairs(snapshot.state) do
        tasks_in_snapshot[task] = (tasks_in_snapshot[task] or 0) + 1
    end
    local snapshot_fmt = { state = snapshot.state, in_transit = dsbox.array({}) }
    for _, message in ipairs(snapshot.messages) do
        snapshot_fmt.in_transit[#snapshot_fmt.in_transit + 1] = message.body.task
        tasks_in_snapshot[message.body.task] = (tasks_in_snapshot[message.body.task] or 0) + 1
    end
    print(string.format("snapshot %s = %s", server, dsbox.to_json(snapshot_fmt)))
end
print(string.format("tasks_in_snapshot = %s", dsbox.to_json(tasks_in_snapshot)))

for c = 1, 26 do
    local task = string.char(96 + c)
    if tasks_in_snapshot[task] == nil then
        print(string.format("ERROR: snapshot has lost task '%s'", task))
    elseif tasks_in_snapshot[task] > 1 then
        print(string.format("ERROR: snapshot has duplicated task '%s'", task))
    end
end