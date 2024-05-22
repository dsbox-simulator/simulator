import Event from "./Event"
import {deserialize} from "ts-jackson";
import { EventStore } from "../EventStore";

export default class CoreSocket {
    socket: WebSocket
    public onevent: (event: Event) => void;

    constructor() {
        const loc = window.location;
        let ws_protocol;
        if (loc.protocol === "https:") {
            ws_protocol = "wss";
        } else {
            ws_protocol = "ws";
        }
        let websocket_uri = `${ws_protocol}://${loc.host}/socket`;

        this.socket = new WebSocket(websocket_uri);
        this.socket.addEventListener("message", this.onReceive.bind(this));
        this.onevent = _ => {};
    }

    private onReceive(message: MessageEvent) {
        const event = deserialize(JSON.parse(message.data), Event);
        EventStore.addEvent(event);
        this.onevent(event);
    }

    public send(data: string) {
        this.socket.send(data);
    }
}
