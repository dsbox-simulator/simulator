import {RpcError, RpcErrorInfo, RpcResult} from "./rpcResponse";
import {RpcRequest} from "./rpcRequest";
import EventEmitter, {EventListener} from "../eventEmitter";

interface OutstandingPromise {
    method: string,
    resolve: (result: any) => void;
    reject: (reason: any) => void;
}


let nextRequestId: number = 1;

export default class WebSocketRpc {
    private readonly wsPath: string;
    private readonly emitter: EventEmitter = new EventEmitter();
    private readonly outstandingPromises: Map<number | string, OutstandingPromise> = new Map();
    private socket: WebSocket;

    constructor(wsPath: string) {
        this.wsPath = wsPath;
        this.socket = this.connectSocket();
    }

    public on<T>(event: string, listener: EventListener<T>): void {
        this.emitter.on(event, listener)
    }

    public off<T>(event: string, listener: EventListener<T>): void {
        this.emitter.off(event, listener)
    }

    public isConnected(): boolean {
        return this.socket.readyState === WebSocket.OPEN;
    }

    async waitConnected(): Promise<void> {
        if (this.socket.readyState !== WebSocket.OPEN) {
            await new Promise<void>(resolve => {
                const listener = () => {
                    this.off("rpc:open", listener);
                    resolve();
                };
                this.on("rpc:open", listener);
            });
        }
    }

    public async call(method: string, params?: any): Promise<any> {
        await this.waitConnected();
        return new Promise((resolve, reject) => {
            const id = nextRequestId++;
            this.outstandingPromises.set(id, {method, resolve, reject});
            this.send({jsonrpc: "2.0", id, method, params});
        });
    }

    public notify(method: string, params?: any): void {
        this.waitConnected()
            .then(() => this.send({jsonrpc: "2.0", id: null, method, params}));
    }

    private send(request: RpcRequest): void {
        if (import.meta.env.PROD) {
            console.log('send', request);
        }
        this.socket.send(JSON.stringify(request));
    }

    private handleMessage(message: any) {
        if (import.meta.env.PROD && message['method'] !== 'progress') {
            console.log('received', message);
        }
        if (isResult(message)) this.handleResult(message);
        if (isError(message)) this.handleError(message);
        if (isRequest(message)) this.handleRequest(message);
    }

    private handleResult(message: RpcResult) {
        if (message.id === null || !this.outstandingPromises.has(message.id)) {
            this.emitter.emit<{ id: string | number | null, result: any }>("unhandledResponse", {
                detail: {
                    id: message.id,
                    result: message.result
                }
            });
        } else {
            const resolver = this.outstandingPromises.get(message.id) as OutstandingPromise;
            this.outstandingPromises.delete(message.id);
            resolver.resolve(message.result);
        }
    }

    private handleError(message: RpcError) {
        if (message.id === null || !this.outstandingPromises.has(message.id)) {
            this.emitter.emit<RpcErrorInfo>("unhandledError", {detail: message.error})
        } else {
            const resolver = this.outstandingPromises.get(message.id) as OutstandingPromise;
            this.outstandingPromises.delete(message.id);
            resolver.reject(message.error);
        }
    }

    private handleRequest(message: RpcRequest) {
        this.emitter.emit<any>(`notification:${message.method}`, {detail: message.params})
    }

    private connectSocket(): WebSocket {
        const protocol = window.location.protocol === 'https:' ? 'wss' : 'ws';
        const url = new URL(this.wsPath, `${protocol}://${window.location.host}`);
        const socket = new WebSocket(url);
        socket.addEventListener("open", _e => this.emitter.emit('rpc:open'));
        socket.addEventListener('close', _e => {
            this.emitter.emit('rpc:close')
            setTimeout(() => {
                this.socket = this.connectSocket();
            }, 500);
        });
        socket.addEventListener('message', (message) => this.handleMessage(JSON.parse(message.data)));
        return socket as WebSocket;
    }
}

function isResult(object: any): object is RpcResult {
    return 'jsonrpc' in object && 'id' in object && 'result' in object;
}

function isError(object: any): object is RpcError {
    return 'jsonrpc' in object && 'id' in object && 'error' in object
}

function isRequest(object: any): object is RpcRequest {
    return 'jsonrpc' in object && 'id' in object && 'method' in object
}