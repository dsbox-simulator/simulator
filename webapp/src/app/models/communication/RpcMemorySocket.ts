import { EventStore } from "../EventStore";
import { IRpcSocket } from "./IRpcSocket";
import { JsonRpcEvent } from "./RpcEvent";

/**
 * Memory socket for Record and Play functionality
 */
export class RpcMemorySocket implements IRpcSocket {

    RpcEvents: JsonRpcEvent[] = [];

    loadEvents(events: JsonRpcEvent[]){
        this.RpcEvents = events;
    }

    call(method: string, params: any[]): Promise<any> {
        if(method.toLowerCase() === "step"){
            const event = this.RpcEvents.shift();
            if(event){
            EventStore.addEvent(event);
            }
        }

        return Promise.resolve();
    }
    
}