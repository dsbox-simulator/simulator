import { JsonRpcEvent } from "./communication/RpcEvent";

/**
 * Extracts all important information from the setup Event
 */
export class DsNodeSetup{
    public id: string;
    public event: JsonRpcEvent;

    public constructor(id: string, event: JsonRpcEvent) {
        this.id = id;
        this.event = event;
    }
}