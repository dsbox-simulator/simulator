import Event from "./communication/Event";

export class DsNodeSetup{
    public id: string;
    public event: Event;

    public constructor(id: string, event: Event) {
        this.id = id;
        this.event = event;
    }
}