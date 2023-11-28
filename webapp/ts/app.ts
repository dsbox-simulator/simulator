

export default function run() {
    let app = new App();
    app.run();
}

function error(message: string): never {
    throw new Error(message);
}

class App {
    private socket: WebSocket;
    private events_table: HTMLElement;

    public constructor() {
        this.events_table = document.getElementById('events') ?? error("could not find element with id #logs");

        let loc = window.location;
        let ws_protocol;
        if (loc.protocol === "https:") {
            ws_protocol = "wss";
        } else {
            ws_protocol = "ws";
        }
        let websocket_uri = `${ws_protocol}://${loc.host}/socket`;

        this.socket = new WebSocket(websocket_uri);

    }

    public run() {
        this.socket.addEventListener("message", this.socketMessage.bind(this))

        let resume_button = document.getElementById("resume");
        if (resume_button !== null) {
            resume_button.addEventListener("click", this.resume.bind(this));
        }

        let step_button = document.getElementById("step");
        if (step_button !== null) {
            step_button.addEventListener("click", this.step.bind(this));
        }
    }

    private addEvent(event: any) {
        let row = document.createElement("tr");

        let cell = document.createElement("td");
        cell.innerText = event.timestamp.logical;
        row.appendChild(cell);

        cell = document.createElement("td");
        cell.innerText = event.timestamp.physical;
        row.appendChild(cell);

        cell = document.createElement("td");
        cell.innerText = event.data.type;
        row.appendChild(cell);

        delete event.data.type;
        cell = document.createElement("td");
        cell.innerText = JSON.stringify(event.data);
        row.appendChild(cell);

        this.events_table.appendChild(row);
    }

    private socketMessage(message: MessageEvent) {
        this.addEvent(JSON.parse(message.data));
    }

    public resume() {
        this.socket.send("resume");
    }

    public step() {
        this.socket.send("step");
    }
}