import { LogMessage } from "./communication/LogMessage";
import { JsonRpcEvent } from "./communication/RpcEvent";

export class DsLogMessage{
    public message: JsonRpcEvent;
    public id: number;
    public send_logical_timestamp: number;
    public source: string;
    public body: string;
    public logmessage: LogMessage;

    public constructor(message: JsonRpcEvent, id: number, send_logical_timestamp: number, source: string, body: string, logmessage: LogMessage) {
        this.message = message;
        this.id = id;
        this.send_logical_timestamp = send_logical_timestamp;
        this.source = source;
        this.body = body;
        this.logmessage = logmessage;
    }
}

