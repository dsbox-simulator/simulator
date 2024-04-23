import json
import sys
from typing import Any, Dict, Generator


class MessageBody:
    def __init__(self, type: str, msg_id: int | None = None, in_reply_to: int | None = None, **kwargs):
        self.type = type
        self.msg_id = msg_id
        self.in_reply_to = in_reply_to
        for k, v in kwargs.items():
            setattr(self, k, v)


class Message:

    def __init__(self, src: str, dest: str, body: MessageBody):
        self.src: str = src
        self.dest: str = dest
        self.body: MessageBody = body

    def reply(self, body: MessageBody):
        if not body.in_reply_to:
            body.in_reply_to = self.msg_id
        return Message(self.dest, self.src, body)

    @property
    def type(self) -> str:
        return self.body.type

    @property
    def msg_id(self) -> int | None:
        return self.body.msg_id

    @property
    def in_reply_to(self) -> int | None:
        return self.body.in_reply_to

    def __str__(self):
        return self.to_dict().__str__()

    def to_dict(self) -> Dict[str, Any]:
        return {"src": self.src, "dest": self.dest, "body": self.body.__dict__}

    def send(self):
        print(json.dumps(self.to_dict()), flush=True)

    @staticmethod
    def from_dict(data: Dict[str, Any]) -> 'Message':
        data['body'] = MessageBody(**data['body'])
        return Message(**data)

    @staticmethod
    def recv() -> 'Message':
        return Message.from_dict(json.loads(sys.stdin.readline()))

    @staticmethod
    def recv_iter() -> Generator['Message', None, None]:
        for line in sys.stdin:
            yield Message.from_dict(json.loads(line))


def log(message: Any, **kwargs):
    print(message.__str__(), file=sys.stderr, **kwargs)
