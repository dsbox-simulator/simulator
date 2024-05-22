import Timestamp from './Timestamp';
import Message from './Message';
import {JsonProperty, Serializable} from 'ts-jackson';

type EventData = Setup | SendMessage | DeliverMessage | NodeDisconnected | NodeLaunched | Log;

@Serializable()
export default class Event {
    @JsonProperty()
    timestamp!: Timestamp;
    @JsonProperty()
    data!: EventData;

    toJson(): string {
        return JSON.stringify(this);
    }
}

@Serializable()
export class Setup {
    @JsonProperty()
    type!: "setup";
    @JsonProperty()
    nodes!: NodeInfo[];
}

@Serializable()
export class NodeInfo {
    @JsonProperty()
    name!: string
    @JsonProperty()
    commandline!: string
    @JsonProperty()
    id!: number
}

@Serializable()
export class SendMessage {
    @JsonProperty()
    type!: "send_message";
    @JsonProperty()
    msg!: Message;
}

@Serializable()
export class DeliverMessage {
    @JsonProperty()
    type!: "deliver_message";
    @JsonProperty()
    sent_timestamp!: number;
}

@Serializable()
export class NodeLaunched {
    @JsonProperty()
    type!: "node_launched";
    @JsonProperty()
    id!: number;
    @JsonProperty()
    commandline!: string;
}

@Serializable()
export class NodeDisconnected {
    @JsonProperty()
    type!: "node_disconnected";
    @JsonProperty()
    id!: number;
}

@Serializable()
export class Log {
    @JsonProperty()
    type!: "log";
    @JsonProperty()
    id!: number;
    @JsonProperty()
    line!: string;
}