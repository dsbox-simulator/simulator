import { AfterViewInit, Component } from '@angular/core';
import cytoscape from 'cytoscape';
import { GraphStore } from './models/GraphStore';
import { CommonModule } from '@angular/common';
import { BrowserAnimationsModule } from '@angular/platform-browser/animations';
import { CdkDragDrop, DragDropModule, moveItemInArray } from '@angular/cdk/drag-drop';

@Component({
  selector: 'app-graph',
  standalone: true,
  imports: [
    CommonModule,    
    DragDropModule
  ],
  templateUrl: './graph.component.html',
  styleUrl: './graph.component.scss'
})

export class GraphComponent implements AfterViewInit {

  networkNodes = GraphStore.networkNodes;

  drop(event: CdkDragDrop<any[]>) {
    moveItemInArray(this.networkNodes, event.previousIndex, event.currentIndex);
    GraphStore.changeNetworkNodeOrder(event.previousIndex, event.currentIndex);
    
  }

  subscription2 = GraphStore.graphSubject.subscribe((graph) => {
    this.updateGraph();
  });

  updateGraph() {

  console.log("updateGraph");


    const networkNodes = GraphStore.networkNodes;
    const nodes = GraphStore.nodes;
    const edges = GraphStore.edges;

    const networkNodesElements = [
      ...networkNodes.map((node) => ({ data: { id: node.id, type: 'anker'}, position: { x: 35, y: node.posY } } )),
      ...networkNodes.map((node) => ({ data: { id: node.id + "d", type: 'anker'}, position: { x: node.length, y: node.posY }  })),
      ...networkNodes.map((node) => ({ data: { id: node.id + "e", source: node.id, target: node.id + "d", type: 'anker' } })),
    ];


    const nodesElements = [
      ...nodes.map((node) => ({ data: { id: node.id, type: 'node'}, position: { x: node.posX, y: node.posY } } )),
    ];

    const edgesElements = [
        ...edges.map((edge) => ({ data: { id: edge.id, source: edge.source.id, target: edge.target.id, label: edge.label } })),
    ];



    console.log("nodes: ", nodesElements);

    console.log("edges: ", edgesElements);

    var maxLength = Math.max(...networkNodes.map(node => node.length));
    maxLength += 100;
    document.getElementById('cy')!.style.minWidth = `${maxLength}px`;

    var cy = cytoscape({

      container: document.getElementById('cy'), // container to render in  
    

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
              'color': '#b58900',
              //label: 'data(label)'
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

    cy.add(networkNodesElements);
    cy.add(nodesElements);
    cy.add(edgesElements);
    
  }


  ngAfterViewInit() {
    
    var cy = cytoscape({

      container: document.getElementById('cy'), // container to render in
    
      elements: [ // list of graph elements to start with
        { data: { id: 'a' }, position: {x: 35, y: 30}},
        { data: { id: 'b' }, position: {x: 100, y: 60}},
        { data: { id: 'c', type: 'anker' }, position: {x: 30, y: 30 }},
        { data: { id: 'd', type: 'anker' }, position: {x: 130, y: 30 }},
        { data: { id: 'e', type: 'anker' }, position: {x: 30, y: 60 }},
        { data: { id: 'f', type: 'anker' }, position: {x: 130, y: 60 }},
        { data: { id: 'ab', source: 'a', target: 'b', label: 'ab' }},
        { data: { id: 'cd', source: 'c', target: 'd', type: 'anker' }},
        { data: { id: 'ef', source: 'e', target: 'f', type: 'anker' }}
      ],
    
      style: [ // the stylesheet for the graph
        {
          selector: 'node',
          style: {
            'width': 7,
            'height': 7,
            'background-color': '#666',
            'label': 'data(id)'
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
            'line-color': '#ccc',
            'target-arrow-color': '#ccc',
            'target-arrow-shape': 'triangle',
            'curve-style': 'bezier',
            label: 'data(label)'
          }
        },
        {
          selector: 'edge[type="anker"]',
          style: {
            'width': 2,
            'line-color': '#000',
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
  }
}
