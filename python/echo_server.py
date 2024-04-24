from pynode import Message, MessageBody, log

init = Message.recv()
assert init.body.type == "init"
log("ready to go")
for message in Message.recv_iter():
    log(f"received: {message}")
    reply = message.reply('echo_ok', echo=message.body.echo)
    log(f"sending: {reply}")
    reply.send()
