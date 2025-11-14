export interface Commands {
    testCommand: Command;
    serverCommand: Command;
}

export interface Command {
    program: string,
    args: string[]
}

export interface Timestamp {
    logical: number;
    physical: string;
}

export interface Event<Data> {
    timestamp: Timestamp;
    data: Data;
}

export type EventData = Reset | SendMessage | DeliverMessage | DropMessage | NodeLaunched | NodeDisconnected | Log;

export interface Reset {
    type: "reset";
}

export function isReset(data: EventData): data is Reset {
    return data.type === "reset";
}

export interface SendMessage {
    type: "send_message";
    msg: Message;
}

export function isSendMessage(data: EventData): data is SendMessage {
    return data.type === "send_message";
}

export interface Message {
    src: string;
    dest: string;
    body: Body;
}

export interface Body {
    type: string;
    id?: number;
    in_reply_to?: number;

    [data: string]: any;
}

export interface DeliverMessage {
    type: "deliver_message";
    sent_timestamp: number;
}

export function isDeliverMessage(data: EventData): data is DeliverMessage {
    return data.type === "deliver_message";
}

export interface DropMessage {
    type: "drop_message";
    sent_timestamp: number;
}

export function isDropMessage(data: EventData): data is DropMessage {
    return data.type === "drop_message";
}

export interface NodeDisconnected {
    type: "node_disconnected";
    id: number;
}

export function isNodeDisconnected(data: EventData): data is NodeDisconnected {
    return data.type === "node_disconnected";
}

export interface NodeLaunched {
    type: "node_launched";
    id: number;
    name: string;
    commandline: string;
}

export function isNodeLaunched(data: EventData): data is NodeLaunched {
    return data.type === "node_launched";
}

export interface Log {
    type: "log";
    id: number;
    message: LogMessage;
}

export function isLog(data: EventData): data is Log {
    return data.type === "log";
}

export interface LogMessage {
    text: string;
    marker: LogMarker | null
}

export interface LogMarker {
    label: string;
    color: LogMarkerColor | null;
}

export type LogMarkerColor =
    "Black" |
    "Red" |
    "Green" |
    "Yellow" |
    "Blue" |
    "Magenta" |
    "Cyan" |
    "White" |
    "BrightBlack" |
    "BrightRed" |
    "BrightGreen" |
    "BrightYellow" |
    "BrightBlue" |
    "BrightMagenta" |
    "BrightCyan" |
    "BrightWhite"

export function splitCommand(command: string): Command {
    const [program, ...args] = command.split(" ");
    return {program: program || "", args};
}

export function displayCommand(command: Command | undefined): string {
    if (command === undefined) {
        return "";
    } else if (command.args.length > 0) {
        return `${command.program} ${command.args.join(" ")}`
    } else {
        return command.program;
    }
}