import { Component, OnInit, OnDestroy, Inject, Input, OnChanges, SimpleChanges } from '@angular/core';
import { EventStore } from '../models/EventStore';
import Event from '../models/communication/Event';
import { Subscription } from 'rxjs';
import { CommonModule } from '@angular/common';
import Timestamp from '../models/communication/Timestamp';
import { JsonRpcEvent } from '../models/communication/RpcEvent';

@Component({
  selector: 'app-event-table',
  standalone: true,
  imports: [
    CommonModule
  ],
  templateUrl: './event-table.component.html',
  styleUrls: ['./event-table.component.scss']
})
export class EventTableComponent implements OnInit, OnDestroy, OnChanges {
  @Input() delivered: boolean = false;
  public events: JsonRpcEvent[] = EventStore.events;
  private eventsSub!: Subscription;

  public searchText = '';
  public sortKey = '';

  sort(key: string) {
    this.sortKey = key;
  }

  constructor() { }

  ngOnChanges(changes: SimpleChanges) {
    if (changes['delivered']) {
      this.updateEvents();
    }
  }

  ngOnInit() {
    this.eventsSub = EventStore.eventsUpdated.subscribe((event: JsonRpcEvent) => {
      this.updateEvents();
    });
  }

  ngOnDestroy() {
    this.eventsSub.unsubscribe();
  }

  private updateEvents() {
    if(this.delivered){
      this.events = EventStore.getNonDeliveredMessages();
    }else{
      this.events = EventStore.events;
    }
  }
}