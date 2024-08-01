import { AfterViewInit, Component } from '@angular/core';
import cytoscape from 'cytoscape';
import { GraphStore } from './models/GraphStore';
import { CommonModule } from '@angular/common';
import { BrowserAnimationsModule } from '@angular/platform-browser/animations';
import { CdkDragDrop, DragDropModule, moveItemInArray } from '@angular/cdk/drag-drop';
import { NetworkNode } from './models/NetworkNode';
import { GraphNode } from './models/GraphNode';
import { GraphEdge } from './models/GraphEdge';
import { GraphLegendComponent } from "../graph-legend/graph-legend.component";
import { TypeColorStore } from '../models/TypeColorStore';


@Component({
    selector: 'app-graph',
    standalone: true,
    templateUrl: './graph.component.html',
    styleUrl: './graph.component.scss',
    imports: [
        CommonModule,
        DragDropModule,
        GraphLegendComponent
    ]
})

export class GraphComponent implements AfterViewInit {

  networkNodes = GraphStore.networkNodes;
  cy: cytoscape.Core | undefined;

  drop(event: CdkDragDrop<any[]>) {
    moveItemInArray(this.networkNodes, event.previousIndex, event.currentIndex);
    GraphStore.changeNetworkNodeOrder(event.previousIndex, event.currentIndex);
    
  }

  updateNodePositions(width: number | undefined) {

    if (this.cy === undefined) {     
      return;
    }
    var containerWidth = width;
    if(containerWidth === undefined) {
       containerWidth = this.cy.extent().x2;
    }
  
    this.cy.nodes().forEach(node => {
      const data = node.data();
      if (data.type === 'anker' && data.id.endsWith('d')) {
        node.position('x', containerWidth! - 35);
      }
    });

    //const scrollContainer = document.querySelector('.cytoscape-scroll-container');
    //scrollContainer!.scrollLeft = containerWidth! - 35;
  }
  
  addNetworkNodeToGraph(networkNode: NetworkNode) {
    const newNodeElement = {
      data: { id: networkNode.id, type: 'anker' },
      position: { x: 35, y: networkNode.posY }
    };
  
    if (this.cy === undefined) {     
      return;
    }

    const containerWidth = this.cy.extent().x2;
  
    const newNodeEndElement = {
      data: { id: networkNode.id + 'd', type: 'anker' },
      position: { x: containerWidth - 35, y: networkNode.posY }
    };
  
    const newEdgeElement = {
      data: { id: networkNode.id + 'e', source: networkNode.id, target: networkNode.id + 'd', type: 'anker' }
    };
  
    this.cy.add([newNodeElement, newNodeEndElement, newEdgeElement]);
  
    // Call updateNodePositions to ensure all nodes are correctly positioned
    this.updateNodePositions(undefined);
  }

  addNodeToGraph(node: GraphNode) {

    if (this.cy === undefined) {
      return;
    }

    var nodeCreated;
    if(node.color === undefined) {
      const newNodeElement = {
        data: { id: node.id, type: 'node', minY: node.posY, maxY: node.posY },
        position: { x: node.posX, y: node.posY }
      };

      nodeCreated = this.cy.add(newNodeElement);
    } else {
      
      const newNodeElement = {
        data: { id: node.id, type: 'marker',label: node.label, minY: node.posY, maxY: node.posY },
        position: { x: node.posX, y: node.posY },
        style: { 'background-color': node.color }
      };

      nodeCreated = this.cy.add(newNodeElement);
    }

  
    var maxLength = Math.max(...GraphStore.networkNodes.map(node => node.length));
    maxLength += 100;
    this.cy.extent().x2 = maxLength;
    const graphHeaderElement = document.getElementById('graph-header')!;
    const height = graphHeaderElement.offsetHeight; 
    const cyelement = document.getElementById('cy')!;
    //cyelement.style.minWidth = `${maxLength}px`;
    cyelement.style.minHeight = `${height}px`; 

    const scrollbar = document.getElementById('cy-scrollbar');

    if(scrollbar !== null){
      const inputScrollbar = scrollbar as HTMLInputElement;
      inputScrollbar.max = String(maxLength);
    }

    this.updateNodePositions(maxLength);
    this.bindNodeDragRestriction(nodeCreated);
  }

  addEdgeToGraph(edge: GraphEdge) {

    var newEdgeElement;
    if(edge.color === undefined) {
      newEdgeElement = {
        data: { id: edge.id, source: edge.source.id, target: edge.target.id, label: edge.label }
      };
    }else{
      newEdgeElement = {
        data: { id: edge.id, source: edge.source.id, target: edge.target.id, label: edge.label, type : edge.type }
      };
    }
    

    if (this.cy === undefined) {
      return;
    }
    const newEdge = this.cy.add(newEdgeElement);
    this.bindEdgeEvents(newEdge);
  }


  bindEdgeEvents(edge: any) {
    edge.on('mouseover', (event: { target: any; }) => {
      var edge = event.target;
      edge.style('text-opacity', 1);
      edge.style('z-compound-depth', 'top');
    });
  
    edge.on('mouseout', (event: { target: any; }) => {
      var edge = event.target;
      edge.style('text-opacity', 0);
      edge.style('z-compound-depth', 'bottom');
    });
  }

  bindNodeDragRestriction(node: any){

    function constrainPosition(node: { data: () => any; }, pos: { x: number; y: number; }) {
      var data = node.data();

      var minX = data.minX;
      var maxX = data.maxX;
      var minY = data.minY;
      var maxY = data.maxY;

      if(!minX)
        minX = 0;
      if(!maxX)
        maxX = Number.MAX_SAFE_INTEGER;
      if(!minY)
        minY = 0;
      if(!maxY)
        maxY = Number.MAX_SAFE_INTEGER;

      console.log("minX: ", minX);
      console.log("maxX: ", maxX);
      console.log("minY: ", minY);
      console.log("maxY: ", maxY);

      return {
        x: Math.max(minX, Math.min(pos.x, maxX)),
        y: Math.max(minY, Math.min(pos.y, maxY))
      };
    }

    node.on('dragfree', function(event: { target: any; }) {
      var node = event.target;
      var pos = node.position();
      var constrainedPos = constrainPosition(node, pos);
      node.position(constrainedPos);
    });

    node.on('position', function(event: { target: any; }) {
      var node = event.target;
      var pos = node.position();
      var constrainedPos = constrainPosition(node, pos);
      if (pos.x !== constrainedPos.x || pos.y !== constrainedPos.y) {
        node.position(constrainedPos);
      }
    });
  }

  appendStyle(type: string, color: string){

    if(this.cy === undefined) {
      return;
    }

    this.cy.style().selector('edge[type="'+ type +'"]')
         .style({
            'line-color': color,
            'target-arrow-color': color,
         }).update();
  }

  

  subscription1 = GraphStore.graphNode.subscribe((node) => {
    this.addNodeToGraph(node);
  });

  subscription2 = GraphStore.graphEdge.subscribe((edge) => {
    this.addEdgeToGraph(edge);
  });

  subscription3 = GraphStore.graphNetWorkNode.subscribe((networkNode) => {
    this.addNetworkNodeToGraph(networkNode);
  });

  subscription4 = GraphStore.graphSubject.subscribe((graph) => {
    
  });

  subscription5 = TypeColorStore.addedNewColor.subscribe((map) => {
    this.appendStyle(map.key, map.color);
  });

  zoomWidth(factor: number) {
    if (this.cy === undefined) {
        return;
    }

    // Get the current extent and center
    const extent = this.cy.extent();
    const centerX = (extent.x1 + extent.x2) / 2;

    // Calculate the new width and adjust node positions
    this.cy.nodes().forEach(node => {
        const pos = node.position();
        const offsetX = pos.x - centerX;
        const newPosX = centerX + offsetX * factor;

        node.position({
            x: newPosX,
            y: pos.y  // Keep y position unchanged
        });
    });

    // Adjust pan to simulate zoom
    const pan = this.cy.pan();
    this.cy.pan({ x: pan.x * factor, y: pan.y });

    // Update the viewport to reflect changes
    this.cy.fit(this.cy.elements(), 50); // Optional padding
}



  initGraph() {

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
      ...nodes.map((node) => ({ data: { id: node.id, type: 'node', minY: node.posY, maxY: node.posY}, position: { x: node.posX, y: node.posY } } )),
    ];

    const edgesElements = [
        ...edges.map((edge) => ({ data: { id: edge.id, source: edge.source.id, target: edge.target.id, label: edge.label } })),
    ];



    console.log("nodes: ", nodesElements);

    console.log("edges: ", edgesElements);

    var maxLength = Math.max(...networkNodes.map(node => node.length));
    maxLength += 100;
    document.getElementById('cy')!.style.minWidth = `${maxLength}px`;

    this.cy = cytoscape({

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
            selector: 'node[type="anker"]', 
            style: {            
              'width': 0.5,
              'height': 0.5,
              'background-color': '#666',
              'label': ''
            }
          },
          {
            selector: 'node[type="marker"]',
            style: {            
              'width': 10,
              'height': 10,
              'label': 'data(label)'
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
              'text-opacity': 0,
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
        },        
    });
    
    function constrainPosition(node: { data: () => any; }, pos: { x: number; y: number; }) {
      var data = node.data();

      var minX = data.minX;
      var maxX = data.maxX;
      var minY = data.minY;
      var maxY = data.maxY;

      if(!minX)
        minX = 0;
      if(!maxX)
        maxX = Number.MAX_SAFE_INTEGER;
      if(!minY)
        minY = 0;
      if(!maxY)
        maxY = Number.MAX_SAFE_INTEGER;

      console.log("minX: ", minX);
      console.log("maxX: ", maxX);
      console.log("minY: ", minY);
      console.log("maxY: ", maxY);

      return {
        x: Math.max(minX, Math.min(pos.x, maxX)),
        y: Math.max(minY, Math.min(pos.y, maxY))
      };
    }

    this.cy.add(networkNodesElements);
    this.cy.add(nodesElements);
    this.cy.add(edgesElements);
    this.cy.edges().filter(edge => edge.data('type') !== 'anker').forEach(edge => {
      edge.on('mouseover', function(event) {
        var edge = event.target;        
        edge.style('text-opacity', 1);
        edge.style('z-compound-depth', 'top');
      });
      
      edge.on('mouseout', function(event) {
        var edge = event.target;
        edge.style('text-opacity', 0);
        edge.style('z-compound-depth', 'bottom');
      });
    });
    
    //this.cy.maxZoom(2);
    //this.cy.minZoom(0.5);
    this.cy.userZoomingEnabled(false);
    this.cy.userPanningEnabled(true);
    
    //doesnt work
    /*this.cy.container()!.addEventListener('wheel', (event) => {
      event.preventDefault();

      let factor = event.deltaY < 0 ? 1.1 : 0.9;

      this.zoomWidth(factor);
    });*/

    const scrollbar = document.getElementById('cy-scrollbar');

    // Initialize scrollbar based on current pan position
    const pan = this.cy.pan();
    if(scrollbar !== null){

      const inputScrollbar = scrollbar as HTMLInputElement;
      inputScrollbar.min = "0";  // Set appropriate min value
      inputScrollbar.max = "1000";   // Set appropriate max value
      inputScrollbar.value = String(pan.x);

      // Update Cytoscape pan when the scrollbar is moved
      scrollbar.addEventListener('input', (event) => {

        const target = event.target as HTMLInputElement;
        console.log("scrollbar value: ", target.value);
        this.cy!.pan({ x: target.valueAsNumber * -1, y: 0 });
      });
    }

    this.cy.nodes().on('dragfree', function(event) {
      var node = event.target;
      var pos = node.position();
      var constrainedPos = constrainPosition(node, pos);
      node.position(constrainedPos);
    });

    this.cy.nodes().on('position', function(event) {
      var node = event.target;
      var pos = node.position();
      var constrainedPos = constrainPosition(node, pos);
      if (pos.x !== constrainedPos.x || pos.y !== constrainedPos.y) {
        node.position(constrainedPos);
      }
    });

  }



  ngAfterViewInit() {
    this.initGraph();
  }
}