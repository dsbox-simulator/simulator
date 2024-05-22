import { Subject } from 'rxjs';
import { GraphEdge } from './GraphEdge';
import { GraphNode } from './GraphNode';
import { DeliverMessage, NodeLaunched, SendMessage, Setup } from '../../models/communication/Event';
import Event from '../../models/communication/Event';
import { NetworkNode } from './NetworkNode';
import { EventStore } from '../../models/EventStore';
import { DsNodeSetup } from '../../models/DsNodeSetup';
import { DsMessage } from '../../models/DsMessage';

export class GraphStore {
    public static  edges: GraphEdge[] = [];
    public static nodes: GraphNode[] = [];
    public static networkNodes: NetworkNode[] = [];

  static graphSubject: Subject<string> = new Subject<string>();


  static subscription = EventStore.eventsUpdated.subscribe((event: Event) => {
        //GraphStore.handleNewEvent(event);
  });

  static subscription2 = EventStore.nodeSetupsUpdated.subscribe((nodeSetup: DsNodeSetup) => {    
      const networkNode = new NetworkNode(nodeSetup.id, nodeSetup.id);
      console.log("addNetworkNode " + networkNode.id);
      this.addNetworkNode(networkNode);
      this.graphSubject.next("update");
  });

  static subscription3 = EventStore.deliverdMessage.subscribe((message: DsMessage) => {

    var srcNode = GraphStore.nodes.find(node => node.id === message.send_logical_timestamp.toString())
    const target = GraphStore.networkNodes.find(node => node.id === message.target);
    if(!target) {return;}
    const destNode = new GraphNode(message.deliver_logical_timestamp!.toString(),message.deliver_logical_timestamp!.toString(), target);
    this.addNode(destNode, srcNode!.posX);

    const edge = new GraphEdge(srcNode!, destNode,message.send_logical_timestamp.toString() + "edge", message.send_logical_timestamp!);
    GraphStore.edges.push(edge);
    this.graphSubject.next("update");
  });

  static subscription4 = EventStore.messagesUpdated.subscribe((message: DsMessage) => {
    const source = GraphStore.networkNodes.find(node => node.id === message.source);
    if(!source) {return;}
    const srcNode = new GraphNode(message.send_logical_timestamp.toString(),message.send_logical_timestamp.toString(), source);
    this.addNode(srcNode);
    this.graphSubject.next("update");
  });


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