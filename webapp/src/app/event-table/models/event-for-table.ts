import {JsonProperty, Serializable} from 'ts-jackson';
import Event, {Setup, SendMessage, DeliverMessage, NodeDisconnected, NodeLaunched, Log, NodeInfo} from '../../models/communication/Event';
import Message from '../../models/communication/Message';

@Serializable()
export class EventForTable extends Event {

    public static fromEvent(event: Event): EventForTable {
        const eventForTable = new EventForTable();
        Object.assign(eventForTable, event);
    
        if (event.data instanceof Setup) {
            eventForTable.nodes = event.data.nodes;
        } else if (event.data instanceof SendMessage) {
            eventForTable.msg = event.data.msg;
        } else if (event.data instanceof DeliverMessage) {
            eventForTable.sent_timestamp = event.data.sent_timestamp;
        } else if (event.data instanceof NodeLaunched) {
            eventForTable.id = event.data.id;
            eventForTable.commandline = event.data.commandline;
        } else if (event.data instanceof NodeDisconnected) {
            eventForTable.id = event.data.id;
        } else if (event.data instanceof Log) {
            eventForTable.id = event.data.id;
            eventForTable.line = event.data.line;
        }
    
        return eventForTable;
    }
    @JsonProperty({required: false})
    nodes?: NodeInfo[];

    @JsonProperty({required: false})
    msg?: Message;

    @JsonProperty({required: false})
    sent_timestamp?: number;

    @JsonProperty({required: false})
    id?: number;

    @JsonProperty({required: false})
    commandline?: string;

    @JsonProperty({required: false})
    line?: string;
}