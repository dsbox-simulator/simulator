import { TypeColorStore } from "./TypeColorStore";
import { JsonRpcEvent, MessageBody } from "./communication/RpcEvent";

export class DsMessage {
    
    public sendMessage: JsonRpcEvent;
    public deliverMessage: JsonRpcEvent | null;
    public id: number;
    public send_logical_timestamp: number;
    public deliver_logical_timestamp: number | null;
    public source: string;
    public target: string;
    public delivered: boolean;
    public update: boolean;
    public body: string;
    public color: string | undefined;
    public type: string | undefined;
    public typeColor: string | undefined;

    private static readonly IgnoreTypes = ["launch", "launch_finished", "init", "all_servers"];

    public constructor(sendMessage: JsonRpcEvent, id: number, send_logical_timestamp: number, source: string, target: string, body: string) {
        this.sendMessage = sendMessage;
        this.deliverMessage = null;
        this.id = id;
        this.send_logical_timestamp = send_logical_timestamp;
        this.deliver_logical_timestamp = null;
        this.source = source;
        this.target = target;
        this.delivered = false;
        this.update = true;
        this.body = body;
        this.determineType();
    }

    public addDeliverMessage(deliverMessage: JsonRpcEvent) {
        this.deliverMessage = deliverMessage;
        this.deliver_logical_timestamp = deliverMessage.params.timestamp.logical;
        this.delivered = true;
        this.update = true;
    }

    public addLogMessage(color: string) {
        this.delivered = true;
        this.update = true;
        this.color = color;
    }

    public determineType(){

        try {
            console.log("Body: " + this.sendMessage.params.data.msg?.body); 
            const body = this.sendMessage.params.data.msg?.body ?? "";
              
            const bodytype  = body as unknown as MessageBody;
          
            if(bodytype == null) {
            return;
            }

            if(DsMessage.IgnoreTypes.includes(bodytype.type)) {
            return;
            }

            this.type = bodytype.type;
            this.typeColor = TypeColorStore.getColor(this.type);
            console.log("Type: " + this.type + " Color: " + this.typeColor);
        } catch (e) {
            //ignore
            console.log("Failed to parse JSON body: " + e);
          }
    }

}