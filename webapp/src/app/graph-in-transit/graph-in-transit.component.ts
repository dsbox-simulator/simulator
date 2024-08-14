import { AfterViewInit, Component } from '@angular/core';
import cytoscape from 'cytoscape';
import { GraphStore } from '../graph/models/GraphStore';
import { EventStore } from '../models/EventStore';

@Component({
  selector: 'app-graph-in-transit',
  standalone: true,
  imports: [],
  templateUrl: './graph-in-transit.component.html',
  styleUrl: './graph-in-transit.component.scss'
})
export class GraphInTransitComponent implements AfterViewInit {
  cy: cytoscape.Core | undefined;
  nodePositions: { [key: string]: { x: number, y: number } } = {};

  ngAfterViewInit(): void {
    this.initGraph();
  }

  subscription = EventStore.eventsUpdated.subscribe((event) => {
    this.initGraph();
  });

  saveNodePositions(): void {
    this.cy?.nodes().forEach(node => {
      this.nodePositions[node.id()] = node.position();
    });
  }

  getNextPowerOf2(num: number): number {
    return Math.pow(2, Math.ceil(Math.log2(num)));
  }

  initGraph() {
    if (this.cy) {
      this.saveNodePositions();
    }

    const messages = EventStore.getNonDeliveredDsMessages();

    // Create a Set to store unique nodes
    const uniqueNodes = new Set<string>();

    // Iterate through the messages to populate the unique nodes
    messages.forEach(message => {
      uniqueNodes.add(message.source);
      uniqueNodes.add(message.target);
    });

    // Convert the unique nodes to node elements positioned in a circle
    const originalTotalNodes = GraphStore.networkNodes.length;
    //Take the next power of 2 to calculate the position of the nodes in the circle
    //Basiccly fill the gaps between the Nodes in the circle
    const totalNodes = this.getNextPowerOf2(originalTotalNodes);
    const radius = 80; 
    const centerX = 130; 
    const centerY = 130; 
    const angleStep = (2 * Math.PI) / totalNodes;

    const nodesElements = GraphStore.networkNodes.map((node, index) => {
      if (this.nodePositions[node.id]) {
        return {
          data: { id: node.id, type: 'node' },
          position: this.nodePositions[node.id]
        };
      } else {

        // Calculate the position of the node in the circle
        // Problem: index 1 for totalNodes 2 is the same as index 2 for totalNodes 4 (position in the circle)
        // Solution: Only use the odd indexes because they are the "new" one added to the circle
        // index - (totalNodes - index -1) will give the correct index for the new nodes
        var calcIndex = index - (totalNodes - index -1);
        
        const angle = calcIndex * angleStep;
        const x = centerX + radius * Math.cos(angle);
        const y = centerY + radius * Math.sin(angle);

        return {
          data: { id: node.id, type: 'node' },
          position: { x, y }
        };
      }
    });

    // Convert the messages to edge elements
    const edgesElements = messages.map(message => ({
      data: { id: `${message.source}-${message.target}`, source: message.source, target: message.target, label: message.send_logical_timestamp }
    }));

    this.cy = cytoscape({
      container: document.getElementById('InTransit_cy'), // container to render in  
      style: [ 
        {
          selector: 'node',
          style: {
            'width': 7,
            'height': 7,
            'background-color': '#666',
            'label': 'data(id)',
            'color': '#fff'
          }
        },
        {
          selector: 'node[type="anker"]', 
          style: {            
            'width': 0.5,
            'height': 0.5,
            'background-color': '#666',
            'label': ''
          }
        },
        {
          selector: 'edge',
          style: {
            'width': 2,
            'line-color': '#b58900',
            'target-arrow-color': '#b58900',
            'target-arrow-shape': 'triangle',
            'curve-style': 'bezier',
            'color': '#FFF',
            'text-opacity': 1,
            'text-background-color': '#333',
            'text-background-opacity': 1,
            'text-background-shape': 'roundrectangle',
            'text-background-padding': '3px',
            label: 'data(label)'
          }
        },
        {
          selector: 'edge[type="anker"]',
          style: {
            'width': 2,
            'line-color': '#fff',
            'target-arrow-color': '#ccc',
            'target-arrow-shape': 'triangle',
            'curve-style': 'haystack'
          }
        }
      ],
      layout: {
        name: 'preset'
      }
    });

    this.cy.maxZoom(2);
    this.cy.minZoom(0.5);
    this.cy.zoom(1.5);    
    this.cy.userZoomingEnabled(false);
    this.cy.userPanningEnabled(true);

    this.cy.add(nodesElements);
    this.cy.add(edgesElements);
  }
}
