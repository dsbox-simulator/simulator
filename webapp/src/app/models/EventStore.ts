import { Subject } from 'rxjs';
import { DsMessage } from './DsMessage';
import { DsNodeSetup } from './DsNodeSetup';
import { JsonRpcEvent } from './communication/RpcEvent';

export class EventStore {
  static events: JsonRpcEvent[] = [];
  static messages: DsMessage[] = [];
  static nodeSetups: DsNodeSetup[] = [];

  static eventsUpdated = new Subject<JsonRpcEvent>();
  static messagesUpdated = new Subject<DsMessage>();
  static deliveredMessage = new Subject<DsMessage>();
  static nodeSetupsUpdated = new Subject<DsNodeSetup>();

  static addEvent(event: JsonRpcEvent) {
    EventStore.events.push(event);
    this.handleEvent(event);
    EventStore.eventsUpdated.next(event);
  }

  static handleEvent(event: JsonRpcEvent) {
    const  data  = event.params.data;
    
    if (data && data.type === "send_message") {
        console.log("SendMessage event received");
        
        const body = JSON.stringify(data.msg?.body);

        const message = new DsMessage(event, event.params.timestamp.logical,
          event.params.timestamp.logical, data.msg!.src, data.msg!.dest, body);
        this.messages.push(message);
        EventStore.messagesUpdated.next(message);
    }

    if (data && data.type === "deliver_message") {
      const message = this.messages.find(message => message.send_logical_timestamp === data.sent_timestamp);
      if (message) {
        message.addDeliverMessage(event);
        EventStore.deliveredMessage.next(message);
      }
    }

    if (data && data.type === "node_launched") {
        console.log("Setup event received");

        const nodeSetup = new DsNodeSetup(data.name!, event);
        this.nodeSetups.push(nodeSetup);
        EventStore.nodeSetupsUpdated.next(nodeSetup);
        
        return;
    }
  }

  static getNonDeliveredMessages() {
    return this.messages.filter(message => !message.delivered && message.target != "core").map(message => message.sendMessage);
  }

  static loadEvents(json: string) {
    const events = JSON.parse(json) as JsonRpcEvent[];
    events.forEach(event => {
      this.addEvent(event);
    });
  }
  
  static saveEvents() {
    const json = JSON.stringify(this.events);
    const blob = new Blob([json], { type: 'application/json' });
    const url = window.URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'data.json';
    document.body.appendChild(a);
    a.click();
    window.URL.revokeObjectURL(url);
    document.body.removeChild(a);
  }
}
