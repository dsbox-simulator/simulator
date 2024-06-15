import { JsonRpcWebSocketClient } from './RpcSocket';

export class CoreSocketFactory {

    static rpcInstance: JsonRpcWebSocketClient;
    public static create(): JsonRpcWebSocketClient {
        
        // Usage example:
        if(this.rpcInstance !== undefined) {
            return this.rpcInstance;
        }

        const client = new JsonRpcWebSocketClient('ws://127.0.0.1:8080/socket');        
        this.rpcInstance = client;
        return this.rpcInstance;
    }

}