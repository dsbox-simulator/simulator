import { Component, OnInit, OnDestroy } from '@angular/core';
import { TypeColorStore } from '../models/TypeColorStore';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { Subscription } from 'rxjs';
import { LinkedPredicate } from '../json-predicate/Models/LinkedPredicate';
import { PredicateStore } from '../json-predicate/Models/PredicateStore';

@Component({
  selector: 'app-graph-legend',
  standalone: true,
  imports: [
    CommonModule,
    FormsModule
  ],
  templateUrl: './graph-legend.component.html',
  styleUrls: ['./graph-legend.component.scss']
})
export class GraphLegendComponent implements OnInit, OnDestroy {
  colorMap: { [key: string]: string } = TypeColorStore.colorMap;
  predicates: LinkedPredicate[] = [];
  private subscriptions: Subscription = new Subscription();

  ngOnInit(): void {
    this.updatePredicates();

    this.subscriptions.add(
      PredicateStore.eventsChanged.subscribe(() => {
        this.updatePredicates();
      })
    );

    this.subscriptions.add(
      TypeColorStore.addedNewColor.subscribe(() => {
        this.colorMap = { ...TypeColorStore.colorMap };
        console.log('Updated ColorMap:', this.colorMap);
      })
    );
  }

  ngOnDestroy(): void {
    this.subscriptions.unsubscribe();
  }

  private updatePredicates(): void {
    this.predicates = PredicateStore.getEvents();
  }

  getKeys(obj: { [key: string]: string }): string[] {
    return Object.keys(obj);
  }

  getPredicateStyle(predicate: LinkedPredicate, nodeIndex: number): { [key: string]: string } {
    const isActive = nodeIndex < predicate.currentState;
    return {
      backgroundColor: isActive ? 'green' : 'red',
      padding: '5px',
      borderRadius: '3px',
      margin: '2px'
    };
  }

  getPredicateExpression(predicate: LinkedPredicate, nodeIndex: number): string {
    return predicate.predicateNode[nodeIndex].toString();
  }
}
