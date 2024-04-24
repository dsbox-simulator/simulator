import json
import sys
from typing import Any, Dict, Generator, Optional


class MessageBody:
    def __init__(self, type: str, msg_id:  Optional[int] = None, in_reply_to:  Optional[int] = None, **kwargs):
        self.type = type
        self.msg_id = msg_id
        self.in_reply_to = in_reply_to
        for k, v in kwargs.items():
            setattr(self, k, v)


class Message:

    def __init__(self, src: str, dest: str, type: str, msg_id:  Optional[int] = None, in_reply_to:  Optional[int] = None,
                 **kwargs):
        self.src: str = src
        self.dest: str = dest
        self.body: MessageBody = MessageBody(type, msg_id, in_reply_to, **kwargs)

    def reply(self, type: str, **kwargs):
        if 'in_repl_to' not in kwargs:
            kwargs['in_reply_to'] = self.msg_id
        return Message(self.dest, self.src, type, **kwargs)

    @property
    def type(self) -> str:
        return self.body.type

    @property
    def msg_id(self) -> Optional[int]:
        return self.body.msg_id

    @property
    def in_reply_to(self) -> Optional[int]:
        return self.body.in_reply_to

    def __str__(self):
        return json.dumps(self.to_dict())

    def to_dict(self) -> Dict[str, Any]:
        return {"src": self.src, "dest": self.dest, "body": self.body.__dict__}

    def send(self):
        print(self.__str__(), flush=True)

    @staticmethod
    def from_dict(data: Dict[str, Any]) -> 'Message':
        body = data.pop('body')
        return Message(**data, **body)

    @staticmethod
    def recv() -> Optional['Message']:
        line = sys.stdin.readline()
        if len(line) == 0:
            return None
        else:
            return Message.from_dict(json.loads(line))

    @staticmethod
    def recv_iter() -> Generator['Message', None, None]:
        for line in sys.stdin:
            if len(line) == 0:
                return
            yield Message.from_dict(json.loads(line))


def log(message: Any, **kwargs):
    print(message.__str__(), file=sys.stderr, flush=True, **kwargs)
