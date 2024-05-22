import { Subject } from 'rxjs';
import { GraphEdge } from './GraphEdge';
import { GraphNode } from './GraphNode';
import { DeliverMessage, NodeLaunched, SendMessage, Setup } from '../../models/communication/Event';
import Event from '../../models/communication/Event';
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

    this.edges = [];
    this.networkNodes = [];

    EventStore.nodeSetups.forEach(nodeSetup => {   
      if(this.networkNodes.find(node => node.id === nodeSetup.id) === null) {
        const networkNode = new NetworkNode(nodeSetup.id, nodeSetup.id);
        console.log("addNetworkNode " + networkNode.id);
        this.addNetworkNode(networkNode);
      }
    });

    EventStore.messages.forEach(message => {
      
      const source = GraphStore.networkNodes.find(node => node.id === message.source);
      if(!source) {return;}
      const srcNode = new GraphNode(message.send_logical_timestamp.toString(),message.send_logical_timestamp.toString(), source);
      this.addNode(srcNode);

      if(message.delivered){
        const target = GraphStore.networkNodes.find(node => node.id === message.target);
        if(!target) {return;}
        const destNode = new GraphNode(message.deliver_logical_timestamp!.toString(),message.deliver_logical_timestamp!.toString(), target);
        this.addNode(destNode, srcNode.posX);

        const edge = new GraphEdge(srcNode, destNode,message.send_logical_timestamp.toString() + "edge", message.send_logical_timestamp!);
        GraphStore.edges.push(edge);
      }

    });

    this.graphSubject.next("update");
    console.log("addevent" + event);
  }

  static addNetworkNode(node: NetworkNode) {
    node.posY = (GraphStore.networkNodes.length + 1) * 35;
    GraphStore.networkNodes.push(node);  
  }

  static addNode(node: GraphNode, posX: number = 0) {
    const networkNode = GraphStore.networkNodes.find(n => n === node.networkNode);
    if (networkNode) {
      const sameNetworkNodes = GraphStore.nodes.filter(n => n.networkNode === networkNode);
      const biggestPosX = Math.max(...sameNetworkNodes.map(node => node.posX));
      node.posX = biggestPosX + 25;
      if(node.posX < posX) {
        node.posX = posX + 25;
      }
      if( node.posX > networkNode.length) {
        networkNode.length = node.posX + 50;
      }
    }
    GraphStore.nodes.push(node);
  }
}