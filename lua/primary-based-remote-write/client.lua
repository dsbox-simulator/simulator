local argparse = require('argparse')
local dsbox = require('dsbox')

local parser = argparse()
parser:option("-n", "Number of servers", "4")
parser:option("-s", "rng seed")

local args = parser:parse(dsbox.args)
local seed = args.s
if seed ~= nil then
    math.randomseed(seed)
end

local num_servers = args.n
local server_names = {}
for i = 1, num_servers do
    table.insert(server_names, string.format("s%d", i))
end
dsbox.Message:send("c0", "core", "setup", { clients = { "c0" }, servers = server_names })
assert(dsbox.recv().body.type == "setup_ok")

local store_sequence = {}
for _ = 1, 10 do
    store_sequence[#store_sequence + 1] = string.char(96 + math.random(26))
end

for i, value in ipairs(store_sequence) do
    dsbox.Message:send("c0", server_names[math.random(num_servers)], "store", { msg_id = i, value = value })
end

for _ = 1, #store_sequence do
    local ack = dsbox.recv(1)
    if ack == nil then
        dsbox.log("ack timed out")
        return
    else
        assert(ack.body.type == "ack")
    end
end

local function check_sequence(expected_sequence, sequence)
    if #sequence ~= #expected_sequence then
        return false
    end
    for i, expected in ipairs(expected_sequence) do
        if sequence[i] ~= expected then
            return false
        end
    end
    return true
end

local expected_sequence
for _, server in ipairs(server_names) do
    dsbox.Message:send("c0", server, "load")
    local response = dsbox.recv(1.0)
    if response == nil then
        dsbox.log("ERROR: (server", server, ") timeout")
    else
        local sequence = response.body.sequence
        if expected_sequence == nil then
            expected_sequence = sequence
        else
            if not check_sequence(expected_sequence, sequence) then
                dsbox.log("ERROR: (server", server, ") expected sequence", expected_sequence, "but got", sequence)
            end
        end
    end
end

