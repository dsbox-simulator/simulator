from .dsbox import Message

init = Message.recv()
assert init.body.type == "init"
for message in Message.recv_iter():
    reply = message.reply('echo_ok', echo=message.body.echo)
    reply.send()
