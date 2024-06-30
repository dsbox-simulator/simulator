import { Component, Input, OnInit } from '@angular/core';
import { TypeColorStore } from '../models/TypeColorStore';
import { CommonModule, NgFor } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { Subscription } from 'rxjs';

@Component({
  selector: 'app-graph-legend',
  standalone: true,
  imports: [
    CommonModule,
    FormsModule
  ],
  templateUrl: './graph-legend.component.html',
  styleUrl: './graph-legend.component.scss'
})
export class GraphLegendComponent implements OnInit {
  colorMap: { [key: string]: string } = TypeColorStore.colorMap;
  private subscription: Subscription = new Subscription();

  ngOnInit(): void {
    // Subscribe to the addedNewColor observable
    this.subscription = TypeColorStore.addedNewColor.subscribe(() => {
      this.colorMap = { ...TypeColorStore.colorMap };
      console.log('Updated ColorMap:', this.colorMap);
    });
  }

  ngOnDestroy(): void {
    // Unsubscribe to avoid memory leaks
    this.subscription.unsubscribe();
  }

  getKeys(obj: { [key: string]: string }): string[] {
    return Object.keys(obj);
  }
  
}
