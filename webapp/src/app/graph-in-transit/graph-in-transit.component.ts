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
  ngAfterViewInit(): void {
    this.initGraph();
  }
  cy: cytoscape.Core | undefined;


  subscription = EventStore.eventsUpdated.subscribe((event) => {
    this.initGraph();
  });

  initGraph(){

    const messages = EventStore.getNonDeliveredDsMessages();

    // Create a Set to store unique nodes
    const uniqueNodes = new Set<string>();

    // Iterate through the messages to populate the unique nodes
    messages.forEach(message => {
      uniqueNodes.add(message.source);
      uniqueNodes.add(message.target);
    });

   // Convert the unique nodes to node elements positioned in a circle
    const totalNodes = uniqueNodes.size;
    const radius = 100; 
    const centerX = 150; 
    const centerY = 150; 
    const angleStep = (2 * Math.PI) / totalNodes;

    const nodesElements = Array.from(uniqueNodes).map((nodeId, index) => {
      const angle = (index * angleStep) + (Math.PI); 
      const x = centerX + radius * Math.cos(angle);
      const y = centerY + radius * Math.sin(angle);

      return {
      data: { id: nodeId, type: 'node', minY: y, maxY: y },
      position: { x, y }
      };
    });

    // Convert the messages to edge elements
    const edgesElements = messages.map(message => ({
      data: { id: `${message.source}-${message.target}`, source: message.source, target: message.target, label: message.send_logical_timestamp } // Adjust label based on your message structure
    }));


    this.cy = cytoscape({

      container: document.getElementById('InTransit_cy'), // container to render in  
    

      style: [ // the stylesheet for the graph
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
            selector: 'node[type="anker"]', // Select nodes with type="square"
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
