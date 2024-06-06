import { Subject } from 'rxjs';
import Event, { DeliverMessage, SendMessage, Setup } from './communication/Event';
import { DsMessage } from './DsMessage';
import { DsNodeSetup } from './DsNodeSetup';

export class EventStore {
  static events: Event[] = [];
  static messages: DsMessage[] = [];
  static nodeSetups: DsNodeSetup[] = [];

  static eventsUpdated = new Subject<Event>();
  static messagesUpdated = new Subject<DsMessage>();
  static deliverdMessage = new Subject<DsMessage>();
  static nodeSetupsUpdated = new Subject<DsNodeSetup>();

  static addEvent(event: Event) {
    EventStore.events.push(event);
    this.handleEvent(event);
    EventStore.eventsUpdated.next(event);
  }

  static handleEvent(event: Event) {

    const sendEvent = event.data as SendMessage;
    
    console.log(sendEvent);
    if (sendEvent && sendEvent.type === "send_message") {
        console.log("SendMessage event received");
        
        const body = JSON.stringify(sendEvent.msg.body);

        const message = new DsMessage(event, event.timestamp.logical,
          event.timestamp.logical, sendEvent.msg.src, sendEvent.msg.dest, body);
        this.messages.push(message);
        EventStore.messagesUpdated.next(message);
    }

    const deliverEvent = event.data as DeliverMessage;
    if (deliverEvent && deliverEvent.type === "deliver_message") {
      const message = this.messages.find(message => message.send_logical_timestamp === deliverEvent.sent_timestamp);
      message!.addDeliverMessage(event);
      EventStore.deliverdMessage.next(message!);
    }


    const launchedEvent = event.data as Setup;
    if (launchedEvent && launchedEvent.type === "setup") {
        console.log("Setup event received");

        launchedEvent.nodes.forEach(node => {
            const nodeSetup = new DsNodeSetup(node.name, event);
            this.nodeSetups.push(nodeSetup);
            EventStore.nodeSetupsUpdated.next(nodeSetup);
        });

        return;
    }
  }

  static getNonDeliveredMessages() {
    return this.messages.filter(message => !message.delivered && message.target != "core").map(message => message.sendMessage);
  }



  static loadEvents(json : string) {
    const events = JSON.parse(json) as Event[];
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