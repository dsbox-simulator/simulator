import { AfterViewInit, Component } from '@angular/core';
import cytoscape from 'cytoscape';
import { GraphStore } from './models/GraphStore';

@Component({
  selector: 'app-graph',
  standalone: true,
  imports: [],
  templateUrl: './graph.component.html',
  styleUrl: './graph.component.scss'
})

export class GraphComponent implements AfterViewInit {

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
        ...edges.map((edge) => ({ data: { id: edge.label, source: edge.source.id, target: edge.target.id } })),
    ];



    console.log("nodes: ", nodesElements);

    console.log("edges: ", edgesElements);

    var cy = cytoscape({

      container: document.getElementById('cy'), // container to render in  
    

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
              'curve-style': 'bezier'
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
        { data: { id: 'ab', source: 'a', target: 'b' }},
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
            'curve-style': 'bezier'
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
