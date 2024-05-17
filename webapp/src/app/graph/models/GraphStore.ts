import { Subject } from 'rxjs';
import { GraphEdge } from './GraphEdge';
import { GraphNode } from './GraphNode';
import { NodeLaunched, SendMessage } from '../../models/Event';
import Event from '../../models/Event';
import { NetworkNode } from './NetworkNode';
import { EventStore } from '../../models/EventStore';

export class GraphStore {
    public static  edges: GraphEdge[] = [];
    public static nodes: GraphNode[] = [];
    public static networkNodes: NetworkNode[] = [];


  static subscription = EventStore.eventsUpdated.subscribe((event: Event) => {
        GraphStore.handleNewEvent(event);
  });

  static graphSubject: Subject<string> = new Subject<string>();


  static handleNewEvent(event: Event) {

    
    const sendEvent = event.data as SendMessage;

    console.log(sendEvent);
    if (sendEvent && sendEvent.type === "send_message") {
        console.log("SendMessage event received");
        
        var source = GraphStore.networkNodes.find(node => node.id === sendEvent.message.src);
        var target = GraphStore.networkNodes.find(node => node.id === sendEvent.message.dest);
        if(!source) {
            const networkNode = new NetworkNode(sendEvent.message.src, sendEvent.message.src);
            this.addNetworkNode(networkNode);
            source = networkNode;
        }
        if(!target) {
            const networkNode = new NetworkNode(sendEvent.message.dest, sendEvent.message.dest);
            this.addNetworkNode(networkNode);
            target = networkNode;
        }

        const srcNode = new GraphNode(event.timestamp.logical.toString() + "s","send", source);
        const destNode = new GraphNode(event.timestamp.logical.toString() + "d","receive", target);

        this.addNode(srcNode);
        this.addNode(destNode);

        const edge = new GraphEdge(srcNode, destNode,event.toJson(), event.timestamp.logical);
        GraphStore.edges.push(edge);


        GraphStore.graphSubject.next("newEvent");
        
        return;
    } 


    const launchedEvent = event.data as NodeLaunched;
    if (launchedEvent instanceof NodeLaunched) {
        console.log("NodeLaunched event received");
        // TODO why is ID not equal to src in sendMessage
        //const node = new NetworkNode(launchedEvent.id.toString(), launchedEvent.commandline);
        //GraphStore.networkNodes.push(node);
        return;
    }
    
    console.log("addevent" + event);
  }

  static addNetworkNode(node: NetworkNode) {
    node.posY = GraphNode.length * 100;
    GraphStore.networkNodes.push(node);  
  }

  static addNode(node: GraphNode) {
    const networkNode = GraphStore.networkNodes.find(n => n === node.networkNode);
    if (networkNode) {
      const sameNetworkNodes = GraphStore.nodes.filter(n => n.networkNode === networkNode);
      node.posX = (sameNetworkNodes.length + 1) * 50;
    }
    GraphStore.nodes.push(node);
  }
}