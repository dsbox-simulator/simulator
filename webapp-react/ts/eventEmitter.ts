export interface EventListener<T> {
    (event: CustomEvent<T>): void;
}

export default class EventEmitter {
    private listeners: Map<string, EventListener<any>[]> = new Map();

    public on<T>(event: string, listener: EventListener<T>): void {
        if (!this.listeners.has(event)) {
            this.listeners.set(event, [])
        }
        this.listeners.get(event)!.push(listener);
    }

    public off<T>(event: string, listener: EventListener<T>): void {
        if (this.listeners.has(event)) {
            this.listeners.set(event, this.listeners.get(event)!.filter(l => l !== listener));
        }
    }

    public emit<T>(event: string, options: CustomEventInit<T> = {}): void {
        if (this.listeners.has(event)) {
            for (const listener of this.listeners.get(event)!) {
                const customEvent = new CustomEvent(event, options);
                listener(customEvent);
            }
        }
    }
}