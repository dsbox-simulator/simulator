import json
import sys


def eprint(*args, **kwargs):
    print(*args, file=sys.stderr, **kwargs)


def main():
    init = json.loads(sys.stdin.readline())
    assert init["body"]["type"] == "init"

    for line in sys.stdin:
        message = json.loads(line)
        reply = {
            "src": message["dest"],
            "dest": message["src"],
            "body": {
                "type": "echo_ok",
                "in_reply_to": message["body"]["msg_id"],
                "echo": message["body"]["echo"],
            }
        }
        print(json.dumps(reply), flush=True)


if __name__ == "__main__":
    main()
