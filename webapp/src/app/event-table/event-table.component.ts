import { Component, OnInit, OnDestroy } from '@angular/core';
import { EventStore } from '../models/EventStore';
import Event from '../models/Event';
import { Subscription } from 'rxjs';
import { CommonModule } from '@angular/common';
import Timestamp from '../models/Timestamp';

@Component({
  selector: 'app-event-table',
  standalone: true,
  imports: [
    CommonModule
  ],
  templateUrl: './event-table.component.html',
  styleUrls: ['./event-table.component.css']
})
export class EventTableComponent implements OnInit, OnDestroy {
  public events: Event[] = [];
  private eventsSub!: Subscription;

  ngOnInit() {
    this.events = EventStore.events;
    this.eventsSub = EventStore.eventsUpdated.subscribe((events: Event) => {
      this.events = EventStore.events;
    });  
    this.events[0] = new Event();
    this.events[0].timestamp = new Timestamp();  
  }

  ngOnDestroy() {
    this.eventsSub.unsubscribe();
  }
}