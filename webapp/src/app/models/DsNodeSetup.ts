import { JsonRpcEvent } from "./communication/RpcEvent";

export class DsNodeSetup{
    public id: string;
    public event: JsonRpcEvent;

    public constructor(id: string, event: JsonRpcEvent) {
        this.id = id;
        this.event = event;
    }
}