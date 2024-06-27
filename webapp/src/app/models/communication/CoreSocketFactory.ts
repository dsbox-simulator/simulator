import { IRpcSocket } from './IRpcSocket';
import { JsonRpcEvent } from './RpcEvent';
import { RpcMemorySocket } from './RpcMemorySocket';
import { JsonRpcWebSocketClient } from './RpcSocket';

export class CoreSocketFactory {

    static rpcInstance: IRpcSocket;
    public static create(): IRpcSocket {
        
        // Usage example:
        if(this.rpcInstance !== undefined) {
            return this.rpcInstance;
        }

        const client = new JsonRpcWebSocketClient('ws://127.0.0.1:8080/socket');        
        this.rpcInstance = client;
        return this.rpcInstance;
    }

    public static load(events: JsonRpcEvent[]){
        const socket = new RpcMemorySocket();
        socket.loadEvents(events);

        this.rpcInstance = socket;
    }

}