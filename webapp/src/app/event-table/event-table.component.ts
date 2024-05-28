import { Component, OnInit, OnDestroy } from '@angular/core';
import { EventStore } from '../models/EventStore';
import Event from '../models/communication/Event';
import { Subscription } from 'rxjs';
import { CommonModule } from '@angular/common';
import Timestamp from '../models/communication/Timestamp';

@Component({
  selector: 'app-event-table',
  standalone: true,
  imports: [
    CommonModule
  ],
  templateUrl: './event-table.component.html',
  styleUrls: ['./event-table.component.scss']
})
export class EventTableComponent implements OnInit, OnDestroy {
  public events: Event[] = EventStore.events;
  private eventsSub!: Subscription;

  ngOnInit() {
  }

  ngOnDestroy() {
    this.eventsSub.unsubscribe();
  }
}