import { EventStore } from "../EventStore";
import { JsonRpcEvent } from "../communication/RpcEvent";
import { IRpcSocket } from "./IRpcSocket";

/**
 * WebSocket client for JSON-RPC communication
 */
export class JsonRpcWebSocketClient implements IRpcSocket {
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
      } else {
          const rpcEvent = this.handleIncomingMessage(event.data);
          EventStore.addEvent(rpcEvent);
      }
  }

  private handleIncomingMessage(jsonRpcMessage: string): JsonRpcEvent {
      return JSON.parse(jsonRpcMessage) as JsonRpcEvent;
  }

  /**
   * 
   * @returns Promise that resolves when the socket is open
   */
  private waitForSocketOpen(): Promise<void> {
      return new Promise((resolve, reject) => {
          if (this.socket.readyState === WebSocket.OPEN) {
              resolve();
          } else {
              this.socket.addEventListener('open', () => resolve());
              this.socket.addEventListener('error', (error) => reject(new Error(`WebSocket error: ${error}`)));
          }
      });
  }

  /**
   * 
   * @param method the method to call
   * @param params the parameters to pass to the method
   * @returns  a promise that resolves with the result of the method call
   */
  call(method: string, params: any[] = []): Promise<any> {
      return this.waitForSocketOpen().then(() => {
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
              this.socket.send(payloadString);
          });
      }).catch((error) => {
          console.error(`Failed to send message: ${error}`);
          throw error; // Re-throw the error if needed
      });
  }
}
