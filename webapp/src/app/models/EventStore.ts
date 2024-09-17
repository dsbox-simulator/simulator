import { Subject } from 'rxjs';
import { DsMessage } from './DsMessage';
import { DsNodeSetup } from './DsNodeSetup';
import { JsonRpcEvent } from './communication/RpcEvent';
import { DsLogMessage } from './DsLogMessage';
import { LogMessage } from './communication/LogMessage';

/**
 * Stores all events that are received from the core
 */
export class EventStore {
  static events: JsonRpcEvent[] = [];
  static messages: DsMessage[] = [];
  static nodeSetups: DsNodeSetup[] = [];
  static logMessages: DsLogMessage[] = [];

  // Subjects for other components to subscribe to
  static eventsUpdated = new Subject<JsonRpcEvent>();
  static messagesUpdated = new Subject<DsMessage>();
  static deliveredMessage = new Subject<DsMessage>();
  static nodeSetupsUpdated = new Subject<DsNodeSetup>();
  static logMessagesUpdated = new Subject<DsLogMessage>();

  static addEvent(event: JsonRpcEvent) {
    EventStore.events.push(event);
    this.handleEvent(event);
    EventStore.eventsUpdated.next(event);
  }

  /**
   * Handle the event and add it to the correct list and notify the subscribers
   * @param event JsonRpcEvent
   * @returns 
   */
  static handleEvent(event: JsonRpcEvent) {
    const  data  = event.params.data;   
   
    if (data && data.type === "send_message") {

        
        const body = JSON.stringify(data.msg?.body);
        let logmessage: LogMessage | null = null;

        try {
          logmessage = JSON.parse(body ?? "") as LogMessage;
        } catch (e) {
          //ignore
        }
        
        if(logmessage && logmessage.marker != undefined) {
            
            const logMessage = new DsLogMessage(event,event.params.timestamp.logical,event.params.timestamp.logical,event.params.data.msg!.src, event.params.data.msg!.body, logmessage);
            this.logMessages.push(logMessage);
            this.logMessagesUpdated.next(logMessage);
    
            return;
        }

        const message = new DsMessage(event, event.params.timestamp.logical,
          event.params.timestamp.logical, data.msg!.src, data.msg!.dest, body);
        this.messages.push(message);
        EventStore.messagesUpdated.next(message);
        return;
    }

    if (data && data.type === "deliver_message") {
      const message = this.messages.find(message => message.send_logical_timestamp === data.sent_timestamp);
      if (message) {
        message.addDeliverMessage(event);
        EventStore.deliveredMessage.next(message);
      }
      return;
    }

    if (data && data.type === "node_launched") {

        const nodeSetup = new DsNodeSetup(data.name!, event);
        this.nodeSetups.push(nodeSetup);
        EventStore.nodeSetupsUpdated.next(nodeSetup);
        
        return;
    }

    
  }


  static dropEvent(event: JsonRpcEvent) {
    let dsevent = this.messages.findIndex(msg => msg.sendMessage === event);
    this.messages[dsevent].droped();
  }

  static getNonDeliveredMessages() {
    return this.messages.filter(message => !message.delivered && message.target != "core").map(message => message.sendMessage);
  }

  static getNonDeliveredDsMessages() {
    return this.messages.filter(message => !message.delivered && message.target != "core");
  }

  /**
   * 
   * @param json the json string to load
   */
  static loadEvents(json: string) {
    const events = JSON.parse(json) as JsonRpcEvent[];
    events.forEach(event => {
      this.addEvent(event);
    });
  }
  
  /**
   * Save the events to a file
   */
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
