local dsbox = require("dsbox")
assert(dsbox.recv().body.type == "init")

for message in dsbox.recv_iter() do
    message:reply("echo_ok", { echo = message.body.echo })
end