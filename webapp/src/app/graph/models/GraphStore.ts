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
import { ConfigurationStore } from '../../configurationStore';

/**
 * Stores all graph data
 */
export class GraphStore {
    public static  edges: GraphEdge[] = [];
    public static nodes: GraphNode[] = [];
    public static networkNodes: NetworkNode[] = [];

    public static nodeCount: number = 1;

    public static readonly heightDiff = 70;
    public static widthDiff: number = 50.0;



  static graphSubject: Subject<string> = new Subject<string>();

  static graphNetWorkNode: Subject<NetworkNode> = new Subject<NetworkNode>();
  static graphNetWorkNodeorderChanged: Subject<NetworkNode> = new Subject<NetworkNode>();
  static graphNode: Subject<GraphNode> = new Subject<GraphNode>();
  static graphEdge: Subject<GraphEdge> = new Subject<GraphEdge>();


  static subscription = EventStore.eventsUpdated.subscribe((event: JsonRpcEvent) => {
        //GraphStore.handleNewEvent(event);
  });

  /**
   * Subscription to the nodeSetupsUpdated event
   */
  static subscription2 = EventStore.nodeSetupsUpdated.subscribe((nodeSetup: DsNodeSetup) => {    
      const networkNode = new NetworkNode(nodeSetup.id, nodeSetup.id);
      this.addNetworkNode(networkNode);
      this.graphSubject.next("update");
  });

  /**
   * Subscription to the deliveredMessage event
   * Add the delivered message as a node and edge to the graph
   */
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

  /**
   * Subscription to the messagesUpdated event
   * Add the message as a node to the graph
   */
  static subscription4 = EventStore.messagesUpdated.subscribe((message: DsMessage) => {
    const source = GraphStore.networkNodes.find(node => node.id === message.source);
    if(!source) {return;}
    const srcNode = new GraphNode(message.send_logical_timestamp.toString(),message.send_logical_timestamp.toString(), source);
    
    this.addNode(srcNode);
    this.graphSubject.next("update");
  });

  /**
   * Subscription to the logMessagesUpdated event
   * Add the log message as a node to the graph
   * The color of the node is set to the color of the log message
   */
  static subscription5 = EventStore.logMessagesUpdated.subscribe((logMessage: DsLogMessage) => {
    const source = GraphStore.networkNodes.find(node => node.id === logMessage.source);
    if(!source) {return;}
    const srcNode = new GraphNode(logMessage.send_logical_timestamp.toString(),logMessage.logmessage.marker.label, source);
    srcNode.color = logMessage.logmessage.marker.color;
    this.addNode(srcNode);
    this.graphSubject.next("update");
  });


  /**
   * 
   * @param node the node to add
   * Add a network node (vertical line) to the graph
   */
  static addNetworkNode(node: NetworkNode) {    

    node.posY = (GraphStore.networkNodes.length + 1) * this.heightDiff;
    GraphStore.networkNodes.push(node);  

    var positions = ConfigurationStore.networkNodePositions; // { [key: string]: number } = {};
    
    // Create list/array GraphStore.networkNodes with corresponding positions
    let nodeListWithPositions = GraphStore.networkNodes.map((node, index) => {
        return {
            node: node,
            position: positions[node.label] !== undefined ? positions[node.label] : index
        };
    });

    // Handle duplicates: if positions are duplicate, increment until unique
    nodeListWithPositions.forEach((item, index) => {
        let pos = item.position;
        while (nodeListWithPositions.some((other, idx) => idx !== index && other.position === pos)) {
            pos += 1;
        }
        item.position = pos;
    });

    // Sort the list by position and reassign positions based on sorted order
    nodeListWithPositions.sort((a, b) => a.position - b.position);

    // Update GraphStore.networkNodes with sorted nodes
    GraphStore.networkNodes = nodeListWithPositions.map(item => item.node);

    this.changeNetWorkNodeOrderIntern();
    this.graphNetWorkNode.next(node);
    this.graphNetWorkNodeorderChanged.next(node);
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
    

    ConfigurationStore.networkNodePositions = {};
    GraphStore.networkNodes.forEach(node => {
      node.posY = (GraphStore.networkNodes.indexOf(node) + 1) * this.heightDiff;
      //store the new position of the network nodes
      ConfigurationStore.networkNodePositions[node.label] = GraphStore.networkNodes.indexOf(node);
    });

    ConfigurationStore.saveConfiguration();

    this.graphSubject.next("update");
  }

  static changeNetWorkNodeOrderIntern(){

    GraphStore.networkNodes.forEach(node => {
      node.posY = (GraphStore.networkNodes.indexOf(node) + 1) * this.heightDiff;
    });
    this.graphSubject.next("update");
  }

  public static setNetworkOrderFromConfig(){
    let temp = GraphStore.networkNodes;
    GraphStore.networkNodes = [];
    temp.forEach(node => {
      this.addNetworkNode(node);
    });
  }
}