interface Timestamp {
    logical: number;
    physical: string;
}

interface MessageBody {
    as_client?: boolean;
    lamport?: number;
    name?: string;
    type: string;
}

interface Message {
    body: string;
    dest: string;
    src: string;
}

interface EventData {
    msg?: Message;
    commandline?: string;
    id?: number;
    name?: string;
    type: string;
    sent_timestamp?: number;
}

interface EventParams {
    data: EventData;
    timestamp: Timestamp;
}

export interface JsonRpcEvent {
    jsonrpc: string;
    method: string;
    params: EventParams;
    id: null;
}
