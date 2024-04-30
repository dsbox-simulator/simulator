local dsbox = require("dsbox")
require("luarocks.loader")
local chronos = require("chronos")
dsbox.Message:send("client", "core", "setup", { clients = { "c" }, servers = dsbox.array({}) })
assert(dsbox.recv().body.type == "setup_ok")

local num_trips = 10000
while true do
    local min = math.maxinteger
    local max = 0
    local total = 0
    for _ = 1, num_trips do
        local before = chronos.nanotime()
        dsbox.Message:send("c", "c", "empty")
        assert(dsbox.recv().body.type == "empty")
        local after = chronos.nanotime()
        local rtt = after - before;
        min = math.min(min, rtt)
        max = math.max(max, rtt)
        total = total + rtt
    end
    local avg = total / num_trips
    print(string.format("round trips ok: avg %.3fms, min %.3fms, max %.3fms",
            avg * 1000,
            min * 1000,
            max * 1000));
end