import { Subject } from 'rxjs';
import { GraphEdge } from './GraphEdge';
import { GraphNode } from './GraphNode';
import { NetworkNode } from './NetworkNode';
import { EventStore } from '../../models/EventStore';
import { DsNodeSetup } from '../../models/DsNodeSetup';
import { DsMessage } from '../../models/DsMessage';
import { Version } from '@angular/core';
import { JsonRpcEvent } from '../../models/communication/RpcEvent';
import { DsLogMessage } from '../../models/DsLogMessage';

export class GraphStore {
    public static  edges: GraphEdge[] = [];
    public static nodes: GraphNode[] = [];
    public static networkNodes: NetworkNode[] = [];

    public static nodeCount: number = 1;

    private static readonly heightDiff = 70;
    public static widthDiff: number = 50.0;



  static graphSubject: Subject<string> = new Subject<string>();

  static graphNetWorkNode: Subject<NetworkNode> = new Subject<NetworkNode>();
  static graphNode: Subject<GraphNode> = new Subject<GraphNode>();
  static graphEdge: Subject<GraphEdge> = new Subject<GraphEdge>();


  static subscription = EventStore.eventsUpdated.subscribe((event: JsonRpcEvent) => {
        //GraphStore.handleNewEvent(event);
  });

  static subscription2 = EventStore.nodeSetupsUpdated.subscribe((nodeSetup: DsNodeSetup) => {    
      const networkNode = new NetworkNode(nodeSetup.id, nodeSetup.id);
      this.addNetworkNode(networkNode);
      this.graphSubject.next("update");
  });

  static subscription3 = EventStore.deliveredMessage.subscribe((message: DsMessage) => {

    var srcNode = GraphStore.nodes.find(node => node.id === message.send_logical_timestamp.toString())
    const target = GraphStore.networkNodes.find(node => node.id === message.target);
    if(!target) {return;}
    const destNode = new GraphNode(message.deliver_logical_timestamp!.toString(),message.deliver_logical_timestamp!.toString(), target);
    this.addNode(destNode, srcNode!.posX);

    const edge = new GraphEdge(srcNode!, destNode,message.send_logical_timestamp.toString() + "edge", message.send_logical_timestamp!, message.body);
    edge.color = message.typeColor;
    edge.type = message.type;
    
    this.addEdge(edge);
  });

  static subscription4 = EventStore.messagesUpdated.subscribe((message: DsMessage) => {
    const source = GraphStore.networkNodes.find(node => node.id === message.source);
    if(!source) {return;}
    const srcNode = new GraphNode(message.send_logical_timestamp.toString(),message.send_logical_timestamp.toString(), source);
    
    this.addNode(srcNode);
    this.graphSubject.next("update");
  });

  static subscription5 = EventStore.logMessagesUpdated.subscribe((logMessage: DsLogMessage) => {
    const source = GraphStore.networkNodes.find(node => node.id === logMessage.source);
    if(!source) {return;}
    const srcNode = new GraphNode(logMessage.send_logical_timestamp.toString(),logMessage.logmessage.marker.label, source);
    srcNode.color = logMessage.logmessage.marker.color;
    this.addNode(srcNode);
    this.graphSubject.next("update");
  });


  static addNetworkNode(node: NetworkNode) {
    node.posY = (GraphStore.networkNodes.length + 1) * this.heightDiff;
    GraphStore.networkNodes.push(node);  
    this.graphNetWorkNode.next(node);
  }

  static addEdge(edge: GraphEdge) {
    if(edge.source.posX + 100 < edge.target.posX) {

      const sameNetworkNodes = GraphStore.nodes.filter(n => n.networkNode === edge.source.networkNode 
        && n.posX > edge.source.posX);
      
        var newPosX = edge.target.posX - 100;
        var offsetX = newPosX - edge.source.posX;

        sameNetworkNodes.forEach(node => {
          node.posX += offsetX;
        });

        edge.source.posX = newPosX;
    }
    GraphStore.edges.push(edge);
    this.graphSubject.next("update");
    this.graphEdge.next(edge);
  }

  static addNode(node: GraphNode, posX: number = 0) {
    const networkNode = GraphStore.networkNodes.find(n => n === node.networkNode);
    if (networkNode) {
      const sameNetworkNodes = GraphStore.nodes.filter(n => n.networkNode === networkNode);
      const biggestPosX = Math.max(...sameNetworkNodes.map(node => node.posX));
      /*node.posX = biggestPosX + this.widthDiff;
      if(node.posX < posX) {
        node.posX = posX + this.widthDiff;
      }*/
      node.posX = this.nodeCount * this.widthDiff;
      this.nodeCount++;
      if( node.posX > networkNode.length) {
        networkNode.length = node.posX + this.widthDiff;
      }
    }
    GraphStore.nodes.push(node);
    this.graphNode.next(node);
  }


  static changeNetworkNodeOrder(oldIndex: number, newIndex: number) {
    
    GraphStore.networkNodes.forEach(node => {
      node.posY = (GraphStore.networkNodes.indexOf(node) + 1) * this.heightDiff;
      //console.log("node: " + node.id + " posY: " + node.posY, node)
    });

    this.graphSubject.next("update");
  }
}