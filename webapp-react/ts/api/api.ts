import EventEmitter from "../eventEmitter";
import WebSocketRpc from "../rpc/rpc";
import {
    DeliverMessage,
    DropMessage,
    Event,
    EventData,
    Log,
    NodeDisconnected,
    NodeLaunched,
    Reset,
    SendMessage
} from "./types";

export default class Api {
    private rpc: WebSocketRpc;
    private emitter: EventEmitter = new EventEmitter();

    public constructor(wsPath: string) {
        this.rpc = new WebSocketRpc(wsPath);
        this.rpc.on("notification:event", this.handleEvent.bind(this))
    }

    public onConnect(listener: () => void): void {
        this.rpc.on("rpc:open", listener)
    }

    public onDisconnect(listener: () => void): void {
        this.rpc.on("rpc:close", listener)
    }

    public isConnected(): boolean {
        return this.rpc.isConnected();
    }

    handleEvent(request: CustomEvent) {
        const event = request.detail as unknown as Event<EventData>;
        this.emitter.emit(`event:${event.data.type}`, {detail: event});
    }

    public onReset(listener: (event: Event<Reset>) => void): void {
        this.emitter.on("event:reset", e => listener(e.detail as Event<Reset>));
    }

    public onSendMessage(listener: (event: Event<SendMessage>) => void): void {
        this.emitter.on("event:send_message", e => listener(e.detail as Event<SendMessage>));
    }

    public onDeliverMessage(listener: (event: Event<DeliverMessage>) => void): void {
        this.emitter.on("event:deliver_message", e => listener(e.detail as Event<DeliverMessage>));
    }

    public onDropMessage(listener: (event: Event<DropMessage>) => void): void {
        this.emitter.on("event:drop_message", e => listener(e.detail as Event<DropMessage>));
    }

    public onNodeLaunched(listener: (event: Event<NodeLaunched>) => void) {
        this.emitter.on("event:node_launched", e => listener(e.detail as Event<NodeLaunched>));
    }

    public onNodeDisconnected(listener: (event: Event<NodeDisconnected>) => void) {
        this.emitter.on("event:node_disconnected", e => listener(e.detail as Event<NodeDisconnected>));
    }

    public onLog(listener: (event: Event<Log>) => void) {
        this.emitter.on("event:log", e => listener(e.detail as Event<Log>));
    }

    public step() {
        this.rpc.notify("step", {});
    }

    public resume() {
        this.rpc.notify("resume", {});
    }

    public break() {
        this.rpc.notify("break", {});
    }

    public deliver(sentTimestamp: number) {
        this.rpc.notify("deliver", {sent_timestamp: sentTimestamp});
    }

    public drop(sentTimestamp: number) {
        this.rpc.notify("drop", {sent_timestamp: sentTimestamp});
    }

    public store(key: string, value: any) {
        this.rpc.notify("store", {key, value});
    }

    public async load(key: string): Promise<any> {
        return await this.rpc.call("load", {key});
    }

    public remove(key: string) {
        this.rpc.notify("drop", {key});
    }
}