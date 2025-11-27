import WebSocketApi from "../api/websocketApi";
import {useSyncExternalStore} from "react";
import EventEmitter from "../eventEmitter";
import {Command, Commands, LogMessage, Message, Timestamp} from "../api/types";
import debounce from "../debounce";
import TauriApi from "../api/tauriApi";
import Api from "../api/api";

export interface NodeInfo {
    name: string;
    commandline: string;
    running: boolean;
}

export interface LogInfo {
    node: string,
    timestamp: Timestamp,
    message: LogMessage,
}

export interface MessageInfo {
    sentAt: Timestamp,
    deliveredAt: Timestamp | null,
    message: Message,
    dropped: boolean,
}


export function isLog(data: MessageInfo | LogInfo | null): data is LogInfo {
    return data?.message.hasOwnProperty("text") || false;
}

export function isMessage(message: MessageInfo | LogInfo | null): message is MessageInfo {
    return message?.message.hasOwnProperty("src") || false;
}

export default class Store {
    private readonly api: Api;
    private nodes: Map<string, NodeInfo> = new Map<string, NodeInfo>();
    private logs: LogInfo[] = [];
    private messages: MessageInfo[] = [];
    private emitter: EventEmitter = new EventEmitter();

    constructor(wsPath: string, inTauri: boolean) {
        if (inTauri) {
            this.api = new TauriApi();
        } else {
            this.api = new WebSocketApi(wsPath);
        }

        const emitDebounced = debounce(this.emitter.emit.bind(this.emitter), 100);
        this.api.onConnect(() => {
            emitDebounced("connection_changed");
        });
        this.api.onDisconnect(() => {
            emitDebounced("connection_changed");
        });
        this.api.onReset(() => {
            this.messages = [];
            this.nodes = new Map<string, NodeInfo>();
            this.logs = [];
            emitDebounced("nodes_changed");
            emitDebounced("log_changed");
            emitDebounced("messages_changed");
        });
        this.api.onSendMessage(event => {
            this.messages = [...this.messages, {
                sentAt: event.timestamp,
                deliveredAt: null,
                message: event.data.msg,
                dropped: false
            }];
            console.log("got message", event.data.msg);
            emitDebounced("messages_changed");
        });
        this.api.onDeliverMessage(event => {
            let sentIdx = null;
            for (let i = this.messages.length - 1; i >= 0; i--) {
                if (this.messages[i]!.sentAt.logical === event.data.sent_timestamp) {
                    sentIdx = i;
                    break;
                }
            }
            if (sentIdx !== null) {
                this.messages = this.messages.map((m, idx) => idx == sentIdx ? {
                    ...m,
                    deliveredAt: event.timestamp
                } : m)
                emitDebounced("messages_changed");
            }
        });
        this.api.onDropMessage(event => {
            let sentIdx = null;
            for (let i = this.messages.length - 1; i >= 0; i--) {
                if (this.messages[i]!.sentAt.logical === event.data.sent_timestamp) {
                    sentIdx = i;
                    break;
                }
            }
            if (sentIdx !== null) {
                this.messages = this.messages.map((m, idx) => idx == sentIdx ? {
                    ...m,
                    dropped: true,
                } : m)
                emitDebounced("messages_changed");
            }
        });
        this.api.onNodeLaunched(event => {
            console.log("node launched", event.data);
            this.nodes = new Map(this.nodes.entries())
            this.nodes.set(event.data.name, {...event.data, running: true});
            emitDebounced("nodes_changed");
        });
        this.api.onNodeDisconnected(event => {
            this.nodes = new Map(this.nodes.entries())
            this.nodes.get(event.data.name)!.running = false;
            emitDebounced("nodes_changed");
        });
        this.api.onLog(event => {
            this.logs = [...this.logs, {
                timestamp: event.timestamp,
                message: event.data.message,
                node: event.data.node
            }];
            emitDebounced("log_changed");
        })
    }


    private useStore<T>(event: string, snapshot: () => T): T {
        return useSyncExternalStore(onStoreChange => {
            this.emitter.on(event, onStoreChange);
            return () => this.emitter.off(event, onStoreChange);
        }, snapshot);
    }

    public useNodes(): Map<string, NodeInfo> {
        return this.useStore("nodes_changed", () => this.nodes);
    }

    public useConnected(): boolean {
        return this.useStore("connection_changed", this.api.isConnected.bind(this.api));
    }

    public useLogs(): LogInfo[] {
        return this.useStore("log_changed", () => this.logs);
    }

    public useMessages(): MessageInfo[] {
        return this.useStore("messages_changed", () => this.messages);
    }

    public async currentCommands(): Promise<Commands> {
        return await this.api.currentCommands();
    }

    public restart(testCommand?: Command, serverCommand?: Command) {
        this.api.restart(testCommand, serverCommand);
    }

    public step() {
        this.api.step();
    }

    public resume() {
        this.api.resume();
    }

    public break() {
        this.api.break();
    }

    public deliver(message: MessageInfo) {
        this.api.deliver(message.sentAt.logical);
    }

    public drop(message: MessageInfo) {
        this.api.drop(message.sentAt.logical);
    }

    public store(key: string, value: any) {
        this.api.store(key, value);
    }

    public async load(key: string): Promise<any> {
        return await this.api.load(key);
    }

    public remove(key: string) {
        this.api.remove(key);
    }
}
