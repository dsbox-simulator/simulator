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
import { net } from 'electron';
import { ConfigurationStore } from '../configurationStore';


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

/**
 * GraphComponent is a component that displays the graph.
 * Adds all the important stuff to the graph from cytoscape
 */
export class GraphComponent implements AfterViewInit {
  // isProgrammaticUpdate needed to update the Y Position of the nodes
  isProgrammaticUpdate = false;
  networkNodes = GraphStore.networkNodes;
  cy: cytoscape.Core | undefined;


  

  /**
   * Reorder the network nodes
   * @param event 
   * @returns 
   */
  drop(event: CdkDragDrop<any[]>) {
    moveItemInArray(this.networkNodes, event.previousIndex, event.currentIndex);
    GraphStore.changeNetworkNodeOrder(event.previousIndex, event.currentIndex);

    if (this.cy === undefined) {
      return;
    }
  
    this.recalculateNodePositions();
  }

  /**
   * After changing the order of the Network Nodes.
   * This Method makes sure that all the Nodes adjust to the new order
   */
  recalculateNodePositions() {

    this.networkNodes = GraphStore.networkNodes;
    this.isProgrammaticUpdate = true;
  
    // Update the positions of all nodes
    this.cy!.nodes().forEach(node => {
      const posY = node.position().y;
      let offsetY = 0;

      let graphNode = GraphStore.nodes.find(n => n.id === node.data().id);

      if (graphNode === undefined) {
        if(node.data().type === 'anker') {
          const nodePosition = node.position();
          let networkNode = GraphStore.networkNodes.find(n => n.id === node.data().id || n.id + 'd' === node.data().id);
          
          if(nodePosition.y !== networkNode?.posY) {
            node.position({
              x: nodePosition.x,
              y: networkNode?.posY!
            });
          }
        }
        return;
        
      }
      const nodePosition = node.position();

      node.position({
        x: nodePosition.x,
        y: graphNode?.posY!
      });

      // Update node data to reflect new constraints
      node.data({
        minY: graphNode?.posY!,
        maxY: graphNode?.posY!
      });       

    });
  
    this.isProgrammaticUpdate = false;
  }
  
  
  /**
    * Makes sure the network nodes grow with the diagram
    * @param width 
    */
  updateNodePositions(width: number | undefined) {

    if (this.cy === undefined) {     
      return;
    }
    var containerWidth = width;
    if(containerWidth === undefined) {
       containerWidth = this.cy.extent().x2;
    }
  
    //Update Anker nodes according to the growing diagram.
    this.cy.nodes().forEach(node => {
      const data = node.data();
      if (data.type === 'anker' && data.id.endsWith('d')) {        
        node.position('x', containerWidth! + 10);
      }      
    });

  }
  
  /**
   * Adds a network node to the graph
   * @param networkNode 
   */
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
    

    //Dirty way of setting the height of the cytoscape container
    //The Cytoscape container doesnt grow dynamically with the graph so wie have to set the Height manually
    let len = this.networkNodes.length * GraphStore.heightDiff + 50;

    const cyContainer = document.getElementById('cy');
    if (cyContainer) {
        cyContainer.style.height = len+'px'; 
      }
  }

/**
  * Adds a node to the graph
  */
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

  //after adding the node we have to update the length of the scrollbar and network nodes
  
    var maxLength = Math.max(...GraphStore.networkNodes.map(node => node.length));
    const graphHeaderElement = document.getElementById('graph-header')!;
    const height = graphHeaderElement.offsetHeight; 
    const cyelement = document.getElementById('cy')!;
    cyelement.style.minHeight = `${height}px`; 


    this.updateScrollbar();

    this.updateNodePositions(maxLength);
    this.bindNodeDragRestriction(nodeCreated);
  }

/**
  * Updates the scrollbar values
  */
  updateScrollbar(dontPan: boolean = false) {
    var maxLength = Math.max(...GraphStore.networkNodes.map(node => node.length));
    const scrollbar = document.getElementById('cy-scrollbar');

    if(scrollbar !== null){
      const inputScrollbar = scrollbar as HTMLInputElement;
      let scrollbarlenght = maxLength - window.innerWidth + 200;
      inputScrollbar.max = String(scrollbarlenght);
      if(dontPan === false){ 
        if(scrollbarlenght < 0){
          scrollbarlenght = 0;
        }
        inputScrollbar.value = String(scrollbarlenght);
        this.cy!.pan({ x: scrollbarlenght * -1, y: 0 });
      }
    }
  }

/**
  * Adds an edge to the graph
  */
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

/**
  * Bind Events to the new Edge
  * Show Message when you hover the edge
  */
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

/**
* Bind Event to the new Node
* Make sure the node is not dragged out of the network node
*/
  bindNodeDragRestriction(node: any) {
    function constrainPosition(node: { data: () => any; }, pos: { x: number; y: number; }) {
      var data = node.data();
      var minX = data.minX;
      var maxX = data.maxX;
      var minY = data.minY;
      var maxY = data.maxY;
  
      if(!minX) minX = 0;
      if(!maxX) maxX = Number.MAX_SAFE_INTEGER;
      if(!minY) minY = 0;
      if(!maxY) maxY = Number.MAX_SAFE_INTEGER;
  
      return {
        x: Math.max(minX, Math.min(pos.x, maxX)),
        y: Math.max(minY, Math.min(pos.y, maxY))
      };
    }
  
    node.on('dragfree', (event: { target: any; }) => {
      var node = event.target;
      var pos = node.position();
      if (!this.isProgrammaticUpdate) {
        var constrainedPos = constrainPosition(node, pos);
        node.position(constrainedPos);
      }
    });
  
    node.on('position', (event: { target: any; }) => {
      var node = event.target;
      var pos = node.position();
      if (!this.isProgrammaticUpdate) {
        var constrainedPos = constrainPosition(node, pos);
        if (pos.x !== constrainedPos.x || pos.y !== constrainedPos.y) {
          node.position(constrainedPos);
        }
      }
    });
  }
  
  
/**
 * Creates a Style for a specific message type
 * @param type 
 * @param color 
 * @returns 
 */
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

  subscription6 = GraphStore.graphNetWorkNodeorderChanged.subscribe((networkNode) => {
    this.recalculateNodePositions();
  });
  

  subscription7 = ConfigurationStore.configurationLoaded.subscribe((loaded) => {
    GraphStore.setNetworkOrderFromConfig();
    this.recalculateNodePositions();
  });

  /**
   * Custom Zoom function
   * only zooms the width
   * @param factor 
   * @returns 
   */
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


/**
 * Initialize the Graph
 */
  initGraph() {


    const networkNodes = GraphStore.networkNodes;
    const nodes = GraphStore.nodes;
    const edges = GraphStore.edges;

    const networkNodesElements = [
      ...networkNodes.map((node) => ({ 
        data: { id: node.id, type: 'anker' }, 
        position: { x: 35, y: node.posY } 
      })),
      ...networkNodes.map((node) => ({ 
        data: { id: node.id + "d", type: 'anker' }, 
        position: { x: node.length, y: node.posY } 
      })),
      ...networkNodes.map((node) => ({ 
        data: { id: node.id + "e", source: node.id, target: node.id + "d", type: 'anker' } 
      })),
    ];

    const nodesElements = [
      ...nodes.map((node) => ({ 
        data: { id: node.id, type: 'node', minY: node.posY, maxY: node.posY }, 
        position: { x: node.posX, y: node.posY } 
      })),
    ];

    // Create a set of valid node IDs for quick lookup
    const nodeIds = new Set(nodes.map(node => node.id));

    const edgesElements = [
      ...edges
        .filter(edge => nodeIds.has(edge.source.id) && nodeIds.has(edge.target.id))
        .map(edge => ({ 
          data: { id: edge.id, source: edge.source.id, target: edge.target.id, label: edge.label } 
        })),
    ];



    var maxLength = Math.max(...networkNodes.map(node => node.length));
    maxLength += 100;
    document.getElementById('cy')!.style.minWidth = `${maxLength}px`;

    this.cy = cytoscape({

      container: document.getElementById('cy'), // container to render in  
    

      style: [ // the stylesheet for the graph
          {
            selector: 'node', //default node style
            style: {
              'width': 7,
              'height': 7,
              'background-color': '#666',
              'label': 'data(id)',
              'color': '#fff'
            }
          },
          {
            selector: 'node[type="anker"]',  //default style for the ends of the the network node
            style: {            
              'width': 0.5,
              'height': 0.5,
              'background-color': '#666',
              'label': ''
            }
          },
          {
            selector: 'node[type="marker"]', //defualt marker style
            style: {            
              'width': 20,
              'height': 30,
              'label': 'data(label)',
              'shape': 'vee',
            }
          },
          {
            selector: 'edge', //default edge style
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
            selector: 'edge[type="anker"]', //netowrk node
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
    
    this.cy.userZoomingEnabled(false);
    this.cy.userPanningEnabled(false);    

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

    this.addScrollLogic();

  }

  /**
   * custom scrolling/zoom of the graph
   */
  addScrollLogic(){
    // Assuming `cy` is your Cytoscape instance
    if (this.cy) {
      const container = this.cy.container();

      if (container) {
        container.addEventListener('wheel', (event) => {
            event.preventDefault(); // Prevent the default scroll behavior    
    
            // Determine scroll direction
            const scrollDirection = event.deltaY > 0 ? 'down' : 'up';

            if (event.ctrlKey) {
    
              // Adjust widthDiff based on scroll direction
              const adjustment = GraphStore.widthDiff * 0.05;
              const oldwidth = GraphStore.widthDiff;
      
              if (scrollDirection === 'down') {
                  GraphStore.widthDiff -= adjustment;
              } else {
                  GraphStore.widthDiff += adjustment;
              }


              var panAdjustment = GraphStore.widthDiff / oldwidth;
              var adjustmentScale = 1 / panAdjustment;
      
              // Adjust node positions based on the new widthDiff
              this.cy?.nodes().forEach(node => {
                  const pos = node.position();
                  const index = pos.x / oldwidth;    
                  node.position({ x: index * GraphStore.widthDiff, y: pos.y });
              });

              GraphStore.nodes.forEach(node => {
                  node.posX = node.posX / adjustmentScale;
              });

              GraphStore.networkNodes.forEach(node => {
                  node.length = node.length / adjustmentScale;
              });

              const panPos = this.cy?.pan().x;
              const panPosY = this.cy?.pan().y;
              this.cy?.pan({ x: panPos! * panAdjustment, y: panPosY! });

              this.updateScrollbar(true);
            }else{
              //just pan
              const pan = this.cy?.pan();
              var delta = event.deltaX;
              if(delta === 0){
                delta = event.deltaY;
              }

              this.cy?.pan({ x: pan!.x + delta, y: pan!.y });
              const scrollbar = document.getElementById('cy-scrollbar');

              if(scrollbar !== null){
                const inputScrollbar = scrollbar as HTMLInputElement;
                inputScrollbar.value = String(this.cy!.pan().x * -1);
              }
              
            }
          });
      
    }
    
    }

  }



  ngAfterViewInit() {
    this.initGraph();
  }
}