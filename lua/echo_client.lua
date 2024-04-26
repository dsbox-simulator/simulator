Message:new("client", "core", "setup", { msg_id = 3, clients = { "c0" }, servers = { "s0" } }):send()

assert(recv().body.type == "setup_ok")

for i = 1, 1 do
    Message:new("c0", "s0", "echo", { msg_id = i, echo = string.format("echo %d", i) }):send()
    local message = recv()
    log("got reply:", message)
end
