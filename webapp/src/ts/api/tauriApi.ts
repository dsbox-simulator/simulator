import {Channel, invoke} from '@tauri-apps/api/core';
import {LazyStore} from "@tauri-apps/plugin-store";
import EventEmitter from "../eventEmitter";
import {
    Command,
    Commands,
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
import Api from "./api";

export default class TauriApi implements Api {
    private emitter: EventEmitter = new EventEmitter();
    private tauri_store: LazyStore = new LazyStore(".dsbox_storage.json");

    public constructor() {
        const onEvent = new Channel<Event<EventData>>;
        onEvent.onmessage = this.handleEvent.bind(this);
        invoke('subscribe_events', {onEvent})
            .then(() => {
            });
    }

    public onConnect(listener: () => void): void {
        listener()
    }

    public onDisconnect(_: () => void): void {
    }

    handleEvent(event: Event<EventData>) {
        this.emitter.emit(`event:${event.data.type}`, {detail: event});
    }

    public isConnected(): boolean {
        return true;
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

    public restart(testCommand?: Command, serverCommand?: Command) {
        invoke("restart", {testCommand, serverCommand})
            .then(() => {
            });
    }

    public break() {
        invoke("break_").then(() => {
        });
    }

    public step() {
        invoke("step").then(() => {
        });
    }

    public resume() {
        invoke("resume").then(() => {
        });
    }

    public async currentCommands(): Promise<Commands> {
        return await invoke("current_commands");
    }

    public deliver(sentTimestamp: number) {
        invoke("deliver", {sentTimestamp}).then(() => {
        });
    }

    public drop(sentTimestamp: number) {
        invoke("drop", {sentTimestamp}).then(() => {
        });
    }

    public store(key: string, value: any) {
        this.tauri_store.set(key, value)
            .then(() => {
            });
    }

    public async load(key: string): Promise<any> {
        return await this.tauri_store.get(key);
    }

    public remove(key: string) {
        this.tauri_store.delete(key)
            .then(() => {
            });
    }
}