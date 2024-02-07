import Timestamp from './Timestamp';
import Message from './Message';
import {JsonProperty, Serializable} from "ts-jackson";

type EventData = Setup | SendMessage | DeliverMessage | NodeDisconnected | Log;

@Serializable()
export default class Event {
    @JsonProperty()
    timestamp!: Timestamp;
    @JsonProperty()
    data!: EventData;
}

@Serializable()
export class Setup {
    @JsonProperty()
    type!: "setup";
    @JsonProperty()
    nodes!: Map<string, number>;
}

@Serializable()
export class SendMessage {
    @JsonProperty()
    type!: "send_message";
    @JsonProperty()
    message!: Message;
}

@Serializable()
export class DeliverMessage {
    @JsonProperty()
    type!: "deliver_message";
    @JsonProperty()
    sent_timestamp!: number;
}

@Serializable()
export class NodeDisconnected {
    @JsonProperty()
    type!: "node_disconnected";
    @JsonProperty()
    node_id!: number;
}

@Serializable()
export class Log {
    @JsonProperty()
    type!: "log";
    @JsonProperty()
    node_id!: number;
    @JsonProperty()
    source_file!: string;
    @JsonProperty()
    line!: string;
}