import { EventStore } from "../EventStore";
import { IRpcSocket } from "./IRpcSocket";
import { JsonRpcEvent } from "./RpcEvent";

export class RpcMemorySocket implements IRpcSocket {

    RpcEvents: JsonRpcEvent[] = [];

    loadEvents(events: JsonRpcEvent[]){
        this.RpcEvents = events;
    }

    call(method: string, params: any[]): Promise<any> {
        if(method.toLowerCase() === "step"){
            const event = this.RpcEvents.shift();
            console.log("Event Memory: ", event);
            if(event){
            EventStore.addEvent(event);
            }
        }

        return Promise.resolve();
    }
    
}