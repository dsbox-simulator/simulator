import {
    Command,
    Commands,
    DeliverMessage,
    DropMessage,
    Event,
    Log,
    NodeDisconnected,
    NodeLaunched,
    Reset,
    SendMessage
} from "./types";

export default interface Api {
    onConnect(listener: () => void): void;

    onDisconnect(listener: () => void): void;

    isConnected(): boolean;

    onReset(listener: (event: Event<Reset>) => void): void;

    onSendMessage(listener: (event: Event<SendMessage>) => void): void;

    onDeliverMessage(listener: (event: Event<DeliverMessage>) => void): void;

    onDropMessage(listener: (event: Event<DropMessage>) => void): void;

    onNodeLaunched(listener: (event: Event<NodeLaunched>) => void): void;

    onNodeDisconnected(listener: (event: Event<NodeDisconnected>) => void): void;

    onLog(listener: (event: Event<Log>) => void): void;

    restart(testCommand?: Command, serverCommand?: Command): void;

    break(): void;

    step(): void

    resume(): void;

    currentCommands(): Promise<Commands>;

    deliver(sentTimestamp: number): void

    drop(sentTimestamp: number): void;

    store(key: string, value: any): void;

    load(key: string): Promise<any>;

    remove(key: string): void
}