import Event, {Log} from "./Event"

export default class State {
    all_events: Event[] = [];
    nodes: Map<string, number> = new Map();
    logs: Map<number, Log[]> = new Map();

    public constructor(old_state: State | null = null) {
        if (old_state !== null) {
            Object.assign(this, old_state);
        }
    }

    public update(event: Event): State {
        this.all_events.push(event);
        switch (event.data.type) {
            case "setup":
                this.nodes = new Map(Object.entries(event.data.nodes));
                for (const [_, node_id] of this.nodes) {
                    if (this.logs.get(node_id) === undefined) {
                        this.logs.set(node_id, []);
                    }
                }
                break;
            case "send_message":
                break;
            case "deliver_message":
                break;
            case "node_disconnected":
                break;
            case "log":
                if(this.logs.get(event.data.node_id) === undefined) {
                    this.logs.set(event.data.node_id, []);
                }
                this.logs.get(event.data.node_id)!.push(event.data);
                break;

        }
        return new State(this);
    }
}