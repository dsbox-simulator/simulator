import Event from "./communication/Event";

export class DsMessage {
    
    public sendMessage: Event;
    public deliverMessage: Event | null;
    public id: number;
    public send_logical_timestamp: number;
    public deliver_logical_timestamp: number | null;
    public source: string;
    public target: string;
    public delivered: boolean;
    public update: boolean;

    public constructor(sendMessage: Event, id: number, send_logical_timestamp: number, source: string, target: string) {
        this.sendMessage = sendMessage;
        this.deliverMessage = null;
        this.id = id;
        this.send_logical_timestamp = send_logical_timestamp;
        this.deliver_logical_timestamp = null;
        this.source = source;
        this.target = target;
        this.delivered = false;
        this.update = true;
    }

    public addDeliverMessage(deliverMessage: Event) {
        this.deliverMessage = deliverMessage;
        this.deliver_logical_timestamp = deliverMessage.timestamp.logical;
        this.delivered = true;
        this.update = true;
    }
}