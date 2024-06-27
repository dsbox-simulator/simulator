import { EventStore } from "../EventStore";
import { JsonRpcEvent } from "../communication/RpcEvent";
import { IRpcSocket } from "./IRpcSocket";

export class JsonRpcWebSocketClient implements IRpcSocket{
    private socket: WebSocket;
    private id: number;
    private pendingRequests: Map<number, (result: any) => void>;
  
    constructor(url: string) {
      this.socket = new WebSocket(url);
      this.id = 1;
      this.pendingRequests = new Map();
  
      this.socket.onmessage = this.handleMessage.bind(this);
      this.socket.onerror = (error) => console.error(`WebSocket error: ${error}`);
    }
  
    private handleMessage(event: MessageEvent) {
      const response = JSON.parse(event.data);
      const callback = this.pendingRequests.get(response.id);
      if (callback) {
        this.pendingRequests.delete(response.id);
        if (response.error) {
          callback(Promise.reject(new Error(response.error.message)));
        } else {
          callback(response.result);
        }
      }
      else
        {
          console.log("Received message: ", event.data);
            const rpcEvent = this.handleIncomingMessage(event.data);
            EventStore.addEvent(rpcEvent);
        }
    }

    private handleIncomingMessage(jsonRpcMessage: string): JsonRpcEvent {
      return JSON.parse(jsonRpcMessage) as JsonRpcEvent;
      
      }
  
    call(method: string, params: any[] = []): Promise<any> {
      return new Promise((resolve, reject) => {
        const id = this.id++;
        const payload = {
          jsonrpc: '2.0',
          method: method,
          params: params,
          id: id
        };
  
        this.pendingRequests.set(id, (result: any) => {
          resolve(result);
        });

        const payloadString = JSON.stringify(payload);
        console.log("Sending payload: ", payloadString);
        this.socket.send(payloadString);
      });
    }
  }
  
  