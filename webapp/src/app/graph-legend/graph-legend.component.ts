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

/**
 * GraphLegendComponent is a component that displays Breakpoints and the Color for Message Types for the graph.
 */
export class GraphLegendComponent implements OnInit, OnDestroy {
  colorMap: { [key: string]: string } = TypeColorStore.colorMap;
  predicates: { predicate: LinkedPredicate, expressions: { expression: string, result: boolean | null }[] }[] = [];
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
      })
    );
  }

  ngOnDestroy(): void {
    this.subscriptions.unsubscribe();
  }

  private updatePredicates(): void {
    this.predicates = PredicateStore.getEvents().map(predicate => ({
      predicate,
      expressions: predicate.syntaxTree.collectExpressionsWithResults()
    }));

    console.log('Updating predicates', this.predicates);
  }

  getPredicateStyle(result: boolean | null): { [key: string]: string } {
    return {
      backgroundColor: result === null ? 'gray' : (result ? 'green' : 'red'),
      padding: '5px',
      borderRadius: '3px',
      margin: '2px'
    };
  }

  getKeys(obj: { [key: string]: string }): string[] {
    return Object.keys(obj);
  }

  trackByPredicate(index: number, item: { predicate: LinkedPredicate }): number {
    return index;
  }

  trackByIndex(index: number): number {
    return index;
  }
}
