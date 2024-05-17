import Event, {Log, NodeInfo} from "./Event"

export default class State {
    all_events: Event[] = [];
    nodes: NodeInfo[] = [];
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
                this.nodes = event.data.nodes;
                for (const node of this.nodes) {
                    if (this.logs.get(node.id) === undefined) {
                        this.logs.set(node.id, []);
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
                if (this.logs.get(event.data.id) === undefined) {
                    this.logs.set(event.data.id, []);
                }
                this.logs.get(event.data.id)!.push(event.data);
                break;

        }
        return new State(this);
    }
}