import { Subject } from 'rxjs';
import Event, { DeliverMessage, SendMessage, Setup } from './communication/Event';
import { DsMessage } from './DsMessage';
import { DsNodeSetup } from './DsNodeSetup';

export class EventStore {
  static events: Event[] = [];
  static messages: DsMessage[] = [];
  static nodeSetups: DsNodeSetup[] = [];

  static eventsUpdated = new Subject<Event>();

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
        
        this.messages.push(new DsMessage(event, event.timestamp.logical,
        event.timestamp.logical, sendEvent.msg.src, sendEvent.msg.dest));
    }

    const deliverEvent = event.data as DeliverMessage;
    if (deliverEvent && deliverEvent.type === "deliver_message") {
      const message = this.messages.find(message => message.send_logical_timestamp === deliverEvent.sent_timestamp);
      message?.addDeliverMessage(event);
    }


    const launchedEvent = event.data as Setup;
    if (launchedEvent && launchedEvent.type === "setup") {
        console.log("Setup event received");

        launchedEvent.nodes.forEach(node => {
            this.nodeSetups.push(new DsNodeSetup(node.name, event));
        });

        return;
    }
  }
}