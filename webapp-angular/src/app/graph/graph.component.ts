import { AfterViewInit, Component } from '@angular/core';
import cytoscape from 'cytoscape';

@Component({
  selector: 'app-graph',
  standalone: true,
  imports: [],
  templateUrl: './graph.component.html',
  styleUrl: './graph.component.css'
})

export class GraphComponent implements AfterViewInit {

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
